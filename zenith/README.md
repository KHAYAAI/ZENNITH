# Zenith Cloud Protocol

A decentralized cloud for verifiable private computation. Zenith combines Substrate blockchain consensus with WebAssembly canisters, dual ZK proof systems, and intelligent routing to deliver cryptographically verifiable, hardware-agnostic computation.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Zenith Stack                        │
├─────────────────────────────────────────────────────┤
│  CLI (zenith-cli) │ Gateway (HTTP/JSON-RPC)         │
├─────────────────────────────────────────────────────┤
│  SDK (Rust)       │ Canister Runtime (Wasmtime)     │
├─────────────────────────────────────────────────────┤
│  ZK Router (Workload Analyzer)                      │
│  ├─ Plonk (Neural Networks)                         │
│  ├─ Cairo (Arithmetic)                              │
│  ├─ RISC Zero (General computation)                 │
│  └─ Aggregator (Unified format)                     │
├─────────────────────────────────────────────────────┤
│  Substrate Node (Validators, Provers, Consensus)    │
│  ├─ Runtime                                         │
│  ├─ Pallets:                                        │
│  │  ├─ pallet-canister (Wasm lifecycle)             │
│  │  ├─ pallet-zk-verifier (Proof verification)      │
│  │  ├─ pallet-governance (DAO)                      │
│  │  ├─ pallet-staking (Validator/Prover stakes)     │
│  │  └─ pallet-token ($ZEN)                          │
│  └─ Consensus (BABE + GRANDPA)                      │
└─────────────────────────────────────────────────────┘
```

## Key Features

- **Verifiable Computation**: All canister executions produce ZK proofs
- **Hardware-Agnostic**: No TEE dependency; purely cryptographic verification
- **Intelligent Routing**: Analyzes workloads and selects optimal ZK system
- **High Performance**: Batching, GPU acceleration, and proof aggregation
- **Decentralized**: No single authority; full governance via DAO
- **Private by Default**: Input/output/state all encrypted; proofs don't leak data

## Getting Started

### Prerequisites

```bash
# Install Rust 1.70+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add wasm32 target
rustup target add wasm32-unknown-unknown

# Install system dependencies
sudo apt-get install -y \
    clang curl libssl-dev llvm libudev-dev protobuf-compiler
```

### Building

```bash
# Clone and enter the zenith directory
cd zenith

# Build all crates
cargo build --all

# Build with release optimizations
cargo build --all --release

# Build Wasm runtime
cargo build --release --target wasm32-unknown-unknown -p zenith-runtime
```

### Running Tests

```bash
# Run all tests
cargo test --all

# Run with logging
RUST_LOG=debug cargo test --all -- --nocapture

# Run specific test
cargo test -p pallet-canister

# Run benchmarks
cargo bench -p zk-plonk
```

### Running a Local Node

```bash
# Development mode (single node, fast finality)
cargo run --release -p zenith-node -- --dev

# Local testnet (3 validators)
# See docker-compose.yml for full testnet setup

# With prover capability
cargo run --release -p zenith-node -- --prover

# Validator node
cargo run --release -p zenith-node -- --validator
```

### Using the CLI

```bash
# Initialize a new canister project
cargo run --release -p zenith-cli -- init my-model

# Build a canister
cd my-model && cargo run --release -p zenith-cli -- build

# Deploy to testnet
cargo run --release -p zenith-cli -- deploy --network testnet

# Call a canister method
cargo run --release -p zenith-cli -- call abc123 predict --arg '{"data": [1, 2, 3]}'

# Verify a proof
cargo run --release -p zenith-cli -- proof verify 0x1234...

# View proof history
cargo run --release -p zenith-cli -- proof history abc123

# Stream canister logs
cargo run --release -p zenith-cli -- logs abc123 --follow
```

### API Gateway

```bash
# Start the gateway
cargo run --release -p zenith-gateway -- --listen 0.0.0.0:8000

# Deploy a canister (HTTP)
curl -X POST http://localhost:8000/v1/canisters \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "wasm_base64": "AGFz...",
    "init_args_base64": "AAAA...",
    "cycles": 1000000
  }'

# Call a canister
curl -X POST http://localhost:8000/v1/canisters/abc123/call \
  -d '{
    "method": "predict",
    "input_base64": "AAAA...",
    "gas_limit": 100000
  }'

# Check call result
curl http://localhost:8000/v1/canisters/abc123/calls/call-id-xyz

# Verify a proof
curl http://localhost:8000/v1/proofs/0x1234.../verify
```

## Project Structure

```
zenith/
├── node/                    # Substrate node binary
├── runtime/                 # Substrate runtime
├── pallets/
│   ├── pallet-canister/     # Wasm canister management
│   ├── pallet-zk-verifier/  # On-chain proof verification
│   ├── pallet-governance/   # DAO governance
│   ├── pallet-staking/      # Validator/prover staking
│   └── pallet-token/        # $ZEN token
├── zk-layer/
│   ├── router/              # Workload analyzer & router
│   ├── plonk/               # Halo2 proving system
│   ├── cairo/               # Cairo/Starkware system
│   ├── risc-zero/           # RISC Zero system
│   └── aggregator/          # Proof aggregation
├── canister-runtime/        # Wasmtime executor
├── gateway/                 # HTTP API gateway
├── sdk/rust/                # Rust SDK for clients
├── zenith-cli/              # Developer CLI
├── .github/workflows/       # CI/CD pipelines
└── README.md
```

## Development Workflow

### 1. Add a New Pallet

```bash
mkdir -p zenith/pallets/pallet-my-feature/src
# Edit zenith/Cargo.toml to add member
# Create src/lib.rs with frame_support::pallet macro
cargo build -p pallet-my-feature
```

### 2. Build a Example Canister

```bash
mkdir -p examples/my-app/src
# Write Wasm canister code in Rust
cargo build --target wasm32-unknown-unknown --release -p my-app
zenith deploy examples/target/wasm32-unknown-unknown/release/my_app.wasm
```

### 3. Deploy to Network

```bash
# Local dev
zenith deploy --network local

# Testnet
zenith deploy --network testnet

# Mainnet
zenith deploy --network mainnet
```

### 4. Monitor Proofs

```bash
zenith proof history my-canister-id
zenith proof verify proof-hash-xyz
```

## Configuration

### Network Configuration (zenith.toml)

```toml
[project]
name = "my-dapp"
version = "0.1.0"

[canister]
wasm = "target/wasm32-unknown-unknown/release/my_dapp.wasm"
id = ""

[network]
default = "testnet"

[network.local]
url = "http://localhost:8000"

[network.testnet]
url = "https://testnet.zenith.cloud"

[network.mainnet]
url = "https://api.zenith.cloud"
```

### Node Configuration

```bash
# Set log level
RUST_LOG=debug ./zenith-node --dev

# Custom data directory
./zenith-node --dev --base-path /custom/path
```

## Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Single inference (CPU) | <2000ms | Small model, 3 layers |
| Single inference (GPU) | <200ms | With Nvidia GPU |
| Batch 100 (GPU) | <500ms | 5ms amortized |
| Proof verification | <100ms | On-chain |
| Canister call latency | <500ms | Including batching |
| Block time | 6 seconds | BABE consensus |
| Finality | 150 blocks | GRANDPA |

## Security

- **Private Input/Output**: All canister I/O encrypted with threshold cryptography
- **Deterministic Execution**: Wasm runs in isolated, deterministic VM
- **Proof Validation**: Every proof verified on-chain; invalid proofs slash prover
- **Stake-Based Security**: Validators and provers must stake $ZEN tokens
- **DAO Governance**: Token holders control protocol upgrades

## Token Economics

**$ZEN Token:**
- Transaction fees
- Validator/prover staking (earn rewards)
- Canister cycles/gas payment
- Governance voting

**Cycle Cost Model:**
- 1 cycle = 1 nanosecond equivalent compute
- Canister call: ~10,000 cycles (overhead)
- 1M cycles ≈ $0.01 (subject to market)

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Commit changes: `git commit -am 'Add my feature'`
4. Push: `git push origin feat/my-feature`
5. Submit a pull request

## Testing Requirements

- All new code must have unit tests (`#[test]`)
- Integration tests in `tests/integration/`
- Benchmarks for performance-critical code
- Minimum 80% code coverage

## CI/CD

GitHub Actions automatically:
- Runs `cargo fmt`, `cargo clippy`, `cargo test` on every PR
- Builds release binaries on merge to main
- Runs security audits (`cargo audit`)
- Benchmarks ZK proving systems weekly

## Roadmap

- [ ] Prompt 2: Full Substrate node implementation
- [ ] Prompt 3: pallet-canister (Wasm lifecycle)
- [ ] Prompt 4: pallet-zk-verifier (proof verification)
- [ ] Prompt 5: ZK router (workload analysis)
- [ ] Prompt 6: Plonk/Halo2 prover (GPU support)
- [ ] Prompt 7: API Gateway
- [ ] Prompt 8: Zenith CLI
- [ ] Prompt 9: Canister runtime (Wasmtime)
- [ ] Prompt 10: Integration tests + CI/CD
- [ ] Prompt 11: Example canisters (Radiology, Credit Scoring, Voting)

## License

Licensed under the Apache 2.0 License. See LICENSE file for details.

## Support

- **Docs**: [github.com/khayaai/zennith/docs](https://github.com/khayaai/zennith/docs)
- **Discord**: [Join our community](https://discord.gg/zenith)
- **Issues**: [GitHub Issues](https://github.com/khayaai/zennith/issues)
- **Email**: support@zenith.cloud

---

**Built with ❤️ for verifiable computation.**
