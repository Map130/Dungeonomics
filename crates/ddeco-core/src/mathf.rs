//! A single implementation of transcendental functions via `libm` (MODEL.md §9).
//!
//! `std::f64::powf/exp/ln` may give different last bits on different platforms
//! and in WASM. `libm` gives the same result on all targets, so within one
//! Rust build the simulation is bit-for-bit deterministic (native x86 == wasm32).
//!
//! All core code must call power/log/exponent arithmetic ONLY from here,
//! not via f64 methods, otherwise cross-target reproducibility is lost.

#[inline]
pub fn powf(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

#[inline]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

#[inline]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

#[inline]
pub fn clamp(x: f64, lo: f64, hi: f64) -> f64 {
    // Mirrors max(lo, min(hi, x)) from Python (ddeco/formulas.py:clamp).
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}
