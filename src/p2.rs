//! Goldilocks Poseidon2, width 8, S-box x^7, 4 + 22 + 4 rounds.
//!
//! A dependency-free transcription of the permutation Tirai's prover uses, for use inside a
//! Soroban contract (no_std, no Plonky3). Checked against vectors emitted by that implementation,
//! which is itself checked against upstream Plonky3. A MEASUREMENT harness, not a verifier.
use crate::consts::{BEGIN_RC, DIAG, END_RC, PARTIAL_RC};

/// p = 2^64 - 2^32 + 1
pub const P: u64 = 0xFFFF_FFFF_0000_0001;
/// 2^64 mod p
const EPS: u64 = 0xFFFF_FFFF;

pub const WIDTH: usize = 8;

#[inline(always)]
pub fn canon(x: u64) -> u64 {
    if x >= P { x - P } else { x }
}

#[inline(always)]
fn add(a: u64, b: u64) -> u64 {
    let (s, c1) = a.overflowing_add(b);
    let (s, c2) = s.overflowing_add(EPS * (c1 as u64));
    s.wrapping_add(EPS * (c2 as u64))
}

/// The textbook Goldilocks reduction: the form written against a machine that HAS a widening
/// multiply. WASM does not, so `x` itself is already an emulated 128-bit product here.
#[inline(always)]
fn reduce128(x: u128) -> u64 {
    let x_lo = x as u64;
    let x_hi = (x >> 64) as u64;
    let x_hi_hi = x_hi >> 32;
    let x_hi_lo = x_hi & EPS;

    let (mut t0, borrow) = x_lo.overflowing_sub(x_hi_hi);
    if borrow {
        t0 = t0.wrapping_sub(EPS);
    }
    let t1 = x_hi_lo.wrapping_mul(EPS);
    let (res, carry) = t0.overflowing_add(t1);
    res.wrapping_add(EPS * (carry as u64))
}

#[inline(always)]
fn mul(a: u64, b: u64) -> u64 {
    reduce128((a as u128) * (b as u128))
}

/// x^7, four products, exactly as the circuit computes it.
#[inline(always)]
fn sbox(x: u64) -> u64 {
    let x2 = mul(x, x);
    let x3 = mul(x2, x);
    let x4 = mul(x2, x2);
    mul(x4, x3)
}

/// [2 3 1 1; 1 2 3 1; 1 1 2 3; 3 1 1 2], additions only.
#[inline(always)]
fn mat4(x: &mut [u64]) {
    let t01 = add(x[0], x[1]);
    let t23 = add(x[2], x[3]);
    let t0123 = add(t01, t23);
    let t01123 = add(t0123, x[1]);
    let t01233 = add(t0123, x[3]);
    let x0 = x[0];
    let x2 = x[2];
    x[3] = add(add(t01233, x0), x0);
    x[1] = add(add(t01123, x2), x2);
    x[0] = add(t01123, t01);
    x[2] = add(t01233, t23);
}

#[inline(always)]
fn external(s: &mut [u64; WIDTH]) {
    mat4(&mut s[0..4]);
    mat4(&mut s[4..8]);
    for k in 0..4 {
        let sum = add(s[k], s[k + 4]);
        s[k] = add(s[k], sum);
        s[k + 4] = add(s[k + 4], sum);
    }
}

#[inline(always)]
fn internal(s: &mut [u64; WIDTH]) {
    let mut sum = s[0];
    let mut i = 1;
    while i < WIDTH {
        sum = add(sum, s[i]);
        i += 1;
    }
    for i in 0..WIDTH {
        s[i] = add(mul(s[i], DIAG[i]), sum);
    }
}

/// One permutation. Input and output are canonical.
pub fn permute(input: [u64; WIDTH]) -> [u64; WIDTH] {
    let mut s = input;
    external(&mut s);
    for rc in &BEGIN_RC {
        for i in 0..WIDTH {
            s[i] = sbox(add(s[i], rc[i]));
        }
        external(&mut s);
    }
    for rc in &PARTIAL_RC {
        s[0] = sbox(add(s[0], *rc));
        internal(&mut s);
    }
    for rc in &END_RC {
        for i in 0..WIDTH {
            s[i] = sbox(add(s[i], rc[i]));
        }
        external(&mut s);
    }
    let mut out = [0u64; WIDTH];
    for i in 0..WIDTH {
        out[i] = canon(s[i]);
    }
    out
}


pub fn mul_pub(a: u64, b: u64) -> u64 { mul(a, b) }

pub fn add_pub(a: u64, b: u64) -> u64 { add(a, b) }
