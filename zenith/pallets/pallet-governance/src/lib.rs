#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_std::vec::Vec;

pub use pallet::*;

/// Minimum votes needed to execute a proposal
pub const QUORUM_THRESHOLD: u128 = 10;
/// Voting period in blocks (~1 day at 6s/block)
pub const VOTING_PERIOD_BLOCKS: u32 = 14_400;

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
    pub executed: bool,
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

    /// Tracks who has voted on each proposal to prevent double-voting
    #[pallet::storage]
    pub type Votes<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u64,         // proposal_id
        Blake2_128Concat,
        T::AccountId, // voter
        bool,        // vote (true = for, false = against)
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ProposalCreated { proposal_id: u64, proposer: T::AccountId },
        Voted { proposal_id: u64, voter: T::AccountId, for_it: bool },
        ProposalExecuted { proposal_id: u64 },
        ProposalRejected { proposal_id: u64 },
    }

    #[pallet::error]
    pub enum Error<T> {
        ProposalNotFound,
        AlreadyVoted,
        VotingStillOpen,
        VotingEnded,
        QuorumNotMet,
        ProposalFailed,
        AlreadyExecuted,
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
            let current_block: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
            let deadline = current_block.saturating_add(VOTING_PERIOD_BLOCKS);

            let proposal = Proposal {
                id: proposal_id,
                proposer: who.clone(),
                title,
                description,
                votes_for: 0,
                votes_against: 0,
                created_at: current_block,
                deadline,
                executed: false,
            };

            Proposals::<T>::insert(proposal_id, proposal);
            ProposalCount::<T>::put(proposal_id.saturating_add(1));

            Self::deposit_event(Event::ProposalCreated { proposal_id, proposer: who });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(5_000_000)]
        pub fn vote(origin: OriginFor<T>, proposal_id: u64, for_it: bool) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(!Votes::<T>::contains_key(proposal_id, &who), Error::<T>::AlreadyVoted);

            let mut proposal =
                Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(!proposal.executed, Error::<T>::AlreadyExecuted);

            let current_block: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
            ensure!(current_block <= proposal.deadline, Error::<T>::VotingEnded);

            if for_it {
                proposal.votes_for = proposal.votes_for.saturating_add(1);
            } else {
                proposal.votes_against = proposal.votes_against.saturating_add(1);
            }

            Proposals::<T>::insert(proposal_id, &proposal);
            Votes::<T>::insert(proposal_id, &who, for_it);

            Self::deposit_event(Event::Voted { proposal_id, voter: who, for_it });
            Ok(())
        }

        /// Execute a passed proposal after voting has ended
        #[pallet::call_index(2)]
        #[pallet::weight(20_000_000)]
        pub fn execute_proposal(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            ensure_signed(origin)?;

            let mut proposal =
                Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(!proposal.executed, Error::<T>::AlreadyExecuted);

            let current_block: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
            ensure!(current_block > proposal.deadline, Error::<T>::VotingStillOpen);

            let total_votes = proposal.votes_for.saturating_add(proposal.votes_against);
            ensure!(total_votes >= QUORUM_THRESHOLD, Error::<T>::QuorumNotMet);
            ensure!(proposal.votes_for > proposal.votes_against, Error::<T>::ProposalFailed);

            proposal.executed = true;
            Proposals::<T>::insert(proposal_id, &proposal);

            Self::deposit_event(Event::ProposalExecuted { proposal_id });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Check if a proposal passed (for off-chain querying)
        pub fn proposal_passed(proposal_id: u64) -> bool {
            match Proposals::<T>::get(proposal_id) {
                Some(p) if p.executed => true,
                _ => false,
            }
        }

        /// Get the current vote tally for a proposal
        pub fn vote_tally(proposal_id: u64) -> (u128, u128) {
            match Proposals::<T>::get(proposal_id) {
                Some(p) => (p.votes_for, p.votes_against),
                None => (0, 0),
            }
        }
    }
}
