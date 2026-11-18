//! Host smoke tests of the facade (rlib). Methods returning `JsValue` (snapshot/explain/
//! quote) need a JS runtime and are not called here — the wasm32 build and the node smoke
//! cover them in CI. Here we check state management: tick, run, undo, save.

use ddeco_wasm::{EffectSpec, Sim};

const WORLD: &str = include_str!("../../../worlds/example_region.toml");

/// A background shock template: a strong blight with a high rate, so that the effect
/// is guaranteed to fire in the test.
const SHOCK: &str = r#"
[[shocks]]
id = "blight"
target_kind = "node"
param = "pcap"
op = "mult"
value = 0.1
duration = 40.0
rate = 5.0
category = "food"
source = "blight"
"#;

#[test]
fn constructs_and_ticks() {
    let mut sim = Sim::new(WORLD, 0).expect("world from example_region.toml");
    assert_eq!(sim.day(), 0);
    sim.step();
    assert_eq!(sim.day(), 1);
    sim.run(10);
    assert_eq!(sim.day(), 11);
}

#[test]
fn undo_restores_previous_state() {
    let mut sim = Sim::new(WORLD, 0).expect("world");
    sim.run(5);
    let saved = sim.save();
    sim.step();
    assert_eq!(sim.day(), 6);

    assert!(sim.undo(), "undo the last step");
    assert_eq!(sim.day(), 5);
    assert_eq!(
        sim.save(),
        saved,
        "the state matches the snapshot before the step"
    );

    assert!(sim.undo(), "undo run(5)");
    assert_eq!(sim.day(), 0);
    assert!(!sim.undo(), "history exhausted");
}

#[test]
fn undo_replays_deterministically() {
    // Undoing and replaying the same day gives the same state bit for bit (§8.4).
    let mut sim = Sim::new(WORLD, 0).expect("world");
    sim.run(5);
    sim.step();
    let after_first = sim.save();
    assert!(sim.undo());
    sim.step();
    assert_eq!(sim.save(), after_first, "replaying the day is reproducible");
}

#[test]
fn warmup_advances_day() {
    let sim = Sim::new(WORLD, 50).expect("warmed-up world");
    assert!(sim.day() > 0, "warmup runs days until convergence");
}

#[test]
fn add_effect_changes_trajectory_and_undoes() {
    // Baseline without effects.
    let mut base = Sim::new(WORLD, 0).expect("world");
    base.run(20);
    let base_dump = base.save();

    // A strong blight on food production in a city changes the trajectory.
    let mut sim = Sim::new(WORLD, 0).expect("world");
    sim.add_effect(
        "node",
        "redwater",
        "pcap",
        "mult",
        0.05,
        40.0,
        "step",
        1.0,
        None,
        Some("food".into()),
        "blight",
    )
    .expect("effect accepted");
    sim.run(20);
    assert_ne!(sim.save(), base_dump, "the effect shifted the economy");

    // Undo removes both the effect and the run → back to the state before add_effect.
    assert!(sim.undo(), "undo the run");
    assert!(sim.undo(), "undo add_effect");
    assert_eq!(sim.day(), 0);
}

#[test]
fn shocks_toggle_affects_run_and_is_deterministic() {
    let world = format!("{WORLD}{SHOCK}");

    // Off (default) — as if there were no shocks.
    let mut off = Sim::new(&world, 0).expect("world");
    assert!(!off.shocks_enabled());
    off.run(20);

    // On — the blight fires, the trajectory differs.
    let mut on = Sim::new(&world, 0).expect("world");
    on.set_shocks_enabled(true);
    on.run(20);
    assert_ne!(on.save(), off.save(), "enabled shocks change the run");

    // Determinism: a second enabled run matches bit for bit (§8.4, §5.11).
    let mut on2 = Sim::new(&world, 0).expect("world");
    on2.set_shocks_enabled(true);
    on2.run(20);
    assert_eq!(on.save(), on2.save(), "shocks are reproducible by seed");
}

#[test]
fn save_load_roundtrip() {
    let mut sim = Sim::new(WORLD, 0).expect("world");
    sim.run(7);
    let dump = sim.save();
    let mut other = Sim::new(WORLD, 0).expect("world");
    other.load_state(&dump).expect("loading the snapshot");
    assert_eq!(other.day(), 7);
    assert_eq!(other.save(), dump, "the state after loading is identical");
}

fn spec(target_id: &str, param: &str, value: f64, category: Option<&str>) -> EffectSpec {
    EffectSpec {
        target_kind: "node".into(),
        target_id: target_id.into(),
        param: param.into(),
        op: "mult".into(),
        value,
        duration: 20.0,
        decay: "linear".into(),
        tau: 1.0,
        good: None,
        category: category.map(String::from),
        source: "preset".into(),
    }
}

#[test]
fn add_effect_specs_is_one_undo_step() {
    let mut sim = Sim::new(WORLD, 0).expect("world");
    sim.run(10);
    let before = sim.save();
    sim.add_effect_specs(vec![
        spec("redwater", "pcap", 0.05, Some("food")),
        spec("redwater", "population", 1.3, None),
    ])
    .expect("batch applies");
    sim.run(10);
    assert!(sim.undo(), "undo run(10)");
    assert!(sim.undo(), "undo the whole batch");
    assert_eq!(sim.save(), before, "both effects gone at once");
    assert!(sim.undo(), "undo the first run(10)");
    assert!(!sim.undo(), "nothing left");
}

#[test]
fn unknown_category_is_rejected() {
    let mut sim = Sim::new(WORLD, 0).expect("world");
    let err = sim
        .add_effect_specs(vec![spec("redwater", "pcap", 0.5, Some("nonsense"))])
        .expect_err("unknown category");
    assert!(err.contains("no category nonsense"), "{err}");
}

#[test]
fn invalid_spec_rejects_whole_batch() {
    let mut sim = Sim::new(WORLD, 0).expect("world");
    let before = sim.save();
    let err = sim
        .add_effect_specs(vec![
            spec("redwater", "pcap", 0.05, Some("food")),
            spec("nowhere", "pcap", 0.5, None),
        ])
        .expect_err("invalid target");
    assert!(err.contains("no node nowhere"), "{err}");
    assert_eq!(sim.save(), before);
    assert!(!sim.undo());
}

#[test]
fn empty_batch_is_rejected() {
    let mut sim = Sim::new(WORLD, 0).expect("world");
    assert_eq!(sim.add_effect_specs(vec![]), Err("no effects".to_string()));
    assert!(!sim.undo());
}
