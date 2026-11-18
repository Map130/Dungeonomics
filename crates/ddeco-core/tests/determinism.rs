//! R4: determinism (§8.4). Two runs with one seed match; undo (clone) and
//! replay give the same state. Shocks are on — this checks the counter-based RNG.
//!
//! A world without arcs (trade was a stub until R5), but with food — so that shocks of the
//! "food" category fire. The comparison is exact: `SimState` derives `PartialEq`.

use std::collections::BTreeMap;

use ddeco_core::calibration::seed_state;
use ddeco_core::effects::{Decay, Op, TargetKind};
use ddeco_core::goods::{Catalog, GoodParams, PriceClass};
use ddeco_core::shocks::ShockTemplate;
use ddeco_core::tick::step_day;
use ddeco_core::world::{GlobalParams, Node, NodeKind, World};

fn food(id: &str) -> GoodParams {
    let d = ddeco_core::goods::defaults_for(PriceClass::Necessity);
    GoodParams {
        id: id.into(),
        category: "food".into(),
        base_price: 1.0,
        alpha: d.alpha,
        epsilon: d.epsilon,
        beta: d.beta,
        m_min: d.m_min,
        m_max: d.m_max,
        spoilage: d.spoilage,
        coverage_days: d.coverage_days,
        c0: d.c0,
        kappa: d.kappa,
        per_capita: 1.0,
        volume: 1.0,
        weight: 1.0,
        in_food_basket: true,
        basket_weight: 1.0,
    }
}

fn node(id: &str, pop: f64, cap: f64) -> Node {
    Node {
        id: id.into(),
        kind: NodeKind::City,
        population: pop,
        prosperity0: 0.5,
        spread: 0.1,
        hub: None,
        hub_share: 0.6,
        capacity: BTreeMap::from([("grain".to_string(), cap)]),
        garrison: BTreeMap::new(),
    }
}

fn test_world() -> World {
    World {
        catalog: Catalog {
            goods: BTreeMap::from([("grain".to_string(), food("grain"))]),
        },
        nodes: BTreeMap::from([
            ("alpha".to_string(), node("alpha", 100.0, 120.0)),
            ("beta".to_string(), node("beta", 120.0, 90.0)),
        ]),
        edges: BTreeMap::new(),
        params: GlobalParams::default(),
        seed: 42,
    }
}

fn shocks() -> Vec<ShockTemplate> {
    vec![ShockTemplate {
        id: "blight".into(),
        target_kind: TargetKind::Node,
        param: "pcap".into(),
        op: Op::Mult,
        value: 0.3,
        duration: 15.0,
        rate: 1.0 / 20.0,
        decay: Decay::Linear,
        tau: 1.0,
        good: None,
        category: Some("food".into()),
        source: "blight".into(),
    }]
}

#[test]
fn two_runs_identical_with_shocks() {
    let w = test_world();
    let sh = shocks();
    let mut a = seed_state(&w);
    let mut b = seed_state(&w);
    for _ in 0..300 {
        step_day(&w, &mut a, &sh);
        step_day(&w, &mut b, &sh);
    }
    assert_eq!(a, b);
}

#[test]
fn rollback_and_replay() {
    let w = test_world();
    let sh = shocks();
    let mut st = seed_state(&w);
    for _ in 0..100 {
        step_day(&w, &mut st, &sh);
    }
    let checkpoint = st.clone(); // in-memory snapshot (§7)
    for _ in 0..50 {
        step_day(&w, &mut st, &sh);
    }
    let after = st.clone();

    let mut replay = checkpoint;
    for _ in 0..50 {
        step_day(&w, &mut replay, &sh);
    }
    assert_eq!(replay, after);
}

#[test]
fn shocks_actually_fire() {
    let w = test_world();
    let sh = shocks();
    let mut st = seed_state(&w);
    let mut fired = false;
    for _ in 0..300 {
        step_day(&w, &mut st, &sh);
        if !st.effects.is_empty() {
            fired = true;
            break;
        }
    }
    assert!(fired, "shocks never fired");
}
