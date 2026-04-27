/// Simple RISC Zero guest program that adds two numbers
/// This demonstrates the guest program that runs inside the zkVM

use risc0_zkvm::guest::env;

fn main() {
    // Read two u32 values from the host
    let a: u32 = env::read();
    let b: u32 = env::read();

    // Perform computation
    let c = a.wrapping_add(b);

    // Write result back to host
    env::commit(&c);
}
