//! R6: cross-language state round-trip (§5.9). The Python emitter writes
//! golden/state_dump.toml, Rust reads it via `state_from_toml` and compares it with
//! the golden/state_dump.json snapshot. We also check that the Rust emitter
//! (`state_to_toml`) digests its own output (its own round-trip).
//!
//! Tolerance 1e-9, not bit-for-bit: the `toml` and `serde_json` parsers turn the same
//! decimal string into f64 with up to 1 ulp of difference. Per §9 this is expected ("golden with
//! tolerance"), and state serialisation by design does not affect the determinism of a run.

use std::collections::BTreeMap;

use ddeco_core::loader::{state_from_toml, state_to_toml};
use ddeco_core::state::SimState;
use serde::Deserialize;

const TOL: f64 = 1e-9;

fn golden(name: &str) -> String {
    std::fs::read_to_string(format!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/{}"),
        name
    ))
    .unwrap_or_else(|e| panic!("golden {name}: {e} — golden/ is generated, not tracked: run `python3 scripts/dump_golden.py`"))
}

#[derive(Deserialize)]
struct Snapshot {
    day: i64,
    stock: BTreeMap<String, f64>,      // "node|good" -> v
    multiplier: BTreeMap<String, f64>, // "node|good" -> v
    prosperity: BTreeMap<String, f64>,
    caravan_accum: BTreeMap<String, f64>,
    pipeline: BTreeMap<String, Vec<BTreeMap<String, f64>>>,
}

fn key(k: &str) -> (String, String) {
    let (n, g) = k.split_once('|').unwrap();
    (n.to_string(), g.to_string())
}

fn assert_close(label: &str, got: f64, want: f64) {
    let scale = want.abs().max(1.0);
    assert!(
        (got - want).abs() <= TOL * scale,
        "{label}: got={got} want={want} (|Δ|={:.3e})",
        (got - want).abs()
    );
}

fn check(state: &SimState, want: &Snapshot) {
    assert_eq!(state.day, want.day, "day");

    assert_eq!(state.stock.len(), want.stock.len(), "number of stock keys");
    for (k, &v) in &want.stock {
        assert_close(&format!("stock {k}"), state.stock[&key(k)], v);
    }
    assert_eq!(
        state.multiplier.len(),
        want.multiplier.len(),
        "number of mult keys"
    );
    for (k, &v) in &want.multiplier {
        assert_close(&format!("mult {k}"), state.multiplier[&key(k)], v);
    }

    assert_eq!(
        state.prosperity.len(),
        want.prosperity.len(),
        "number of π nodes"
    );
    for (n, &v) in &want.prosperity {
        assert_close(&format!("pros {n}"), state.prosperity[n], v);
    }
    assert_eq!(
        state.caravan_accum.len(),
        want.caravan_accum.len(),
        "number of accum arcs"
    );
    for (e, &v) in &want.caravan_accum {
        assert_close(&format!("accum {e}"), state.caravan_accum[e], v);
    }

    assert_eq!(
        state.pipeline.len(),
        want.pipeline.len(),
        "number of pipeline arcs"
    );
    for (e, slots) in &want.pipeline {
        let got = &state.pipeline[e];
        assert_eq!(got.len(), slots.len(), "number of slots {e}");
        for (i, slot) in slots.iter().enumerate() {
            assert_eq!(got[i].len(), slot.len(), "goods in {e}[{i}]");
            for (g, &v) in slot {
                assert_close(&format!("pipeline {e}[{i}] {g}"), got[i][g], v);
            }
        }
    }
}

#[test]
fn reads_python_emitter_output() {
    let want: Snapshot = serde_json::from_str(&golden("state_dump.json")).unwrap();
    let state = state_from_toml(&golden("state_dump.toml")).expect("parse state.toml");
    check(&state, &want);
    // Effects/caravans are not serialised — they are empty after loading.
    assert!(state.effects.is_empty());
    assert!(state.caravans.is_empty());
}

#[test]
fn rust_emitter_roundtrips() {
    let state = state_from_toml(&golden("state_dump.toml")).unwrap();
    // Re-emit with the Rust emitter and parse again — the state must match.
    let reparsed = state_from_toml(&state_to_toml(&state)).expect("re-parse");
    assert_eq!(
        reparsed, state,
        "round-trip through the Rust emitter preserves the state"
    );
}
