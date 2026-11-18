//! Named caravans (MODEL.md §5.10). Port of `ddeco/caravans.py`.
//!
//! A caravan is the face of a flow already under way, not a source of new goods. When
//! the accumulated value of the flow along an arc exceeds the threshold θ_car, the system
//! "materialises" the large flow into a named caravan: the cargo is the batches currently in
//! the arc's pipeline, the guard strength is random and depends on road danger (k_e).
//! Looting applies §5.9 to that arc's pipeline.

use std::collections::BTreeMap;

use crate::rng;
use crate::state::SimState;
use crate::trade::eff_difficulty;
use crate::world::World;

/// A materialised caravan — the face of a flow already under way along an arc.
#[derive(Debug, Clone, PartialEq)]
pub struct Caravan {
    pub edge: String,
    pub day_departed: i64,
    pub day_arrival: i64,
    pub value: f64,
    pub guard: f64,
    pub cargo: Vec<(String, f64)>, // sorted snapshot of the cargo in transit
}

/// Snapshot of the cargo in transit on an arc: total volume per good, ordered by id (§9).
fn cargo_snapshot(state: &SimState, edge: &str) -> Vec<(String, f64)> {
    let mut totals: BTreeMap<String, f64> = BTreeMap::new();
    if let Some(slots) = state.pipeline.get(edge) {
        for slot in slots {
            for (g, &amt) in slot {
                *totals.entry(g.clone()).or_insert(0.0) += amt;
            }
        }
    }
    totals.into_iter().collect()
}

/// Step 9: spawn caravans on arcs where the accumulated flow is ≥ θ_car.
///
/// Deterministic: the guard strength comes from the counter-based RNG (seed, day, arc).
pub fn materialize(world: &World, state: &mut SimState) -> Vec<Caravan> {
    let theta = world.params.caravan_threshold;
    let mut made = Vec::new();
    for e in world.edge_ids().cloned().collect::<Vec<_>>() {
        let accum = state.caravan_accum.get(&e).copied().unwrap_or(0.0);
        if accum < theta {
            continue;
        }
        let edge = &world.edges[&e];
        // Road danger k_e scales the guard strength; noise from the counter-based RNG.
        let k_eff = eff_difficulty(world, &state.effects, state.day, &e);
        let guard = k_eff * (0.5 + rng::uniform(world.seed, state.day, &[&e, "caravan_guard"]));
        let tau = edge.delay_days_with(k_eff) as i64;
        let caravan = Caravan {
            edge: e.clone(),
            day_departed: state.day,
            day_arrival: state.day + tau,
            value: accum,
            guard: round3(guard),
            cargo: cargo_snapshot(state, &e),
        };
        made.push(caravan.clone());
        state.caravans.push(caravan);
        state.caravan_accum.insert(e, accum - theta); // carry the remainder over
    }
    made
}

/// Round to 3 decimals (mirrors the Python reference's `round(x, 3)` for the guard field).
fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}
