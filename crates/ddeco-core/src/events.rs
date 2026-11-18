//! One-shot events (MODEL.md §3, §5.9). Port of `ddeco/events.py`.
//!
//! Unlike parametric effects, one-shot events change the *state* once
//! and are not part of the parameters: GM stock edits, prosperity hits, caravan
//! robbery. All of them are deterministic and do not use the RNG.

use crate::state::SimState;

/// The GM sets a stock by hand (a player bought out / brought in goods). REQUIREMENTS §5.8.
pub fn set_stock(state: &mut SimState, node: &str, good: &str, amount: f64) {
    state
        .stock
        .insert((node.to_string(), good.to_string()), amount.max(0.0));
}

/// Shift a stock by delta (may be negative), not below zero.
pub fn add_stock(state: &mut SimState, node: &str, good: &str, delta: f64) {
    let key = (node.to_string(), good.to_string());
    let cur = state.stock.get(&key).copied().unwrap_or(0.0);
    state.stock.insert(key, (cur + delta).max(0.0));
}

/// Prosperity hit: π ← π·(1 − ρ) (a raid, §5.8).
pub fn prosperity_hit(state: &mut SimState, node: &str, rho: f64) {
    if let Some(p) = state.prosperity.get_mut(node) {
        *p *= 1.0 - rho;
    }
}

/// Robbery: remove a share ρ of the cargo from the arc's pipeline (§5.9).
///
/// `slot_index = None` — all cargo in transit on the arc; otherwise a specific batch.
/// Returns the total volume looted (the GM decides whether the players get it).
pub fn loot_caravan(state: &mut SimState, edge: &str, rho: f64, slot_index: Option<usize>) -> f64 {
    let slots = match state.pipeline.get_mut(edge) {
        Some(s) if !s.is_empty() => s,
        _ => return 0.0,
    };
    let targets: Vec<usize> = match slot_index {
        Some(i) => {
            if i >= slots.len() {
                return 0.0;
            }
            vec![i]
        }
        None => (0..slots.len()).collect(),
    };
    let mut looted = 0.0;
    for i in targets {
        let goods: Vec<String> = slots[i].keys().cloned().collect();
        for g in goods {
            let take = slots[i][&g] * rho;
            *slots[i].get_mut(&g).unwrap() -= take;
            looted += take;
        }
    }
    looted
}
