#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_runtime::traits::Saturating;

pub use pallet::*;

/// Minimum stake required to participate as validator or prover (1000 ZEN in base units)
pub const MINIMUM_VALIDATOR_STAKE: u128 = 1_000 * 10_u128.pow(12);
pub const MINIMUM_PROVER_STAKE: u128 = 500 * 10_u128.pow(12);
/// Slash percentage: 10% of stake removed per slash
pub const SLASH_PERCENT: u128 = 10;
/// Annual reward rate: 5% APY, distributed per-block
/// Assuming 5,256,000 blocks per year (6s block time)
pub const BLOCKS_PER_YEAR: u128 = 5_256_000;
pub const ANNUAL_REWARD_PERCENT: u128 = 5;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct StakeInfo<AccountId> {
    pub validator: AccountId,
    pub amount: u128,
    pub staked_at: u32,
    pub last_reward_block: u32,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct ProverStakeInfo<AccountId> {
    pub prover: AccountId,
    pub amount: u128,
    pub staked_at: u32,
    pub proofs_verified: u64,
    pub slashes: u64,
    pub last_reward_block: u32,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    pub type ValidatorStakes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        StakeInfo<T::AccountId>,
        OptionQuery,
    >;

    #[pallet::storage]
    pub type ProverStakes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        ProverStakeInfo<T::AccountId>,
        OptionQuery,
    >;

    #[pallet::storage]
    pub type TotalSlashed<T: Config> = StorageValue<_, u128, ValueQuery>;

    #[pallet::storage]
    pub type TotalStaked<T: Config> = StorageValue<_, u128, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Staked {
            account: T::AccountId,
            amount: u128,
            role: Vec<u8>,
        },
        Unstaked {
            account: T::AccountId,
            amount: u128,
        },
        Slashed {
            prover: T::AccountId,
            amount: u128,
            reason: Vec<u8>,
        },
        RewardClaimed {
            account: T::AccountId,
            amount: u128,
        },
        ProofVerified {
            prover: T::AccountId,
            total_verified: u64,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        InsufficientFunds,
        NotStaked,
        BelowMinimumStake,
        NoRewardsToClaim,
        AlreadyStaked,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000_000)]
        pub fn stake_validator(origin: OriginFor<T>, amount: u128) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(amount >= MINIMUM_VALIDATOR_STAKE, Error::<T>::BelowMinimumStake);
            ensure!(!ValidatorStakes::<T>::contains_key(&who), Error::<T>::AlreadyStaked);

            let current_block: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();

            ValidatorStakes::<T>::insert(
                &who,
                StakeInfo {
                    validator: who.clone(),
                    amount,
                    staked_at: current_block,
                    last_reward_block: current_block,
                },
            );
            TotalStaked::<T>::mutate(|t| *t = t.saturating_add(amount));

            Self::deposit_event(Event::Staked { account: who, amount, role: b"validator".to_vec() });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(10_000_000)]
        pub fn stake_prover(origin: OriginFor<T>, amount: u128) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(amount >= MINIMUM_PROVER_STAKE, Error::<T>::BelowMinimumStake);
            ensure!(!ProverStakes::<T>::contains_key(&who), Error::<T>::AlreadyStaked);

            let current_block: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();

            ProverStakes::<T>::insert(
                &who,
                ProverStakeInfo {
                    prover: who.clone(),
                    amount,
                    staked_at: current_block,
                    proofs_verified: 0,
                    slashes: 0,
                    last_reward_block: current_block,
                },
            );
            TotalStaked::<T>::mutate(|t| *t = t.saturating_add(amount));

            Self::deposit_event(Event::Staked { account: who, amount, role: b"prover".to_vec() });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(10_000_000)]
        pub fn unstake(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            if let Some(stake) = ValidatorStakes::<T>::take(&who) {
                TotalStaked::<T>::mutate(|t| *t = t.saturating_sub(stake.amount));
                Self::deposit_event(Event::Unstaked { account: who, amount: stake.amount });
            } else if let Some(stake) = ProverStakes::<T>::take(&who) {
                TotalStaked::<T>::mutate(|t| *t = t.saturating_sub(stake.amount));
                Self::deposit_event(Event::Unstaked { account: who, amount: stake.amount });
            } else {
                return Err(Error::<T>::NotStaked.into());
            }

            Ok(())
        }

        /// Slash a prover for submitting an invalid proof (root-only)
        #[pallet::call_index(3)]
        #[pallet::weight(15_000_000)]
        pub fn slash_prover(
            origin: OriginFor<T>,
            prover: T::AccountId,
            reason: Vec<u8>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut stake = ProverStakes::<T>::get(&prover).ok_or(Error::<T>::NotStaked)?;
            let slash_amount = stake.amount.saturating_mul(SLASH_PERCENT) / 100;

            stake.amount = stake.amount.saturating_sub(slash_amount);
            stake.slashes = stake.slashes.saturating_add(1);
            ProverStakes::<T>::insert(&prover, &stake);

            TotalSlashed::<T>::mutate(|t| *t = t.saturating_add(slash_amount));
            TotalStaked::<T>::mutate(|t| *t = t.saturating_sub(slash_amount));

            Self::deposit_event(Event::Slashed { prover, amount: slash_amount, reason });
            Ok(())
        }

        /// Claim block rewards proportional to stake and time staked
        #[pallet::call_index(4)]
        #[pallet::weight(8_000_000)]
        pub fn claim_rewards(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let current_block: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();

            if let Some(mut stake) = ValidatorStakes::<T>::get(&who) {
                let reward = Self::compute_reward(stake.amount, stake.last_reward_block, current_block);
                ensure!(reward > 0, Error::<T>::NoRewardsToClaim);
                stake.last_reward_block = current_block;
                ValidatorStakes::<T>::insert(&who, &stake);
                Self::deposit_event(Event::RewardClaimed { account: who, amount: reward });
                return Ok(());
            }

            if let Some(mut stake) = ProverStakes::<T>::get(&who) {
                let reward = Self::compute_reward(stake.amount, stake.last_reward_block, current_block);
                ensure!(reward > 0, Error::<T>::NoRewardsToClaim);
                stake.last_reward_block = current_block;
                ProverStakes::<T>::insert(&who, &stake);
                Self::deposit_event(Event::RewardClaimed { account: who, amount: reward });
                return Ok(());
            }

            Err(Error::<T>::NotStaked.into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Compute reward = stake * APY * (blocks_since_last_claim / blocks_per_year)
        pub fn compute_reward(stake_amount: u128, from_block: u32, to_block: u32) -> u128 {
            let blocks_elapsed = (to_block as u128).saturating_sub(from_block as u128);
            if blocks_elapsed == 0 {
                return 0;
            }
            stake_amount
                .saturating_mul(ANNUAL_REWARD_PERCENT)
                .saturating_div(100)
                .saturating_mul(blocks_elapsed)
                .saturating_div(BLOCKS_PER_YEAR)
        }

        /// Record a proof verified by a prover (called by pallet-zk-verifier)
        pub fn record_proof_verified(prover: &T::AccountId) {
            ProverStakes::<T>::mutate(prover, |maybe| {
                if let Some(stake) = maybe {
                    stake.proofs_verified = stake.proofs_verified.saturating_add(1);
                }
            });
        }
    }
}
