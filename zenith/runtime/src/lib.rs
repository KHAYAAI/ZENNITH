#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    construct_runtime, derive_impl, dispatch::DispatchClass, parameter_types,
    traits::{AsEnsureOriginForSystemShell, ConstU128, ConstU32, ConstU8, EitherOfDiverse},
    weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use frame_system::limits::{BlockLength, BlockWeights};
use scale_info::TypeInfo;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, Verify},
    transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, MultiSignature,
};
use sp_std::prelude::*;
use sp_version::RuntimeVersion;

pub use pallet_balances::Call as BalancesCall;

pub use frame_support::weights::constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};

// Opaque block type
pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

/// Opaque types. These are used by external interfaces to call into the runtime.
pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;

    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;

    impl_opaque_keys! {
        pub struct SessionKeys {
            pub aura: Aura,
            pub grandpa: Grandpa,
        }
    }
}

pub use opaque::{Block, UncheckedExtrinsic};

// Runtime version
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("zenith"),
    impl_name: create_runtime_str!("zenith"),
    authoring_version: 1,
    spec_version: 100,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    state_version: 1,
};

// Block number type
pub type BlockNumber = u32;
pub type Nonce = u32;
pub type Hash = sp_core::H256;
pub type AccountId = <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Balance = u128;
pub type Index = u32;

// Signed Extra type for extrinsics
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

// Signed Transaction type
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;

// Runtime parameters
parameter_types! {
    pub const Version: RuntimeVersion = VERSION;

    pub const SS58Prefix: u8 = 42;

    pub const BlockHashCount: BlockNumber = 2400;

    pub BlockWeights: BlockWeights = BlockWeights::with_base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            weights.reserved = Some(OPERATIONAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        });

    pub BlockLength: BlockLength = BlockLength::max_with_normal_ratio(
        5 * 1024 * 1024,
        NORMAL_DISPATCH_RATIO,
    );

    pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
    pub const OPERATIONAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(25);
    pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
        2u64 * WEIGHT_REF_TIME_PER_SECOND,
        u64::MAX,
    );
}

use sp_runtime::Perbill;

// Frame System configuration
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = BlockWeights;
    type BlockLength = BlockLength;
    type DbWeight = RocksDbWeight;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type Nonce = Nonce;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = AccountIdLookup<AccountId, ()>;
    type Block = Block;
    type BlockHashCount = BlockHashCount;
    type Version = Version;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnSetCode = ();
    type SS58Prefix = SS58Prefix;
    type MaxConsumers = ConstU32<16>;
}

// Timestamp configuration
parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
    pub const SLOT_DURATION: u64 = 6000;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

// Aura configuration
impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<32>;
    type AllowMultipleBlocksPerSlot = frame_support::traits::ConstBool<false>;
    type SlotDuration = pallet_aura::SlotDuration;
}

// Grandpa configuration
impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_grandpa::weights::SubstrateWeight<Runtime>;
    type MaxAuthorities = ConstU32<32>;
    type MaxNominators = ConstU32<0>;
    type MaxSetIdSessionEntries = ConstU64<0>;
    type KeyOwnerProof = ();
    type EquivocationReportSystem = ();
}

// Balances configuration
parameter_types! {
    pub const ExistentialDeposit: Balance = 500;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type FreezeIdentifier = ();
    type MaxFrozen = ConstU32<0>;
    type MaxHolds = ConstU32<0>;
    type RuntimeHoldReason = ();
}

// Transaction Payment configuration
parameter_types! {
    pub const TransactionByteFee: Balance = 1;
    pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type WeightToFee = frame_support::weights::IdentityFee<Balance>;
    type LengthToFee = frame_support::weights::ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = ();
}

// Sudo configuration
impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

// Session configuration
parameter_types! {
    pub const Period: BlockNumber = 6;
    pub const Offset: BlockNumber = 0;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ValidatorIdOf = ();
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = ();
    type SessionHandler = <opaque::SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdToAccountId;
    type Keys = opaque::SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

// Local pallet configurations
impl pallet_canister::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

impl pallet_zk_verifier::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

impl pallet_governance::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

impl pallet_staking::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

impl pallet_token::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

// Construct the runtime with all pallets
construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,
        Sudo: pallet_sudo,
        Session: pallet_session,

        // Zenith-specific pallets
        Canister: pallet_canister,
        ZkVerifier: pallet_zk_verifier,
        Governance: pallet_governance,
        Staking: pallet_staking,
        Token: pallet_token,
    }
);

// Implement runtime API
#[cfg(feature = "std")]
impl sp_runtime::traits::GetNodeBlockType for Runtime {
    type NodeBlock = Block;
}

#[cfg(feature = "std")]
impl sp_runtime::traits::GetRuntimeBlockType for Runtime {
    type RuntimeBlock = Block;
}

// Runtime APIs
impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            frame_executive::Executive::<Runtime, frame_system::ChainContext<Runtime>>::execute_block(block);
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            frame_executive::Executive::<Runtime, frame_system::ChainContext<Runtime>>::initialize_block(header);
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version).map(Into::into)
        }

        fn metadata_versions() -> sp_std::vec::Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            frame_executive::Executive::<Runtime, frame_system::ChainContext<Runtime>>::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            frame_executive::Executive::<Runtime, frame_system::ChainContext<Runtime>>::finalize_block()
        }

        fn inherent_extrinsics(_data: sp_inherents::InherentData) -> sp_std::vec::Vec<<Block as BlockT>::Extrinsic> {
            sp_std::vec![]
        }

        fn check_inherents(
            _block: Block,
            _data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            sp_inherents::CheckInherentsResult::new()
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            frame_executive::Executive::<Runtime, frame_system::ChainContext<Runtime>>::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(_header: &<Block as BlockT>::Header) {
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
        }

        fn authorities() -> sp_std::vec::Vec<AuraId> {
            Aura::authorities().to_vec()
        }
    }

    impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> sp_consensus_grandpa::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            _equivocation_proof: sp_consensus_grandpa::EquivocationProof<
                <Block as BlockT>::Hash,
                sp_runtime::traits::NumberFor<Block>,
            >,
            _key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            None
        }

        fn generate_key_ownership_proof(
            _set_id: sp_consensus_grandpa::SetId,
            _authority_id: GrandpaId,
        ) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
            None
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(_seed: Option<sp_std::vec::Vec<u8>>) -> sp_std::vec::Vec<u8> {
            opaque::SessionKeys::generate(seed.as_ref().map(|b| (&b[..]).into())).encode()
        }

        fn decode_session_keys(
            encoded: sp_std::vec::Vec<u8>,
        ) -> Option<sp_std::vec::Vec<(sp_std::vec::Vec<u8>, sp_std::vec::Vec<u8>)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
    }
}

const SLOT_DURATION: u64 = 6000;
