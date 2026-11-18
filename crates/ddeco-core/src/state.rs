//! Simulation state (MODEL.md §2). Port of `ddeco/state.py`.
//!
//! It evolves on its own; effects change only the world's parameters. Everything needed to
//! undo/replay a day (§7, §8.4) lives here — a snapshot via `Clone`.
//!
//! `(node, good)` keys in a `BTreeMap` sort lexicographically by tuple, which
//! matches `sorted(...)` in Python and fixes the summation order (§9).

use std::collections::BTreeMap;

use crate::caravans::Caravan;
use crate::effects::Effect;
use crate::world::World;

pub type Key = (String, String); // (node, good)

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SimState {
    pub day: i64,
    pub stock: BTreeMap<Key, f64>,                              // s_ig
    pub multiplier: BTreeMap<Key, f64>,                         // m_ig
    pub prosperity: BTreeMap<String, f64>,                      // π_i
    pub pipeline: BTreeMap<String, Vec<BTreeMap<String, f64>>>, // q[e][d]
    pub effects: Vec<Effect>,
    pub caravan_accum: BTreeMap<String, f64>, // §5.10
    pub caravans: Vec<Caravan>,
}

impl SimState {
    /// Q_jg: total `good` in transit to node `node` (§5.6). Fixed
    /// traversal order of arcs and slots (§9).
    pub fn in_transit_to(&self, world: &World, node: &str, good: &str) -> f64 {
        let mut total = 0.0;
        for e in world.edge_ids() {
            if world.edges[e].dst != node {
                continue;
            }
            if let Some(slots) = self.pipeline.get(e) {
                for slot in slots {
                    total += slot.get(good).copied().unwrap_or(0.0);
                }
            }
        }
        total
    }
}

/// Empty state: zero stocks, m = 1, π = π0, empty pipelines.
pub fn blank_state(world: &World) -> SimState {
    let mut st = SimState::default();
    for n in world.node_ids() {
        st.prosperity.insert(n.clone(), world.nodes[n].prosperity0);
        for g in world.catalog.ids() {
            st.stock.insert((n.clone(), g.clone()), 0.0);
            st.multiplier.insert((n.clone(), g.clone()), 1.0);
        }
    }
    for e in world.edge_ids() {
        st.pipeline.insert(e.clone(), Vec::new());
        st.caravan_accum.insert(e.clone(), 0.0);
    }
    st
}
