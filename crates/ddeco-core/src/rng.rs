//! Counter-based RNG (MODEL.md §5.11, §9). Port of `ddeco/rng.py`.
//!
//! A value is derived from coordinates (seed, day, target id, template id), not from
//! a call sequence — undoing and replaying a day gives the same result.
//!
//! The input encoding mirrors Python bit for bit, so the sequences of shocks and
//! caravan guards match across languages: `blake2b` (16-byte digest), fields as
//! decimal ASCII strings joined by 0x1f; a u64 from the first 8 bytes
//! little-endian, divided by 2^64.

use blake2::digest::{Update, VariableOutput};
use blake2::Blake2bVar;

const SEP: &[u8] = b"\x1f";

fn digest(seed: u64, day: i64, parts: &[&str]) -> [u8; 16] {
    let mut h = Blake2bVar::new(16).expect("16 ≤ 64");
    h.update(seed.to_string().as_bytes());
    h.update(SEP);
    h.update(day.to_string().as_bytes());
    for p in parts {
        h.update(SEP);
        h.update(p.as_bytes());
    }
    let mut out = [0u8; 16];
    h.finalize_variable(&mut out).expect("16 bytes");
    out
}

/// A uniform number in [0, 1) for the coordinates (seed, day, parts).
pub fn uniform(seed: u64, day: i64, parts: &[&str]) -> f64 {
    let d = digest(seed, day, parts);
    let value = u64::from_le_bytes(d[..8].try_into().unwrap());
    value as f64 / 2.0f64.powi(64)
}

/// True with probability `probability`, deterministically by coordinates.
pub fn chance(seed: u64, day: i64, probability: f64, parts: &[&str]) -> bool {
    if probability <= 0.0 {
        return false;
    }
    if probability >= 1.0 {
        return true;
    }
    uniform(seed, day, parts) < probability
}
