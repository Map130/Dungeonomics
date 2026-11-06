//! R6: кросс-языковой round-trip состояния (§5.9). Python-эмиттер пишет
//! golden/state_dump.toml, Rust читает его через `state_from_toml` и сверяет со
//! снимком golden/state_dump.json. Плюс проверяем, что Rust-эмиттер
//! (`state_to_toml`) сам себя переваривает (свой round-trip).
//!
//! Допуск 1e-9, а не бит-в-бит: парсеры `toml` и `serde_json` переводят одну и ту же
//! десятичную строку в f64 с расхождением до 1 ulp. По §9 это штатно («golden с
//! допуском»), а сериализация состояния по условию не влияет на детерминизм прогона.

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
    .unwrap_or_else(|_| panic!("golden {name}"))
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

    assert_eq!(state.stock.len(), want.stock.len(), "число ключей stock");
    for (k, &v) in &want.stock {
        assert_close(&format!("stock {k}"), state.stock[&key(k)], v);
    }
    assert_eq!(
        state.multiplier.len(),
        want.multiplier.len(),
        "число ключей mult"
    );
    for (k, &v) in &want.multiplier {
        assert_close(&format!("mult {k}"), state.multiplier[&key(k)], v);
    }

    assert_eq!(
        state.prosperity.len(),
        want.prosperity.len(),
        "число узлов π"
    );
    for (n, &v) in &want.prosperity {
        assert_close(&format!("pros {n}"), state.prosperity[n], v);
    }
    assert_eq!(
        state.caravan_accum.len(),
        want.caravan_accum.len(),
        "число дуг accum"
    );
    for (e, &v) in &want.caravan_accum {
        assert_close(&format!("accum {e}"), state.caravan_accum[e], v);
    }

    assert_eq!(
        state.pipeline.len(),
        want.pipeline.len(),
        "число дуг pipeline"
    );
    for (e, slots) in &want.pipeline {
        let got = &state.pipeline[e];
        assert_eq!(got.len(), slots.len(), "число слотов {e}");
        for (i, slot) in slots.iter().enumerate() {
            assert_eq!(got[i].len(), slot.len(), "номенклатура {e}[{i}]");
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
    // Эффекты/караваны не сериализуются — после загрузки они пусты.
    assert!(state.effects.is_empty());
    assert!(state.caravans.is_empty());
}

#[test]
fn rust_emitter_roundtrips() {
    let state = state_from_toml(&golden("state_dump.toml")).unwrap();
    // Переэмитить Rust-эмиттером и разобрать снова — состояние должно совпасть.
    let reparsed = state_from_toml(&state_to_toml(&state)).expect("re-parse");
    assert_eq!(
        reparsed, state,
        "round-trip через Rust-эмиттер сохраняет состояние"
    );
}
