//! Trade flows: arbitrage and hub delivery (MODEL.md §5.6). Port of `ddeco/trade.py`.
//!
//! Background trade without agents: if the expected price difference between nodes exceeds
//! the travel cost, part of the stock goes into the arc's pipeline with a delay. The decision to
//! ship is taken on the *expected* multiplier, accounting for goods already in transit Q
//! (otherwise traders would ship every day while the first batch is on the road → overflow).
//!
//! Everything is computed from a synchronous snapshot of stocks at the start of step 6: first
//! the mandatory village deliveries to the hub, then arbitrage on the remainder. The order of
//! arcs/goods is fixed (§9), so the result is reproducible. Arc parameters k_e and cap_e are the
//! effective values (after effects) on the day of the step (§2), as in the Python reference.

use std::collections::BTreeMap;

use crate::effects::{effective, Effect, TargetKind};
use crate::mathf::{self, clamp};
use crate::state::{Key, SimState};
use crate::world::World;

/// k_e after effects, on the given day (§2).
pub fn eff_difficulty(world: &World, effects: &[Effect], day: i64, edge: &str) -> f64 {
    let base = world.edges[edge].difficulty;
    effective(
        base,
        effects,
        "difficulty",
        TargetKind::Edge,
        edge,
        day,
        None,
        None,
        0.0,
        f64::INFINITY,
    )
}

/// cap_e after effects, on the given day (§2).
pub fn eff_capacity(world: &World, effects: &[Effect], day: i64, edge: &str) -> f64 {
    let base = world.edges[edge].capacity;
    effective(
        base,
        effects,
        "capacity",
        TargetKind::Edge,
        edge,
        day,
        None,
        None,
        0.0,
        f64::INFINITY,
    )
}

/// Put cargo into the arc's pipeline with delay τ (arrives in τ days).
///
/// Index τ−1: the arrival step (§4.2) takes index 0 at the start of each day, so
/// cargo at index τ−1 is taken exactly τ days later.
pub fn add_to_pipeline(state: &mut SimState, edge_id: &str, tau: usize, good: &str, amount: f64) {
    let slots = state.pipeline.entry(edge_id.to_string()).or_default();
    let idx = tau - 1;
    while slots.len() <= idx {
        slots.push(BTreeMap::new());
    }
    *slots[idx].entry(good.to_string()).or_insert(0.0) += amount;
}

/// Mandatory delivery from a village to its hub city: a share η_v of production regardless of
/// the price difference (§5.6). Arbitrage then works on the remainder.
fn hub_deliveries(world: &World, state: &mut SimState, produced: &BTreeMap<Key, f64>) {
    for n in world.node_ids().cloned().collect::<Vec<_>>() {
        let node = &world.nodes[&n];
        let hub = match &node.hub {
            Some(h) => h.clone(),
            None => continue,
        };
        let edge_id = format!("{n}__{hub}");
        let edge = match world.edges.get(&edge_id) {
            Some(e) => e,
            None => continue, // no village→hub road — skip the delivery
        };
        let tau = edge.delay_days_with(eff_difficulty(world, &state.effects, state.day, &edge_id));
        let hub_share = node.hub_share;
        for g in world.catalog.ids().cloned().collect::<Vec<_>>() {
            let key = (n.clone(), g.clone());
            let p = produced.get(&key).copied().unwrap_or(0.0);
            if p <= 0.0 {
                continue;
            }
            let amount = (hub_share * p).min(state.stock[&key]);
            if amount <= 0.0 {
                continue;
            }
            *state.stock.get_mut(&key).unwrap() -= amount;
            add_to_pipeline(state, &edge_id, tau, &g, amount);
            let base_price = world.catalog.get(&g).base_price;
            *state.caravan_accum.entry(edge_id.clone()).or_insert(0.0) += base_price * amount;
        }
    }
}

/// m̂ = clamp(((s + Q)/s*)^(−α), m_min, m_max) — accounting for goods in transit to the node.
fn expected_multiplier(world: &World, state: &SimState, node: &str, good: &str, star: f64) -> f64 {
    let good_p = world.catalog.get(good);
    if star <= 0.0 {
        return good_p.m_min;
    }
    let q = state.in_transit_to(world, node, good);
    let r = (state.stock[&(node.to_string(), good.to_string())] + q) / star;
    if r <= 0.0 {
        return good_p.m_max;
    }
    clamp(mathf::powf(r, -good_p.alpha), good_p.m_min, good_p.m_max)
}

/// Move stock along arcs where the gain exceeds the travel cost (§5.6).
fn arbitrage(world: &World, state: &mut SimState, star: &BTreeMap<Key, f64>) {
    let p = &world.params;

    // Expected multipliers and the amount available for export from the start-of-step snapshot.
    let mut mhat: BTreeMap<Key, f64> = BTreeMap::new();
    let mut avail: BTreeMap<Key, f64> = BTreeMap::new();
    for n in world.node_ids() {
        for g in world.catalog.ids() {
            let key = (n.clone(), g.clone());
            let s_star = star[&key];
            mhat.insert(key.clone(), expected_multiplier(world, state, n, g, s_star));
            avail.insert(
                key.clone(),
                (state.stock[&key] - p.rho_reserve * s_star).max(0.0),
            );
        }
    }

    // Desired flow along each arc and good (u_eg).
    let mut u: BTreeMap<(String, String), f64> = BTreeMap::new();
    for e in world.edge_ids() {
        let edge = &world.edges[e];
        let k_eff = eff_difficulty(world, &state.effects, state.day, e);
        let gamma = p.c_gamma * edge.travel_time * k_eff;
        for g in world.catalog.ids() {
            let good_p = world.catalog.get(g);
            let dst = (edge.dst.clone(), g.clone());
            let src = (edge.src.clone(), g.clone());
            let delta_over_p0 = mhat[&dst] - mhat[&src] - gamma * good_p.weight;
            if delta_over_p0 <= 0.0 {
                continue;
            }
            let flow = p.kappa_trade * clamp(delta_over_p0, 0.0, 1.0) * avail[&src];
            if flow > 0.0 {
                u.insert((e.clone(), g.clone()), flow);
            }
        }
    }

    // ψ_ig: do not ship out more than is available, summing over outgoing arcs (per good).
    let mut psi: BTreeMap<Key, f64> = BTreeMap::new();
    for n in world.node_ids() {
        for g in world.catalog.ids() {
            let mut demand_sum = 0.0;
            for e in world.outgoing(n) {
                demand_sum += u.get(&(e.clone(), g.clone())).copied().unwrap_or(0.0);
            }
            let a = avail[&(n.clone(), g.clone())];
            let value = if demand_sum > 0.0 {
                (a / demand_sum).min(1.0)
            } else {
                1.0
            };
            psi.insert((n.clone(), g.clone()), value);
        }
    }

    // φ_e: road capacity limit (summing volume over goods).
    for e in world.edge_ids().cloned().collect::<Vec<_>>() {
        let (src, capacity, tau) = {
            let edge = &world.edges[&e];
            (
                edge.src.clone(),
                eff_capacity(world, &state.effects, state.day, &e),
                edge.delay_days_with(eff_difficulty(world, &state.effects, state.day, &e)),
            )
        };
        let mut vol_sum = 0.0;
        for g in world.catalog.ids() {
            let good_p = world.catalog.get(g);
            let ueg = u.get(&(e.clone(), g.clone())).copied().unwrap_or(0.0);
            vol_sum += good_p.volume * psi[&(src.clone(), g.clone())] * ueg;
        }
        let phi = if vol_sum > 0.0 {
            (capacity / vol_sum).min(1.0)
        } else {
            1.0
        };
        for g in world.catalog.ids().cloned().collect::<Vec<_>>() {
            let ueg = u.get(&(e.clone(), g.clone())).copied().unwrap_or(0.0);
            if ueg <= 0.0 {
                continue;
            }
            let f = phi * psi[&(src.clone(), g.clone())] * ueg;
            if f <= 0.0 {
                continue;
            }
            *state.stock.get_mut(&(src.clone(), g.clone())).unwrap() -= f;
            add_to_pipeline(state, &e, tau, &g, f);
            let base_price = world.catalog.get(&g).base_price;
            *state.caravan_accum.entry(e.clone()).or_insert(0.0) += base_price * f;
        }
    }
}

/// The whole step 6: mandatory hub deliveries first, then arbitrage.
pub fn dispatch(
    world: &World,
    state: &mut SimState,
    star: &BTreeMap<Key, f64>,
    produced: &BTreeMap<Key, f64>,
) {
    hub_deliveries(world, state, produced);
    arbitrage(world, state, star);
}
