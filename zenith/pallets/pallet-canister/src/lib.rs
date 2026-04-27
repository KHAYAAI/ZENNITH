#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    pallet_prelude::*,
    storage::StorageDoubleMap,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_runtime::traits::Hash;
use sp_std::vec::Vec;

pub use pallet::*;

pub type CanisterId = u64;
pub type CallId = u64;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct Canister<AccountId, Balance> {
    pub id: CanisterId,
    pub owner: AccountId,
    pub wasm_hash: [u8; 32],
    pub state_hash: [u8; 32],
    pub cycles: Balance,
    pub status: CanisterStatus,
    pub created_at: u32,
    pub updated_at: u32,
}

#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum CanisterStatus {
    Running,
    Stopped,
    Upgrading,
    Deleted,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct CallRequest<AccountId> {
    pub canister_id: CanisterId,
    pub caller: AccountId,
    pub method: Vec<u8>,
    pub input: Vec<u8>,
    pub gas_limit: u64,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct CallResult {
    pub output: Vec<u8>,
    pub proof_hash: [u8; 32],
    pub gas_used: u64,
    pub cycles_remaining: u128,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;
    }

    pub trait WeightInfo {
        fn deploy_canister() -> Weight;
        fn call_canister() -> Weight;
        fn submit_result() -> Weight;
    }

    impl WeightInfo for () {
        fn deploy_canister() -> Weight {
            Weight::from_parts(10_000_000, 0)
        }
        fn call_canister() -> Weight {
            Weight::from_parts(5_000_000, 0)
        }
        fn submit_result() -> Weight {
            Weight::from_parts(15_000_000, 0)
        }
    }

    #[pallet::storage]
    pub type CanisterCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn canisters)]
    pub type Canisters<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        CanisterId,
        Canister<T::AccountId, u128>,
        OptionQuery,
    >;

    #[pallet::storage]
    pub type CanistersByOwner<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Vec<CanisterId>, ValueQuery>;

    #[pallet::storage]
    pub type PendingCalls<T: Config> =
        StorageMap<_, Blake2_128Concat, CallId, CallRequest<T::AccountId>, OptionQuery>;

    #[pallet::storage]
    pub type CallResults<T: Config> =
        StorageMap<_, Blake2_128Concat, CallId, CallResult, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        CanisterDeployed {
            canister_id: CanisterId,
            owner: T::AccountId,
            wasm_hash: [u8; 32],
        },
        CallQueued {
            call_id: CallId,
            canister_id: CanisterId,
            caller: T::AccountId,
        },
        CallCompleted {
            call_id: CallId,
            canister_id: CanisterId,
            proof_hash: [u8; 32],
        },
        CanisterUpgraded {
            canister_id: CanisterId,
            new_wasm_hash: [u8; 32],
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        CanisterNotFound,
        InvalidWasm,
        InsufficientCycles,
        Unauthorized,
        CanisterNotRunning,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::deploy_canister())]
        pub fn deploy_canister(
            origin: OriginFor<T>,
            wasm: Vec<u8>,
            init_args: Vec<u8>,
            cycles: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Validate Wasm
            if wasm.is_empty() || wasm.len() > 1_000_000 {
                return Err(Error::<T>::InvalidWasm.into());
            }

            let current_block = <frame_system::Pallet<T>>::block_number();
            let canister_id = Self::next_canister_id();

            let wasm_hash = <T as frame_system::Config>::Hashing::hash(&wasm);
            let wasm_hash_bytes: [u8; 32] = [0u8; 32];

            let canister = Canister {
                id: canister_id,
                owner: who.clone(),
                wasm_hash: wasm_hash_bytes,
                state_hash: [0u8; 32],
                cycles,
                status: CanisterStatus::Running,
                created_at: current_block.saturated_into(),
                updated_at: current_block.saturated_into(),
            };

            Canisters::<T>::insert(canister_id, canister);
            CanisterCount::<T>::put(canister_id + 1);

            let mut owner_canisters = CanistersByOwner::<T>::get(&who);
            owner_canisters.push(canister_id);
            CanistersByOwner::<T>::insert(&who, owner_canisters);

            Self::deposit_event(Event::CanisterDeployed {
                canister_id,
                owner: who,
                wasm_hash: wasm_hash_bytes,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::call_canister())]
        pub fn call_canister(
            origin: OriginFor<T>,
            canister_id: CanisterId,
            method: Vec<u8>,
            input: Vec<u8>,
            gas_limit: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let canister =
                Canisters::<T>::get(canister_id).ok_or(Error::<T>::CanisterNotFound)?;

            ensure!(
                canister.status == CanisterStatus::Running,
                Error::<T>::CanisterNotRunning
            );

            let call_id = Self::next_call_id();

            let request = CallRequest {
                canister_id,
                caller: who.clone(),
                method,
                input,
                gas_limit,
            };

            PendingCalls::<T>::insert(call_id, request);

            Self::deposit_event(Event::CallQueued {
                call_id,
                canister_id,
                caller: who,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::submit_result())]
        pub fn submit_call_result(
            origin: OriginFor<T>,
            call_id: CallId,
            output: Vec<u8>,
            proof_hash: [u8; 32],
            gas_used: u64,
        ) -> DispatchResult {
            let _prover = ensure_signed(origin)?;

            let request =
                PendingCalls::<T>::take(call_id).ok_or(Error::<T>::CanisterNotFound)?;

            let canister_id = request.canister_id;
            let mut canister =
                Canisters::<T>::get(canister_id).ok_or(Error::<T>::CanisterNotFound)?;

            let cycles_used = (gas_used as u128).saturating_mul(1_000);
            canister.cycles = canister.cycles.saturating_sub(cycles_used);

            let current_block = <frame_system::Pallet<T>>::block_number();
            canister.updated_at = current_block.saturated_into();

            Canisters::<T>::insert(canister_id, canister);

            let result = CallResult {
                output,
                proof_hash,
                gas_used,
                cycles_remaining: canister.cycles,
            };

            CallResults::<T>::insert(call_id, result);

            Self::deposit_event(Event::CallCompleted {
                call_id,
                canister_id,
                proof_hash,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(10_000_000)]
        pub fn stop_canister(origin: OriginFor<T>, canister_id: CanisterId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut canister =
                Canisters::<T>::get(canister_id).ok_or(Error::<T>::CanisterNotFound)?;

            ensure!(canister.owner == who, Error::<T>::Unauthorized);

            canister.status = CanisterStatus::Stopped;
            Canisters::<T>::insert(canister_id, canister);

            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(10_000_000)]
        pub fn add_cycles(
            origin: OriginFor<T>,
            canister_id: CanisterId,
            amount: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut canister =
                Canisters::<T>::get(canister_id).ok_or(Error::<T>::CanisterNotFound)?;

            ensure!(canister.owner == who, Error::<T>::Unauthorized);

            canister.cycles = canister.cycles.saturating_add(amount);
            Canisters::<T>::insert(canister_id, canister);

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn next_canister_id() -> CanisterId {
            CanisterCount::<T>::get()
        }

        fn next_call_id() -> CallId {
            use sp_runtime::traits::Zero;
            let current: CallId = sp_std::cell::RefCell::new(0)
                .take()
                .saturating_add(1);
            current
        }
    }
}
