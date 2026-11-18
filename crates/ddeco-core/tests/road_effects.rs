//! Road effects reach trade and caravans (§2, §5.6, §5.10): k_e and cap_e are effective values.
//! Mirror of `tests/test_road_effects.py`.

use ddeco_core::calibration::seed_state;
use ddeco_core::effects::{Decay, Effect, Op, TargetKind};
use ddeco_core::loader::world_from_toml;
use ddeco_core::state::SimState;
use ddeco_core::tick::{run, step_day};
use ddeco_core::world::World;

fn traded_world() -> (World, SimState) {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../worlds/example_region.toml"
    ))
    .expect("example_region.toml");
    let world = world_from_toml(&text).expect("parse");
    let mut st = seed_state(&world);
    run(&world, &mut st, 200, &[]);
    (world, st)
}

fn road_effect(st: &mut SimState, edge: &str, param: &str, value: f64, duration: f64) {
    st.effects.push(Effect {
        param: param.to_string(),
        target_kind: TargetKind::Edge,
        target_id: edge.to_string(),
        op: Op::Mult,
        value,
        start_day: st.day,
        duration,
        decay: Decay::Step,
        tau: 1.0,
        good: None,
        category: None,
        source: "test".to_string(),
    });
}

/// Volume put into the arc's pipeline by one step: total after − (total before − arrivals).
fn volume_total(world: &World, st: &SimState, edge: &str) -> (f64, f64) {
    let mut total = 0.0;
    let mut slot0 = 0.0;
    for (i, slot) in st.pipeline.get(edge).into_iter().flatten().enumerate() {
        for (g, amt) in slot {
            let v = world.catalog.get(g).volume * amt;
            total += v;
            if i == 0 {
                slot0 += v;
            }
        }
    }
    (total, slot0)
}

fn added_volume(world: &World, st: &mut SimState, edge: &str) -> f64 {
    let (before, arriving) = volume_total(world, st, edge);
    step_day(world, st, &[]);
    let (after, _) = volume_total(world, st, edge);
    after - (before - arriving)
}

#[test]
fn difficulty_effect_lengthens_delay() {
    let edge = "eastford__redwater";
    let (world, mut st) = traded_world();
    let (bworld, mut bst) = traded_world();
    road_effect(&mut st, edge, "difficulty", 2.5, 10.0);
    step_day(&world, &mut st, &[]);
    step_day(&bworld, &mut bst, &[]);
    let slots = &st.pipeline[edge];
    assert!(slots.len() >= 5 && !slots[4].is_empty());
    assert!(bst.pipeline[edge].len() <= 2);
}

#[test]
fn capacity_effect_limits_flow() {
    let edge = "redwater__eastford";
    let (world, mut st) = traded_world();
    let (bworld, mut bst) = traded_world();
    road_effect(&mut st, edge, "capacity", 0.1, 30.0);
    let mut limited = 0.0;
    for _ in 0..10 {
        let a = added_volume(&world, &mut st, edge);
        assert!(a <= 600.0 * 0.1 + 1e-9, "added {a}");
        limited += a;
    }
    let mut base = 0.0;
    for _ in 0..10 {
        base += added_volume(&bworld, &mut bst, edge);
    }
    assert!(limited < base, "limited={limited} base={base}");
}

#[test]
fn road_effect_expires() {
    let edge = "eastford__redwater";
    let (world, mut st) = traded_world();
    road_effect(&mut st, edge, "difficulty", 2.5, 5.0);
    run(&world, &mut st, 6, &[]);
    st.pipeline.insert(edge.to_string(), Vec::new());
    step_day(&world, &mut st, &[]);
    let slots = &st.pipeline[edge];
    assert_eq!(slots.len(), 2);
    assert!(!slots[1].is_empty());
}
