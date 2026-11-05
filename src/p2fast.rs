//! The same permutation with the widening multiply written against WASM's own instruction set.
//!
//! WASM has native i64 arithmetic but no 64x64 -> 128 product. Rust's `u128` multiply may lower
//! to a `__multi3` libcall, i.e. a real function call per field multiply. Here the high and low
//! halves are built explicitly from four 32-bit products, so every operation is one wasm i64
//! instruction and nothing is called.
use crate::consts::{BEGIN_RC, DIAG, END_RC, PARTIAL_RC};

const P: u64 = 0xFFFF_FFFF_0000_0001;
const EPS: u64 = 0xFFFF_FFFF;
const M32: u64 = 0xFFFF_FFFF;
pub const WIDTH: usize = 8;

#[inline(always)]
fn canon(x: u64) -> u64 {
    if x >= P { x - P } else { x }
}

#[inline(always)]
fn add(a: u64, b: u64) -> u64 {
    let (s, c1) = a.overflowing_add(b);
    let (s, c2) = s.overflowing_add(EPS * (c1 as u64));
    s.wrapping_add(EPS * (c2 as u64))
}

/// Four 32-bit products, no u128, no libcall.
#[inline(always)]
fn mul_hi_lo(a: u64, b: u64) -> (u64, u64) {
    let a_lo = a & M32;
    let a_hi = a >> 32;
    let b_lo = b & M32;
    let b_hi = b >> 32;

    let ll = a_lo.wrapping_mul(b_lo);
    let lh = a_lo.wrapping_mul(b_hi);
    let hl = a_hi.wrapping_mul(b_lo);
    let hh = a_hi.wrapping_mul(b_hi);

    let mid = (ll >> 32).wrapping_add(lh & M32).wrapping_add(hl & M32);
    let lo = (ll & M32) | (mid << 32);
    let hi = hh
        .wrapping_add(lh >> 32)
        .wrapping_add(hl >> 32)
        .wrapping_add(mid >> 32);
    (hi, lo)
}

#[inline(always)]
pub fn mul(a: u64, b: u64) -> u64 {
    let (x_hi, x_lo) = mul_hi_lo(a, b);
    let x_hi_hi = x_hi >> 32;
    let x_hi_lo = x_hi & EPS;

    let (t0, borrow) = x_lo.overflowing_sub(x_hi_hi);
    let t0 = t0.wrapping_sub(EPS * (borrow as u64));
    let t1 = x_hi_lo.wrapping_mul(EPS);
    let (res, carry) = t0.overflowing_add(t1);
    res.wrapping_add(EPS * (carry as u64))
}

#[inline(always)]
fn sbox(x: u64) -> u64 {
    let x2 = mul(x, x);
    let x3 = mul(x2, x);
    let x4 = mul(x2, x2);
    mul(x4, x3)
}

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
