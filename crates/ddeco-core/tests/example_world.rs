//! R26: the shipped example world is stable (mirror of `tests/test_example_world.py`).
//! After the warm-up (§5.14) converges, no node × good with demand sits on a corridor
//! edge (m_min / m_max). Exceptions go into `PINNED_OK` with a reason; none are needed.

use ddeco_core::calibration::warmup;
use ddeco_core::formulas::base_demand;
use ddeco_core::loader::world_from_toml;
use ddeco_core::tick::step_day;
use ddeco_core::world::World;

const MAX_DAYS: u32 = 2000;
const TOL: f64 = 1e-4;
const EDGE: f64 = 1e-3;

/// (node, good, reason). Keep empty unless a good genuinely cannot avoid the edge.
const PINNED_OK: &[(&str, &str, &str)] = &[];

fn example_world() -> World {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../worlds/example_region.toml"
    ))
    .expect("example_region.toml");
    world_from_toml(&text).expect("parse")
}

#[test]
fn warmup_converges() {
    let world = example_world();
    let mut st = warmup(&world, MAX_DAYS, TOL);
    assert!(
        st.day < i64::from(MAX_DAYS),
        "warm-up hit max_days without converging"
    );
    let prev = st.multiplier.clone();
    step_day(&world, &mut st, &[]);
    let delta = st
        .multiplier
        .iter()
        .map(|(k, v)| (v - prev[k]).abs())
        .fold(0.0_f64, f64::max);
    assert!(delta < TOL, "delta {delta}");
}

#[test]
fn no_multiplier_pinned_to_corridor_edge() {
    let world = example_world();
    let st = warmup(&world, MAX_DAYS, TOL);
    let mut pinned = Vec::new();
    for (n, node) in &world.nodes {
        for (g, good) in &world.catalog.goods {
            let demand = base_demand(
                good,
                node.population,
                st.prosperity[n],
                world.params.pi_ref,
                node.garrison_of(g),
            );
            let allowed = PINNED_OK.iter().any(|(pn, pg, _)| pn == n && pg == g);
            if demand <= 0.0 || allowed {
                continue;
            }
            let m = st.multiplier[&(n.clone(), g.clone())];
            if m - good.m_min < EDGE || good.m_max - m < EDGE {
                pinned.push((n.clone(), g.clone(), m));
            }
        }
    }
    assert!(pinned.is_empty(), "pinned: {pinned:?}");
}

#[test]
fn every_good_has_a_producer() {
    let world = example_world();
    for g in world.catalog.goods.keys() {
        assert!(world.nodes.values().any(|n| n.pcap(g) > 0.0), "{g}");
    }
}
