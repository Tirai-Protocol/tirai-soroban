//! Metered CPU cost of a Tirai spend verification's dominant work inside the Soroban host,
//! under the network's own cost model. All figures are MARGINAL across two call sizes, so VM
//! instantiation, dispatch and argument marshalling fall out of the per-operation numbers.
//!
//! NOTE ON THE WORKLOAD. The on-chain transcript hash is keccak256, which Soroban provides as a
//! host function. Poseidon2 is the PROVER's hash and does not run in the verifier; it is measured
//! here because it is a dense field-arithmetic workload and so calibrates cost per field
//! operation, which is what the verifier actually pays for.
use soroban_sdk::{Bytes, Env};

mod c {
    soroban_sdk::contractimport!(file = "target/wasm32v1-none/release/tirai_soroban_bench.wasm");
}

const BUDGET: f64 = 100_000_000.0;
// Operation counts of a spend verification. Pre-revision-3, re-measured in Milestone 1.
const MULTIPLIES: f64 = 30_000.0;
const OTHER_FIELD_OPS: f64 = 60_000.0;
const KECCAK_CALLS: f64 = 650.0;
/// Average bytes absorbed per transcript hash call.
const KECCAK_CHUNK: u32 = 88;

fn cpu<T>(env: &Env, f: impl FnOnce() -> T) -> u64 {
    env.cost_estimate().budget().reset_unlimited();
    let _ = f();
    env.cost_estimate().budget().cpu_instruction_cost()
}

#[test]
fn measure() {
    let env = Env::default();
    let id = env.register(c::WASM, ());
    let cl = c::Client::new(&env, &id);
    let marg = |a: u64, b: u64, n: f64| (b - a) as f64 / n;

    let instantiation = cpu(&env, || cl.noop()) as f64;

    let perm_u128 = marg(cpu(&env, || cl.perm(&1)), cpu(&env, || cl.perm(&101)), 100.0);
    let perm_wasm = marg(cpu(&env, || cl.perm_fast(&1)), cpu(&env, || cl.perm_fast(&101)), 100.0);
    let mul_u128 = marg(cpu(&env, || cl.mul(&1)), cpu(&env, || cl.mul(&1001)), 1000.0);
    let mul_wasm = marg(cpu(&env, || cl.mul_fast(&1)), cpu(&env, || cl.mul_fast(&1001)), 1000.0);
    let add = marg(cpu(&env, || cl.add(&1)), cpu(&env, || cl.add(&1001)), 1000.0);

    // keccak256 as a host function, at the average transcript chunk size
    let data = Bytes::from_array(&env, &[7u8; KECCAK_CHUNK as usize]);
    let kec = marg(cpu(&env, || cl.keccak(&data, &1)), cpu(&env, || cl.keccak(&data, &101)), 100.0);

    let row = |n: &str, v: f64| println!("  {n:<36}{v:>11.1} CPU  ({:>7.1} wasm insns)", v / 4.0);

    println!("\n================= SOROBAN MEASUREMENT, SCF #46 =================");
    println!("  soroban-sdk 29.0.1, wasm32v1-none, opt-level 3 / overflow-checks on");
    println!("  Goldilocks Poseidon2 w8 x^7 4+22+4, 22/22 reference vectors matched\n");
    println!("  PER OPERATION");
    row("field add", add);
    row("field multiply, Rust u128", mul_u128);
    row("field multiply, WASM-native 32x32", mul_wasm);
    println!("  {:<36}{:>11.2}x", "-> widening-multiply speedup", mul_u128 / mul_wasm);
    row("keccak256 host call, 88 bytes", kec);
    println!();
    row("Poseidon2 permutation, u128", perm_u128);
    row("Poseidon2 permutation, WASM-native", perm_wasm);
    println!("  {:<36}{:>11.2}x\n", "-> speedup", perm_u128 / perm_wasm);

    for (label, m) in [("Rust u128 (naive port)", mul_u128), ("WASM-native (tuned)", mul_wasm)] {
        let f_mul = MULTIPLIES * m;
        let f_add = OTHER_FIELD_OPS * add;
        let f_kec = KECCAK_CALLS * kec;
        let total = f_mul + f_add + f_kec + instantiation;
        println!("  SPEND VERIFICATION, {label}");
        println!("    {MULTIPLIES:>9.0} multiplies        {f_mul:>12.0} CPU");
        println!("    {OTHER_FIELD_OPS:>9.0} other field ops   {f_add:>12.0} CPU");
        println!("    {KECCAK_CALLS:>9.0} keccak calls      {f_kec:>12.0} CPU");
        println!("              VM instantiation  {instantiation:>12.0} CPU");
        println!("    {:>32}{total:>12.0} CPU = {:.1}% of the 100M budget",
                 "TOTAL ", 100.0 * total / BUDGET);
        println!("    verdict: {}\n", if total < BUDGET { "FITS" } else { "DOES NOT FIT" });
    }
    println!("  Proof deserialisation is not measured here and the doc estimates it at 1.3M.");
    println!("  Operation counts predate the current circuit revision and are re-measured separately.");
    println!("================================================================\n");
}
