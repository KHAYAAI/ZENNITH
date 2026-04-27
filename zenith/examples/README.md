# Zenith Example Canisters

This directory contains real, working examples of Wasm canisters that run on Zenith.
Each example demonstrates a different workload pattern and shows how Zenith routes them
to the optimal ZK proving system.

## Examples

### 1. Neural Network Inference (Plonk/Halo2)
- **Path**: `neural-network-ai/`
- **Workload**: Dense matrix multiplication + ReLU activations
- **Optimal Prover**: Plonk/Halo2 (polynomial constraints)
- **Use Case**: Medical imaging analysis, object detection
- **Characteristics**:
  - High neural_network_density (~0.8+)
  - Low control_flow_complexity
  - Sequential memory access

### 2. Credit Scoring (Cairo/Starkware)
- **Path**: `credit-scoring/`
- **Workload**: Pure arithmetic branching (decision tree)
- **Optimal Prover**: Cairo (arithmetic circuits)
- **Use Case**: Fair lending decisions with ZK proof of non-discrimination
- **Characteristics**:
  - High arithmetic_purity (~0.7+)
  - Low memory randomness
  - Simple control flow

### 3. Private Voting (RISC Zero Fallback)
- **Path**: `private-voting/`
- **Workload**: Complex branching + encryption
- **Optimal Prover**: RISC Zero (universal fallback)
- **Use Case**: Anonymous voting aggregation
- **Characteristics**:
  - Moderate all densities
  - Complex control flow
  - Use when other systems don't fit

## Building Examples

```bash
# Build one canister
cd neural-network-ai
cargo build --target wasm32-unknown-unknown --release

# Build all
for dir in */; do
  (cd "$dir" && cargo build --target wasm32-unknown-unknown --release)
done
```

## Testing with Zenith

```bash
# Deploy neural network canister
zenith deploy examples/neural-network-ai/target/wasm32-unknown-unknown/release/neural_network_ai.wasm

# Call with sample input
zenith call <canister-id> infer --arg '{"data": [0.5, 0.3, 0.8, ...]}'

# Verify proof was generated with correct prover
zenith proof history <canister-id>
```

## Workload Analysis

Each canister will be analyzed by the ZK Router:

```
WasmAnalyzer.analyze(wasm_bytes)
  ↓
WorkloadProfile {
  neural_network_density: 0.82,
  arithmetic_purity: 0.05,
  control_flow_complexity: 0.08,
  ...
}
  ↓
SystemScorer.score(profile)
  ↓
RoutingScores {
  plonk: 0.85,    ← Winner
  cairo: 0.20,
  risc_zero: 1.0
}
  ↓
Dispatcher.select(scores)
  ↓
ProverSystem::Plonk
```

## Performance Expectations

| Canister | Input Size | Native Time | Proof Time | Proving System |
|----------|-----------|-------------|-----------|----------------|
| Neural Network (GPU) | 1000 floats | 5ms | 50-200ms | Plonk |
| Neural Network (CPU) | 1000 floats | 50ms | 500-2000ms | Plonk |
| Credit Scoring | 20 features | 1ms | 20-100ms | Cairo |
| Private Voting | 10k votes | 100ms | 500-1500ms | RISC Zero |

## Extending Examples

To create your own canister:

1. Create a new Cargo library crate:
   ```bash
   cargo new --lib my-canister
   ```

2. Implement canister interface:
   ```rust
   #[no_mangle]
   pub extern "C" fn execute(input_ptr: i32, input_len: i32) -> i32 {
       // Your computation here
   }
   ```

3. Build for Wasm:
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```

4. Deploy via Zenith:
   ```bash
   zenith deploy target/wasm32-unknown-unknown/release/my_canister.wasm
   ```

The ZK Router will automatically analyze your Wasm and route to the best proving system!
