#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_std::vec::Vec;

pub use pallet::*;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct Proposal<AccountId> {
    pub id: u64,
    pub proposer: AccountId,
    pub title: Vec<u8>,
    pub description: Vec<u8>,
    pub votes_for: u128,
    pub votes_against: u128,
    pub created_at: u32,
    pub deadline: u32,
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
    pub type ProposalCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type Proposals<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, Proposal<T::AccountId>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ProposalCreated { proposal_id: u64, proposer: T::AccountId },
        Voted { proposal_id: u64, voter: T::AccountId, for_it: bool },
    }

    #[pallet::error]
    pub enum Error<T> {
        ProposalNotFound,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000_000)]
        pub fn create_proposal(
            origin: OriginFor<T>,
            title: Vec<u8>,
            description: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let proposal_id = ProposalCount::<T>::get();
            let current_block = <frame_system::Pallet<T>>::block_number();

            let proposal = Proposal {
                id: proposal_id,
                proposer: who.clone(),
                title,
                description,
                votes_for: 0,
                votes_against: 0,
                created_at: current_block.saturated_into(),
                deadline: (current_block + 10000u32.into()).saturated_into(),
            };

            Proposals::<T>::insert(proposal_id, proposal);
            ProposalCount::<T>::put(proposal_id + 1);

            Self::deposit_event(Event::ProposalCreated {
                proposal_id,
                proposer: who,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(5_000_000)]
        pub fn vote(origin: OriginFor<T>, proposal_id: u64, for_it: bool) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut proposal =
                Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            if for_it {
                proposal.votes_for = proposal.votes_for.saturating_add(1);
            } else {
                proposal.votes_against = proposal.votes_against.saturating_add(1);
            }

            Proposals::<T>::insert(proposal_id, proposal);

            Self::deposit_event(Event::Voted {
                proposal_id,
                voter: who,
                for_it,
            });

            Ok(())
        }
    }
}
