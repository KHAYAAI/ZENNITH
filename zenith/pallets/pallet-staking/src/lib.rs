#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{pallet_prelude::*, traits::Currency};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;

pub use pallet::*;

type BalanceOf<T> = <<T as pallet_balances::Config>::Balance as Default>::Default;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct StakeInfo<AccountId, Balance> {
    pub validator: AccountId,
    pub amount: Balance,
    pub staked_at: u32,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct ProverStakeInfo<AccountId, Balance> {
    pub prover: AccountId,
    pub amount: Balance,
    pub staked_at: u32,
    pub proofs_verified: u64,
    pub slashes: u64,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_balances::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    pub type ValidatorStakes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        StakeInfo<T::AccountId, <T as pallet_balances::Config>::Balance>,
        OptionQuery,
    >;

    #[pallet::storage]
    pub type ProverStakes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        ProverStakeInfo<T::AccountId, <T as pallet_balances::Config>::Balance>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Staked {
            account: T::AccountId,
            amount: <T as pallet_balances::Config>::Balance,
            role: Vec<u8>,
        },
        Unstaked {
            account: T::AccountId,
            amount: <T as pallet_balances::Config>::Balance,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        InsufficientFunds,
        NotStaked,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000_000)]
        pub fn stake_validator(
            origin: OriginFor<T>,
            amount: <T as pallet_balances::Config>::Balance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let current_block = <frame_system::Pallet<T>>::block_number();

            let stake = StakeInfo {
                validator: who.clone(),
                amount,
                staked_at: current_block.saturated_into(),
            };

            ValidatorStakes::<T>::insert(&who, stake);

            Self::deposit_event(Event::Staked {
                account: who,
                amount,
                role: b"validator".to_vec(),
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(10_000_000)]
        pub fn stake_prover(
            origin: OriginFor<T>,
            amount: <T as pallet_balances::Config>::Balance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let current_block = <frame_system::Pallet<T>>::block_number();

            let stake = ProverStakeInfo {
                prover: who.clone(),
                amount,
                staked_at: current_block.saturated_into(),
                proofs_verified: 0,
                slashes: 0,
            };

            ProverStakes::<T>::insert(&who, stake);

            Self::deposit_event(Event::Staked {
                account: who,
                amount,
                role: b"prover".to_vec(),
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(10_000_000)]
        pub fn unstake(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            if let Some(stake) = ValidatorStakes::<T>::take(&who) {
                Self::deposit_event(Event::Unstaked {
                    account: who,
                    amount: stake.amount,
                });
            } else if let Some(stake) = ProverStakes::<T>::take(&who) {
                Self::deposit_event(Event::Unstaked {
                    account: who,
                    amount: stake.amount,
                });
            }

            Ok(())
        }
    }
}
