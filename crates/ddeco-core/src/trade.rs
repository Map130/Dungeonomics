//! Торговые потоки: арбитраж и поставка в хаб (MODEL.md §5.6). Порт `ddeco/trade.py`.
//!
//! Фоновая торговля без агентов: если ожидаемая разница цен между узлами превышает
//! стоимость пути, часть запаса уходит в конвейер дуги с задержкой. Решение о
//! перевозке принимается по *ожидаемому* множителю с учётом уже едущего товара Q
//! (иначе торговцы слали бы груз каждый день, пока едет первая партия → перелив).
//!
//! Всё считается по синхронному снимку запасов на начало шага 6: сначала
//! обязательные поставки деревень в хаб, затем арбитраж по остатку. Порядок обхода
//! дуг/товаров фиксирован (§9), поэтому результат воспроизводим. Параметры дуги
//! (travel_time, difficulty, capacity) берутся базовыми — как в Python-эталоне.

use std::collections::BTreeMap;

use crate::mathf::{self, clamp};
use crate::state::{Key, SimState};
use crate::world::World;

/// Положить груз в конвейер дуги с задержкой τ (прибудет через τ дней).
///
/// Индекс τ−1: шаг прибытия (§4.2) снимает индекс 0 в начале каждого дня, поэтому
/// груз на индексе τ−1 будет снят ровно через τ дней.
pub fn add_to_pipeline(state: &mut SimState, edge_id: &str, tau: usize, good: &str, amount: f64) {
    let slots = state.pipeline.entry(edge_id.to_string()).or_default();
    let idx = tau - 1;
    while slots.len() <= idx {
        slots.push(BTreeMap::new());
    }
    *slots[idx].entry(good.to_string()).or_insert(0.0) += amount;
}

/// Обязательная поставка деревни в город-хаб: доля η_v производства независимо от
/// разницы цен (§5.6). Арбитраж дальше работает с остатком.
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
            None => continue, // нет дороги деревня→хаб — поставку пропускаем
        };
        let tau = edge.delay_days();
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

/// m̂ = clamp(((s + Q)/s*)^(−α), m_min, m_max) — с учётом товара в пути в узел.
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

/// Перелив запасов по дугам, где выгода превышает стоимость пути (§5.6).
fn arbitrage(world: &World, state: &mut SimState, star: &BTreeMap<Key, f64>) {
    let p = &world.params;

    // Ожидаемые множители и доступное к вывозу по снимку начала шага.
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

    // Желаемый поток по каждой дуге и товару (u_eg).
    let mut u: BTreeMap<(String, String), f64> = BTreeMap::new();
    for e in world.edge_ids() {
        let edge = &world.edges[e];
        let gamma = p.c_gamma * edge.travel_time * edge.difficulty;
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

    // ψ_ig: не вывезти больше, чем доступно, суммируя по исходящим дугам (на товар).
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

    // φ_e: ограничение по ёмкости дороги (суммируя объём по товарам).
    for e in world.edge_ids().cloned().collect::<Vec<_>>() {
        let (src, capacity, tau) = {
            let edge = &world.edges[&e];
            (edge.src.clone(), edge.capacity, edge.delay_days())
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

/// Шаг 6 целиком: сначала обязательные поставки в хаб, затем арбитраж.
pub fn dispatch(
    world: &World,
    state: &mut SimState,
    star: &BTreeMap<Key, f64>,
    produced: &BTreeMap<Key, f64>,
) {
    hub_deliveries(world, state, produced);
    arbitrage(world, state, star);
}
