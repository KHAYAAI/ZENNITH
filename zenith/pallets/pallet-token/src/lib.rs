#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_runtime::traits::Zero;

pub use pallet::*;

pub const TOKEN_DECIMALS: u8 = 12;
pub const TOKEN_SYMBOL: &[u8] = b"ZEN";
pub const INITIAL_SUPPLY: u128 = 100_000_000 * 10_u128.pow(12); // 100M ZEN

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct TokenInfo {
    pub name: [u8; 32],
    pub symbol: [u8; 8],
    pub decimals: u8,
    pub total_supply: u128,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// The account that holds the initial token treasury
        type TreasuryAccount: Get<T::AccountId>;
    }

    #[pallet::storage]
    pub type TokenMetadata<T: Config> = StorageValue<_, TokenInfo, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn balance_of)]
    pub type Balances<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        u128,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type TotalIssuance<T: Config> = StorageValue<_, u128, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Transfer {
            from: T::AccountId,
            to: T::AccountId,
            amount: u128,
        },
        Minted {
            to: T::AccountId,
            amount: u128,
        },
        Burned {
            from: T::AccountId,
            amount: u128,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        InsufficientBalance,
        InvalidAmount,
        Overflow,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(5_000_000)]
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId,
            amount: u128,
        ) -> DispatchResult {
            let from = ensure_signed(origin)?;
            ensure!(amount > 0, Error::<T>::InvalidAmount);

            let from_balance = Balances::<T>::get(&from);
            ensure!(from_balance >= amount, Error::<T>::InsufficientBalance);

            Balances::<T>::mutate(&from, |b| *b = b.saturating_sub(amount));
            Balances::<T>::mutate(&to, |b| *b = b.saturating_add(amount));

            Self::deposit_event(Event::Transfer { from, to, amount });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(3_000_000)]
        pub fn mint(
            origin: OriginFor<T>,
            to: T::AccountId,
            amount: u128,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(amount > 0, Error::<T>::InvalidAmount);

            Balances::<T>::mutate(&to, |b| *b = b.saturating_add(amount));
            TotalIssuance::<T>::mutate(|t| *t = t.saturating_add(amount));

            // Update metadata total supply
            TokenMetadata::<T>::mutate(|info| {
                info.total_supply = info.total_supply.saturating_add(amount);
            });

            Self::deposit_event(Event::Minted { to, amount });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(3_000_000)]
        pub fn burn(
            origin: OriginFor<T>,
            from: T::AccountId,
            amount: u128,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(amount > 0, Error::<T>::InvalidAmount);

            let balance = Balances::<T>::get(&from);
            ensure!(balance >= amount, Error::<T>::InsufficientBalance);

            Balances::<T>::mutate(&from, |b| *b = b.saturating_sub(amount));
            TotalIssuance::<T>::mutate(|t| *t = t.saturating_sub(amount));

            TokenMetadata::<T>::mutate(|info| {
                info.total_supply = info.total_supply.saturating_sub(amount);
            });

            Self::deposit_event(Event::Burned { from, amount });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Internal transfer without origin check (for pallets)
        pub fn do_transfer(from: &T::AccountId, to: &T::AccountId, amount: u128) -> DispatchResult {
            ensure!(amount > 0, Error::<T>::InvalidAmount);
            let from_balance = Balances::<T>::get(from);
            ensure!(from_balance >= amount, Error::<T>::InsufficientBalance);
            Balances::<T>::mutate(from, |b| *b = b.saturating_sub(amount));
            Balances::<T>::mutate(to, |b| *b = b.saturating_add(amount));
            Ok(())
        }

        /// Internal mint without origin check (for pallets like staking rewards)
        pub fn do_mint(to: &T::AccountId, amount: u128) {
            if amount > 0 {
                Balances::<T>::mutate(to, |b| *b = b.saturating_add(amount));
                TotalIssuance::<T>::mutate(|t| *t = t.saturating_add(amount));
            }
        }

        /// Internal burn without origin check
        pub fn do_burn(from: &T::AccountId, amount: u128) -> DispatchResult {
            ensure!(amount > 0, Error::<T>::InvalidAmount);
            let balance = Balances::<T>::get(from);
            ensure!(balance >= amount, Error::<T>::InsufficientBalance);
            Balances::<T>::mutate(from, |b| *b = b.saturating_sub(amount));
            TotalIssuance::<T>::mutate(|t| *t = t.saturating_sub(amount));
            Ok(())
        }
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: sp_std::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            let mut name = [0u8; 32];
            let symbol_vec = b"ZEN ";
            let mut symbol = [0u8; 8];
            symbol[..symbol_vec.len()].copy_from_slice(symbol_vec);
            name[..6].copy_from_slice(b"Zenith");

            let token_info = TokenInfo {
                name,
                symbol,
                decimals: TOKEN_DECIMALS,
                total_supply: INITIAL_SUPPLY,
            };

            TokenMetadata::<T>::put(token_info);
            TotalIssuance::<T>::put(INITIAL_SUPPLY);

            // Mint initial supply to treasury account
            let treasury = T::TreasuryAccount::get();
            Balances::<T>::insert(&treasury, INITIAL_SUPPLY);
        }
    }
}
