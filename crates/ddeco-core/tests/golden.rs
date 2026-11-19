//! Checking the Rust core against golden runs of the Python reference (R3: single-node scenarios).
//!
//! Numbers are compared with a relative tolerance of 1e-9 (different libm, §9). The other
//! scenarios (example world, shocks, events) live in the R4/R5 test files.

use std::collections::BTreeMap;

use ddeco_core::goods::{Catalog, GoodParams};
use ddeco_core::state::{blank_state, SimState};
use ddeco_core::tick::step_day;
use ddeco_core::world::{GlobalParams, Node, NodeKind, World};
use serde::Deserialize;

const TOL: f64 = 1e-9;

#[derive(Deserialize)]
struct Frame {
    day: i64,
    stock: BTreeMap<String, f64>,
    multiplier: BTreeMap<String, f64>,
    prosperity: BTreeMap<String, f64>,
}

#[derive(Deserialize)]
struct SingleNode {
    params: Params,
    days: u32,
    frames: Vec<Frame>,
}

#[derive(Deserialize)]
struct Params {
    alpha: f64,
    beta: f64,
    eps: f64,
    #[serde(rename = "T")]
    t: f64,
    demand: f64,
}

fn golden(name: &str) -> String {
    std::fs::read_to_string(format!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/{}.json"),
        name
    ))
    .unwrap_or_else(|e| panic!("golden {name}: {e} — golden/ is generated, not tracked: run `python3 scripts/dump_golden.py`"))
}

fn single_node_world(p: &Params) -> World {
    let good = GoodParams {
        id: "x".into(),
        category: "test".into(),
        base_price: 1.0,
        alpha: p.alpha,
        epsilon: p.eps,
        beta: p.beta,
        m_min: 0.1,
        m_max: 10.0,
        spoilage: 0.0,
        coverage_days: p.t,
        c0: 1.0,
        kappa: 0.0,
        per_capita: 1.0,
        volume: 1.0,
        weight: 1.0,
        in_food_basket: false,
        basket_weight: 0.0,
    };
    let node = Node {
        id: "n".into(),
        kind: NodeKind::City,
        population: p.demand,
        prosperity0: 0.5,
        spread: 0.1,
        hub: None,
        hub_share: 0.6,
        capacity: BTreeMap::from([("x".to_string(), p.demand)]),
        garrison: BTreeMap::new(),
    };
    World {
        catalog: Catalog {
            goods: BTreeMap::from([("x".to_string(), good)]),
        },
        nodes: BTreeMap::from([("n".to_string(), node)]),
        edges: BTreeMap::new(),
        params: GlobalParams::default(),
        seed: 0,
    }
}

fn assert_close(label: &str, got: f64, want: f64) {
    let scale = want.abs().max(1.0);
    assert!(
        (got - want).abs() <= TOL * scale,
        "{label}: rust={got} python={want} (|Δ|={:.3e})",
        (got - want).abs()
    );
}

fn check_frame(state: &SimState, frame: &Frame) {
    for (k, &want) in &frame.stock {
        let (n, g) = k.split_once('|').unwrap();
        assert_close(
            &format!("day {} stock {k}", frame.day),
            state.stock[&(n.to_string(), g.to_string())],
            want,
        );
    }
    for (k, &want) in &frame.multiplier {
        let (n, g) = k.split_once('|').unwrap();
        assert_close(
            &format!("day {} mult {k}", frame.day),
            state.multiplier[&(n.to_string(), g.to_string())],
            want,
        );
    }
    for (k, &want) in &frame.prosperity {
        assert_close(
            &format!("day {} pros {k}", frame.day),
            state.prosperity[k],
            want,
        );
    }
}

fn run_single(name: &str) {
    let sc: SingleNode = serde_json::from_str(&golden(name)).unwrap();
    let world = single_node_world(&sc.params);
    let mut state = blank_state(&world);
    let star = sc.params.t * sc.params.demand;
    state.stock.insert(("n".into(), "x".into()), 0.8 * star);
    state.multiplier.insert(("n".into(), "x".into()), 1.0);

    let mut it = sc.frames.iter();
    let mut next = it.next();
    if let Some(fr) = next {
        if fr.day == 0 {
            check_frame(&state, fr);
            next = it.next();
        }
    }
    for _ in 0..sc.days {
        step_day(&world, &mut state, &[]);
        if let Some(fr) = next {
            if fr.day == state.day {
                check_frame(&state, fr);
                next = it.next();
            }
        }
    }
    assert!(next.is_none(), "{name}: not all frames were checked");
}

#[test]
fn single_node_necessity() {
    run_single("single_node_necessity");
}

#[test]
fn single_node_common() {
    run_single("single_node_common");
}

#[test]
fn single_node_luxury() {
    run_single("single_node_luxury");
}
