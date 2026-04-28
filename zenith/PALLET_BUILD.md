# Building and Deploying Zenith Substrate Pallets

This guide explains how to build the Substrate pallets in the excluded workspace members.

## Overview

The following pallets are excluded from the workspace due to Substrate version constraints:

- `pallet-token` - Native token implementation with balance tracking
- `pallet-staking` - Validator/prover staking with slashing and rewards
- `pallet-governance` - Proposal system with voting and execution
- `pallet-zk-verifier` - Proof verification engine
- `pallet-canister` - Canister deployment and execution coordination

## Building Pallets

### Individual Pallet Build

```bash
# Build a single pallet
cargo build -p pallet-token --release

# Test a single pallet (if tests exist)
cargo test -p pallet-token
```

### Build All Pallets

```bash
#!/bin/bash
# build-all-pallets.sh

set -e

for pallet in \
    pallet-canister \
    pallet-token \
    pallet-zk-verifier \
    pallet-staking \
    pallet-governance
do
    echo "Building $pallet..."
    cargo build -p $pallet --release
    echo "✓ $pallet built successfully"
done

echo "All pallets built successfully!"
```

Make it executable:
```bash
chmod +x build-all-pallets.sh
./build-all-pallets.sh
```

## Pallet Features & Dependencies

Each pallet requires:
- Substrate frame_support
- Substrate frame_system
- Codec (SCALE encoding)
- Serde (JSON serialization)

### pallet-canister (Lines 1-500)

**Purpose:** Manages WebAssembly canister deployment and execution state

**Key Types:**
- `Canister` - Deployed WASM code with metadata
- `PendingCall` - Pending method invocation
- `RoutingInfo` - Intelligent routing to ZK prover

**Extrinsics:**
- `deploy_canister(wasm, init_args, cycles)` - Deploy new canister
- `call_canister(canister_id, method, input, gas)` - Execute canister method

**Storage:**
- `Canisters: StorageMap<CanisterId, Canister>` - Active canisters
- `PendingCalls: StorageMap<CallId, PendingCall>` - In-flight calls
- `NextCallId: StorageValue<CallId>` - Call counter

### pallet-token (Lines 1-450+)

**Purpose:** Native token with transfer, mint, and burn operations

**Key Types:**
- `Account` - User account with balance
- `Balance` - Amount of ZEN tokens (u128)

**Extrinsics:**
- `transfer(to, amount)` - Transfer balance to another account
- `mint(to, amount)` - Mint new tokens (root only)
- `burn(from, amount)` - Burn tokens (root only)

**Storage:**
- `Balances: StorageMap<T::AccountId, u128, ValueQuery>` - Account balances
- `TotalIssuance: StorageValue<u128, ValueQuery>` - Total supply
- `TreasuryAccount: Config::Get<T::AccountId>` - Treasury address

**GenesisConfig:**
- Mints `INITIAL_SUPPLY` (100M ZEN) to treasury on chain initialization

### pallet-zk-verifier (Lines 1-300+)

**Purpose:** Verifies proofs from RISC Zero, Plonk, and Cairo prover systems

**Key Types:**
- `ProverSystem` - Enum: RiscZero | Plonk | Cairo
- `Proof` - Proof bytes with prover system identifier
- `VerificationResult` - Valid/Invalid outcome

**Extrinsics:**
- `submit_proof(proof_bytes, prover_system)` - Submit proof for verification

**Verification Logic:**
- RISC Zero: Check 96+ bytes with non-zero execution/receipt commitments
- Plonk: Check magic bytes (b"PLNK"), 100+ bytes, non-zero witness commitment
- Cairo: Check magic bytes (b"CAIR"), 100+ bytes, non-zero trace commitment

### pallet-staking (Lines 1-260+)

**Purpose:** Enables validator and prover staking with rewards and slashing

**Key Types:**
- `StakeInfo` - Validator stake record
- `ProverStakeInfo` - Prover stake with verification metrics

**Extrinsics:**
- `stake_validator(amount)` - Become a validator (min 1000 ZEN)
- `stake_prover(amount)` - Become a prover (min 500 ZEN)
- `unstake()` - Remove stake
- `claim_rewards()` - Claim 5% APY rewards
- `slash_prover(prover, reason)` - Slash 10% of stake (root only)

**Economics:**
- Annual Reward: 5% APY
- Slash Percent: 10% per slash
- Block Time: 6 seconds (~5.256M blocks per year)

### pallet-governance (Lines 1-187+)

**Purpose:** On-chain governance with proposals, voting, and execution

**Key Types:**
- `Proposal` - Governance proposal with voting state
- `Vote` - Individual vote (for/against)

**Extrinsics:**
- `create_proposal(title, description)` - Submit proposal
- `vote(proposal_id, for_it)` - Vote on proposal
- `execute_proposal(proposal_id)` - Execute passed proposal (requires quorum)

**Governance Rules:**
- Voting Period: 14,400 blocks (~1 day)
- Quorum: Minimum 10 votes required
- Passing Condition: votes_for > votes_against
- Vote Deduplication: Prevents double-voting via DoubleMap

## Integration with Runtime

After building, pallets must be integrated into `runtime/src/lib.rs`:

```rust
// Add pallet to construct_runtime! macro
construct_runtime! {
    pub struct Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Timestamp: pallet_timestamp,
        ...
        // Add our pallets
        PalletCanister: pallet_canister,
        PalletToken: pallet_token,
        PalletStaking: pallet_staking,
        PalletGovernance: pallet_governance,
        PalletZkVerifier: pallet_zk_verifier,
    }
}
```

## Testing Pallets Locally

### Start a Substrate Node with Pallets

```bash
cd zenith/node
cargo build --release
./target/release/zenith-node --dev
```

### Verify Pallet Extrinsics via Polkadot.js

1. Open Polkadot.js UI: https://polkadot.js.org/apps/?rpc=ws://localhost:9944
2. Select "Developer" → "Extrinsics"
3. Available transactions:
   - `palletToken.transfer(to, amount)`
   - `palletStaking.stakeValidator(amount)`
   - `palletStaking.claimRewards()`
   - `palletGovernance.createProposal(title, description)`
   - `palletGovernance.vote(proposal_id, for_it)`
   - `palletZkVerifier.submitProof(proof_bytes, prover_system)`

### Run Pallet Unit Tests

```bash
# Build tests (no test running for excluded workspace members)
cargo test -p pallet-token --lib
```

## Troubleshooting

### "cannot find pallet_X in workspace"

This is expected. Pallets are explicitly excluded from the workspace. Use:
```bash
cargo build -p pallet-name
```

### Type mismatch in runtime integration

Ensure the pallet types match the runtime's type aliases:

```rust
// In pallet
type Balance = u128;
type AccountId = sp_core::sr25519::Public;

// In runtime/lib.rs
impl pallet_token::Config for Runtime {
    type AccountId = AccountId;
    type Event = Event;
    // ... other type overrides
}
```

### Compilation fails with "frame_support not found"

Ensure Cargo.toml has correct Substrate dependencies:

```toml
[dependencies]
frame-support = { version = "28", default-features = false }
frame-system = { version = "28", default-features = false }
```

## Production Deployment

### 1. Build Release Binary

```bash
cargo build -p zenith-node --release
```

### 2. Initialize Chain State

```bash
./target/release/zenith-node \
  --chain mainnet \
  --base-path /data/zenith-chain
```

### 3. Fund Treasury Account (via GenesisConfig)

In `runtime/src/genesis_config.rs`:

```rust
let treasury_account = AccountId32::new([1; 32]);

pallet_token: PalletTokenConfig {
    initial_supply: 100_000_000 * 10u128.pow(12), // 100M ZEN
    treasury_account,
    _phantom: Default::default(),
}
```

### 4. Enable Validators

Register validators via sudo call:

```bash
# Via Polkadot.js UI
# "Sudo" → "palletToken.mint(validator_account, 1000_000_000_000_000)"
# "Sudo" → "palletStaking.stakeValidator(1000_000_000_000_000)"
```

## Pallet Dependencies Graph

```
┌──────────────────┐
│ pallet-canister  │
└────────┬─────────┘
         │
         ├──→ pallet-zk-verifier (proof verification)
         └──→ pallet-token (cycle payments)

┌──────────────────┐
│ pallet-staking   │
└────────┬─────────┘
         │
         └──→ pallet-token (stake/reward transfers)

┌──────────────────┐
│ pallet-governance│
└────────┬─────────┘
         │
         └──→ (no direct dependencies)

┌──────────────────┐
│ pallet-zk-verifier
└────────┬─────────┘
         │
         └──→ (no external pallet dependencies)

┌──────────────────┐
│ pallet-token     │
└────────────────────┘
         (foundational)
```

## Next Steps

1. Build all pallets: `./build-all-pallets.sh`
2. Integrate into runtime
3. Test locally with `--dev` node
4. Deploy to testnet
5. Deploy to mainnet with multi-sig treasury controls
