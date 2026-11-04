//! Начальное состояние и прогрев мира (MODEL.md §5.14). Порт `ddeco/calibration.py`.

use crate::formulas as f;
use crate::mathf::{self, clamp};
use crate::state::{blank_state, SimState};
use crate::tick::step_day;
use crate::world::World;

/// Решение m^(β+ε) = D/Pcap. При Pcap = 0 цена уходит вверх (нет производства).
pub fn equilibrium_multiplier(beta: f64, eps: f64, demand: f64, pcap: f64) -> f64 {
    if pcap <= 0.0 {
        return f64::INFINITY;
    }
    mathf::powf(demand / pcap, 1.0 / (beta + eps))
}

/// Стартовое состояние из авто-калибровки равновесия без торговли (§5.14).
pub fn seed_state(world: &World) -> SimState {
    let mut st = blank_state(world);
    let pi_ref = world.params.pi_ref;
    for n in world.node_ids() {
        let node = &world.nodes[n];
        let pros = node.prosperity0;
        for g in world.catalog.ids() {
            let good = world.catalog.get(g);
            let demand = f::base_demand(good, node.population, pros, pi_ref, node.garrison_of(g));
            let star = f::target_stock(good, demand);
            let m_eq = clamp(
                equilibrium_multiplier(good.beta, good.epsilon, demand, node.pcap(g)),
                good.m_min,
                good.m_max,
            );
            // r из m*: m = r^(-α) ⇒ r = m^(-1/α); s = r·s*
            let r = if good.alpha > 0.0 {
                mathf::powf(m_eq, -1.0 / good.alpha)
            } else {
                1.0
            };
            let key = (n.clone(), g.clone());
            st.multiplier.insert(key.clone(), m_eq);
            st.stock.insert(key, r * star);
        }
    }
    st
}

/// Прогреть мир: прогон без событий до сходимости множителей (§5.14).
pub fn warmup(world: &World, max_days: u32, tol: f64) -> SimState {
    let mut st = seed_state(world);
    for _ in 0..max_days {
        let prev = st.multiplier.clone();
        step_day(world, &mut st, &[]);
        let delta = st
            .multiplier
            .iter()
            .map(|(k, v)| (v - prev[k]).abs())
            .fold(0.0_f64, f64::max);
        if delta < tol {
            break;
        }
    }
    st
}
