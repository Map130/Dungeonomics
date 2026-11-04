//! Счётчиковый RNG (MODEL.md §5.11, §9). Порт `ddeco/rng.py`.
//!
//! Значение выводится из координат (seed, день, id цели, id шаблона), а не из
//! последовательности вызовов — откат и повтор дня дают тот же результат.
//!
//! Кодировка входа повторяет Python бит-в-бит, чтобы последовательности шоков и
//! охраны караванов совпадали между языками: `blake2b` (digest 16 байт), поля как
//! десятичные ASCII-строки через разделитель 0x1f; u64 из первых 8 байт
//! little-endian, делённый на 2^64.

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
    h.finalize_variable(&mut out).expect("16 байт");
    out
}

/// Равномерное число в [0, 1) для координат (seed, day, parts).
pub fn uniform(seed: u64, day: i64, parts: &[&str]) -> f64 {
    let d = digest(seed, day, parts);
    let value = u64::from_le_bytes(d[..8].try_into().unwrap());
    value as f64 / 2.0f64.powi(64)
}

/// True с вероятностью `probability`, детерминированно по координатам.
pub fn chance(seed: u64, day: i64, probability: f64, parts: &[&str]) -> bool {
    if probability <= 0.0 {
        return false;
    }
    if probability >= 1.0 {
        return true;
    }
    uniform(seed, day, parts) < probability
}
