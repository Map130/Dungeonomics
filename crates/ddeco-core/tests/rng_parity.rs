//! R4: the counter-based RNG matches Python bit for bit (golden/rng.json).
//!
//! The hash is integer-based, so EXACT equality is required here, not a tolerance.

use ddeco_core::rng::{chance, uniform};
use serde::Deserialize;

#[derive(Deserialize)]
struct Rng {
    seed: u64,
    samples: Vec<Sample>,
}

#[derive(Deserialize)]
struct Sample {
    day: i64,
    target: String,
    template: String,
    uniform_bits: u64,
    chance_0_1: bool,
}

#[test]
fn rng_matches_python_bit_for_bit() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../golden/rng.json"
    ))
    .expect(
        "golden/rng.json — golden/ is generated, not tracked: run `python3 scripts/dump_golden.py`",
    );
    let g: Rng = serde_json::from_str(&text).unwrap();
    for s in &g.samples {
        let parts = [s.target.as_str(), s.template.as_str()];
        let u = uniform(g.seed, s.day, &parts);
        assert_eq!(
            u.to_bits(),
            s.uniform_bits,
            "uniform({}, {}, {:?}) rust_bits={} python_bits={}",
            g.seed,
            s.day,
            parts,
            u.to_bits(),
            s.uniform_bits
        );
        assert_eq!(chance(g.seed, s.day, 0.1, &parts), s.chance_0_1);
    }
}
