# Goldilocks Poseidon2 on Soroban: measured cost

A measurement harness, not a verifier. It answers one question: what does field arithmetic over
a 64-bit prime actually cost inside Soroban, given that WASM has no 64x64 to 128 widening
multiply?

This is the groundwork for bringing [Tirai](https://tirai.io), a post-quantum private transfer
protocol, to Stellar. Tirai's proofs are hash based and need no trusted setup. Verifying one is
dominated by field arithmetic and by keccak256, so the viability of a Soroban verifier rests on
the per-operation cost of exactly those two things.

Measured 4 November 2026 on Stellar testnet.

- Contract `CAEZZNQ2QEYF2UVBCXIOHYVP6QSIJJHMGKYKMERZ7JGH7JUJZFLRVM47`
- Deploy transaction `0ab9f8f3864e6a0dba075b16bc286edef63b046784bd90057f649870edf4bb6e`
- soroban-sdk 29.0.1, target `wasm32v1-none`, opt-level 3, overflow checks on

## Correctness before cost

Both implementations here reproduce all 22 reference vectors of the permutation Tirai's prover
uses, which is itself checked against upstream Plonky3. `cargo test --test vectors` is the gate.
An unvalidated benchmark measures nothing, and a 31-bit field would have sailed through this
while testing none of the hard part, since it needs no widening multiply at all.

## Results

Per operation, read from `simulateTransaction` on live testnet. Every figure is marginal across
two call sizes, so VM instantiation, contract dispatch and argument marshalling fall out.

| Operation | CPU instructions | WASM instructions |
|---|---|---|
| Field add | 90.1 | 22.5 |
| Field multiply, Rust `u128` | 664 | 166 |
| Field multiply, WASM-native 32x32 | 332 | 83 |
| keccak256 host call, 88 bytes | 13,726 | host function |
| Poseidon2 permutation, `u128` | 387,695 | 96,924 |
| Poseidon2 permutation, WASM-native | 212,192 | 53,048 |

A local `soroban-sdk` host run agreed with testnet to within 4%, which is what `cargo test --test
bench` reproduces: 373,008 and 204,320 for the two permutations, 13,560 for keccak256, identical
figures for add and both multiplies, and the same 2.00x and 1.83x ratios. The table above is the
live network, which is the number that counts.

### The widening multiply is worth 2.00x

WASM has no 64x64 to 128 multiply instruction. Write the field multiply the ordinary way, with
Rust's `u128`, and it costs 664 CPU instructions, because the multiply lowers to a library call.
Build the high and low halves explicitly from four 32-bit products, so every operation is a
native `i64` instruction and nothing is called, and it costs 332.

That is a 2.00x difference on the single hottest operation, and 1.83x on the whole permutation.
`src/p2.rs` is the ordinary version and `src/p2fast.rs` is the tuned one. They are kept side by
side so the gap stays measurable rather than remembered.

### A spend verification fits the budget

Soroban's CPU budget is 100,000,000 instructions per transaction. Operation counts below are
Tirai's current published figures and predate a circuit revision, so they are an estimate of the
workload rather than a measurement of it.

| | Rust `u128` | WASM-native |
|---|---|---|
| 30,000 field multiplies | 19,920,000 | 9,960,000 |
| 60,000 other field ops | 5,406,000 | 5,406,000 |
| 650 keccak256 calls | 8,921,900 | 8,921,900 |
| **Total** | **34,247,900** | **24,287,900** |
| Share of the budget | 34.2% | 24.3% |
| Verdict | fits | fits |

It fits either way. Writing the arithmetic for the platform roughly halves the field-arithmetic
half of the bill, which buys room for the parts not measured here.

## Reproducing it

Needs the [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/stellar-cli)
and a Rust toolchain.

```sh
rustup target add wasm32v1-none
cargo test --test vectors            # correctness gate, 22 vectors, both variants
stellar contract build               # must be this, see below
cargo test --test bench -- --nocapture
```

To read the numbers off the live network rather than the local host:

```sh
stellar keys generate bench --network testnet --fund
stellar contract deploy --wasm target/wasm32v1-none/release/tirai_soroban_bench.wasm \
  --source bench --network testnet
```

Then invoke `perm` and `perm_fast` at two different `n` values and subtract. The RPC no longer
returns a `cost` field, so read `resources.instructions` by decoding the simulation's
`transactionData` as `SorobanTransactionData`.

### Three things that cost time

**Build with `stellar contract build`, not `cargo build --target wasm32-unknown-unknown`.**
On Rust 1.82 and later that target enables reference-types and multi-value, which Soroban
rejects at upload. The correct target is `wasm32v1-none`.

**Pin ed25519-dalek if you use an older SDK.** `soroban-env-host` 22 resolves ed25519-dalek 3.0
and then fails to compile. `cargo update -p ed25519-dalek --precise 2.2.0`.

**`--cost` prints nothing** on Stellar CLI v30. Decode `transactionData` instead.

## What this does not settle

- Proof deserialisation is not measured. Separate work estimates it at about 1.3M instructions.
- Poseidon2 does not run in the verifier. Tirai's on-chain transcript hash is keccak256, which
  Soroban provides as a host function. The permutation is measured because it is a dense
  field-arithmetic workload, so it calibrates cost per field operation, which is what a verifier
  does pay for.
- Per-operation costs come from tight loops. A real verifier carries more memory traffic.
- The budget is 100,000,000 in `soroban-env-host` 29.0.1. SLP-0004 proposes 400,000,000 but it is
  not live, and nothing here assumes it.

## Scope

This repository contains the permutation and the benchmark harness. Tirai's prover, circuit and
proof system internals are not part of it and remain proprietary.

## Licence

Apache 2.0. See [LICENSE](LICENSE).
