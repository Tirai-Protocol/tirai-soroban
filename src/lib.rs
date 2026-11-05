//! Measurement contract for SCF #46 Milestone 1: what one Goldilocks Poseidon2 permutation
//! costs inside Soroban, where WASM has no 64x64 -> 128 widening multiply.
//!
//! Each entry point runs `n` operations so the marginal cost of one can be read off two calls,
//! which removes contract dispatch and host overhead from the figure. Results are returned so
//! that nothing can be optimised away.
#![no_std]
pub mod consts;
pub mod p2;
pub mod p2fast;

use soroban_sdk::{contract, contractimpl, Bytes, Env};

#[contract]
pub struct Bench;

#[contractimpl]
impl Bench {
    /// Dispatch baseline: the cost of being called at all.
    pub fn noop(_e: Env) -> u64 {
        0
    }

    /// `n` chained permutations.
    pub fn perm(_e: Env, n: u32) -> u64 {
        let mut s: [u64; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut i = 0u32;
        while i < n {
            s = p2::permute(s);
            i += 1;
        }
        s[0]
    }

    /// `n` chained field multiplies: the emulated widening multiply on its own.
    pub fn mul(_e: Env, n: u32) -> u64 {
        let mut a: u64 = 0x123456789abcdef;
        let mut i = 0u32;
        while i < n {
            a = p2::mul_pub(a, 0xdeadbeefcafe);
            i += 1;
        }
        a
    }

    /// `n` chained field additions, for comparison.
    pub fn add(_e: Env, n: u32) -> u64 {
        let mut a: u64 = 0x123456789abcdef;
        let mut i = 0u32;
        while i < n {
            a = p2::add_pub(a, 0xdeadbeefcafe);
            i += 1;
        }
        a
    }
    /// `n` chained permutations, WASM-native widening multiply.
    pub fn perm_fast(_e: Env, n: u32) -> u64 {
        let mut s: [u64; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut i = 0u32;
        while i < n {
            s = p2fast::permute(s);
            i += 1;
        }
        s[0]
    }

    /// `n` chained field multiplies, WASM-native widening multiply.
    pub fn mul_fast(_e: Env, n: u32) -> u64 {
        let mut a: u64 = 0x123456789abcdef;
        let mut i = 0u32;
        while i < n {
            a = p2fast::mul(a, 0xdeadbeefcafe);
            i += 1;
        }
        a
    }

    /// `n` keccak256 host calls over `data`: the on-chain transcript hash.
    pub fn keccak(e: Env, data: Bytes, n: u32) -> u32 {
        let mut acc = 0u32;
        let mut i = 0u32;
        while i < n {
            let h = e.crypto().keccak256(&data);
            acc = acc.wrapping_add(h.to_array()[0] as u32);
            i += 1;
        }
        acc
    }

}
