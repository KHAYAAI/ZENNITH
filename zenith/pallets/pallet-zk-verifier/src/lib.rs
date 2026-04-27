#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_std::vec::Vec;

pub use pallet::*;

#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ProverSystem {
    RiscZero,
    Plonk,
    Cairo,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct ZenithProof<AccountId> {
    pub proof_hash: [u8; 32],
    pub computation_hash: [u8; 32],
    pub result_hash: [u8; 32],
    pub prover_system: ProverSystem,
    pub prover_node: AccountId,
    pub timestamp: u32,
    pub canister_id: u64,
    pub call_id: u64,
    pub proof_bytes: Vec<u8>,
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
        fn submit_proof() -> Weight;
        fn batch_submit_proofs() -> Weight;
    }

    impl WeightInfo for () {
        fn submit_proof() -> Weight {
            Weight::from_parts(20_000_000, 0)
        }
        fn batch_submit_proofs() -> Weight {
            Weight::from_parts(100_000_000, 0)
        }
    }

    #[pallet::storage]
    pub type ProofCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn proofs)]
    pub type Proofs<T: Config> =
        StorageMap<_, Blake2_128Concat, [u8; 32], ZenithProof<T::AccountId>, OptionQuery>;

    #[pallet::storage]
    pub type ProofsByCanister<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64,
        Vec<[u8; 32]>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type ProofsByProver<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Vec<[u8; 32]>, ValueQuery>;

    #[pallet::storage]
    pub type AuthorizedProvers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ProofVerified {
            proof_hash: [u8; 32],
            prover: T::AccountId,
            prover_system: ProverSystem,
        },
        ProofRejected {
            proof_hash: [u8; 32],
            prover: T::AccountId,
            reason: Vec<u8>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidProof,
        UnauthorizedProver,
        VerificationFailed,
        DuplicateProof,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_proof())]
        pub fn submit_proof(
            origin: OriginFor<T>,
            proof: ZenithProof<T::AccountId>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify prover is authorized
            ensure!(
                AuthorizedProvers::<T>::get(&who),
                Error::<T>::UnauthorizedProver
            );

            // Check for duplicates
            ensure!(
                !Proofs::<T>::contains_key(proof.proof_hash),
                Error::<T>::DuplicateProof
            );

            // Verify proof based on prover system
            Self::verify_proof(&proof)?;

            // Store proof
            Proofs::<T>::insert(proof.proof_hash, proof.clone());

            // Update indexes
            let mut canister_proofs = ProofsByCanister::<T>::get(proof.canister_id);
            canister_proofs.push(proof.proof_hash);
            ProofsByCanister::<T>::insert(proof.canister_id, canister_proofs);

            let mut prover_proofs = ProofsByProver::<T>::get(&who);
            prover_proofs.push(proof.proof_hash);
            ProofsByProver::<T>::insert(&who, prover_proofs);

            ProofCount::<T>::put(ProofCount::<T>::get() + 1);

            Self::deposit_event(Event::ProofVerified {
                proof_hash: proof.proof_hash,
                prover: who,
                prover_system: proof.prover_system,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::batch_submit_proofs())]
        pub fn batch_submit_proofs(
            origin: OriginFor<T>,
            proofs: Vec<ZenithProof<T::AccountId>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                AuthorizedProvers::<T>::get(&who),
                Error::<T>::UnauthorizedProver
            );

            for proof in proofs {
                if !Proofs::<T>::contains_key(proof.proof_hash) {
                    let _ = Self::verify_proof(&proof);
                    Proofs::<T>::insert(proof.proof_hash, proof.clone());

                    let mut canister_proofs = ProofsByCanister::<T>::get(proof.canister_id);
                    canister_proofs.push(proof.proof_hash);
                    ProofsByCanister::<T>::insert(proof.canister_id, canister_proofs);

                    let mut prover_proofs = ProofsByProver::<T>::get(&who);
                    prover_proofs.push(proof.proof_hash);
                    ProofsByProver::<T>::insert(&who, prover_proofs);
                }
            }

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(10_000_000)]
        pub fn authorize_prover(origin: OriginFor<T>, prover: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            AuthorizedProvers::<T>::insert(&prover, true);
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(10_000_000)]
        pub fn unauthorize_prover(origin: OriginFor<T>, prover: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            AuthorizedProvers::<T>::remove(&prover);
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn verify_proof(proof: &ZenithProof<T::AccountId>) -> DispatchResult {
            match proof.prover_system {
                ProverSystem::RiscZero => {
                    // Placeholder: In production, call actual risc0 verifier
                    ensure!(!proof.proof_bytes.is_empty(), Error::<T>::InvalidProof);
                }
                ProverSystem::Plonk => {
                    // Placeholder: In production, call actual Halo2 verifier
                    ensure!(!proof.proof_bytes.is_empty(), Error::<T>::InvalidProof);
                }
                ProverSystem::Cairo => {
                    // Placeholder: In production, call actual Cairo verifier
                    ensure!(!proof.proof_bytes.is_empty(), Error::<T>::InvalidProof);
                }
            }
            Ok(())
        }

        pub fn get_proofs_for_canister(canister_id: u64) -> Vec<[u8; 32]> {
            ProofsByCanister::<T>::get(canister_id)
        }

        pub fn get_proofs_by_prover(prover: &T::AccountId) -> Vec<[u8; 32]> {
            ProofsByProver::<T>::get(prover)
        }
    }
}
