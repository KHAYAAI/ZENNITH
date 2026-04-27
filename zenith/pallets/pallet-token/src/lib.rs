#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;

pub use pallet::*;

pub const TOKEN_DECIMALS: u8 = 12;
pub const TOKEN_SYMBOL: &[u8] = b"ZEN";

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
    }

    #[pallet::storage]
    pub type TokenMetadata<T: Config> = StorageValue<_, TokenInfo, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Transfer {
            from: T::AccountId,
            to: T::AccountId,
            amount: u128,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        InsufficientBalance,
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

            Self::deposit_event(Event::Transfer {
                from,
                to,
                amount,
            });

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig;

    impl Default for GenesisConfig {
        fn default() -> Self {
            Self
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig {
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
                total_supply: 100_000_000 * 10_u128.pow(TOKEN_DECIMALS as u32),
            };

            TokenMetadata::<T>::put(token_info);
        }
    }
}
