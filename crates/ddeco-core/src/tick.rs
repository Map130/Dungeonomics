//! Оркестрация одного дня (MODEL.md §4). Порт `ddeco/tick.py`.
//!
//! Порядок шагов фиксирован. Торговля (6) и материализация караванов (9) — полные
//! (§5.6, §5.10). Параметры дуги и спред берутся базовыми, как в Python-эталоне.

use std::collections::BTreeMap;

use crate::effects::{effective, Effect, TargetKind};
use crate::shocks::{self, ShockTemplate};
use crate::state::{Key, SimState};
use crate::world::World;
use crate::{caravans, formulas as f, trade};

// --- Эффективные параметры (база из мира + активные эффекты, §3) --------------

fn eff_pcap(world: &World, effects: &[Effect], day: i64, node: &str, good: &str) -> f64 {
    let base = world.nodes[node].pcap(good);
    let cat = &world.catalog.get(good).category;
    effective(
        base,
        effects,
        "pcap",
        TargetKind::Node,
        node,
        day,
        Some(good),
        Some(cat),
        0.0,
        f64::INFINITY,
    )
}

fn eff_population(world: &World, effects: &[Effect], day: i64, node: &str) -> f64 {
    let base = world.nodes[node].population;
    effective(
        base,
        effects,
        "population",
        TargetKind::Node,
        node,
        day,
        None,
        None,
        0.0,
        f64::INFINITY,
    )
}

fn eff_garrison(world: &World, effects: &[Effect], day: i64, node: &str, good: &str) -> f64 {
    let base = world.nodes[node].garrison_of(good);
    let cat = &world.catalog.get(good).category;
    effective(
        base,
        effects,
        "garrison",
        TargetKind::Node,
        node,
        day,
        Some(good),
        Some(cat),
        0.0,
        f64::INFINITY,
    )
}

// --- Шаги дня -----------------------------------------------------------------

fn drop_expired(state: &mut SimState) {
    let day = state.day;
    state.effects.retain(|e| !e.expired(day));
}

fn arrivals(world: &World, state: &mut SimState) {
    for e in world.edges.keys().cloned().collect::<Vec<_>>() {
        let dst = world.edges[&e].dst.clone();
        let slots = state.pipeline.get_mut(&e);
        let arrived = match slots {
            Some(s) if !s.is_empty() => s.remove(0),
            _ => continue,
        };
        for (g, amt) in arrived {
            *state.stock.entry((dst.clone(), g)).or_insert(0.0) += amt;
        }
    }
}

/// Шаги 3–5: производство, потребление, порча по снимку начала дня (m0).
fn production_consumption_spoilage(
    world: &World,
    state: &mut SimState,
    m0: &BTreeMap<Key, f64>,
    demand: &mut BTreeMap<Key, f64>,
    star: &mut BTreeMap<Key, f64>,
    produced: &mut BTreeMap<Key, f64>,
) {
    let p = &world.params;
    let day = state.day;
    for n in world.nodes.keys().cloned().collect::<Vec<_>>() {
        let pros = state.prosperity[&n];
        let pop = eff_population(world, &state.effects, day, &n);
        for g in world.catalog.goods.keys().cloned().collect::<Vec<_>>() {
            let good = world.catalog.get(&g);
            let key = (n.clone(), g.clone());
            let d = f::base_demand(
                good,
                pop,
                pros,
                p.pi_ref,
                eff_garrison(world, &state.effects, day, &n, &g),
            );
            demand.insert(key.clone(), d);
            star.insert(key.clone(), f::target_stock(good, d));

            let prod = f::production(
                good,
                eff_pcap(world, &state.effects, day, &n, &g),
                m0[&key],
                p.lo_prod,
                p.hi_prod,
            );
            produced.insert(key.clone(), prod);
            let mut s = state.stock[&key] + prod;
            s -= f::consumption(good, d, m0[&key], s);
            s = f::after_spoilage(good, s);
            state.stock.insert(key, s);
        }
    }
}

fn basket_terms(
    world: &World,
    state: &SimState,
    node: &str,
    food: bool,
    star: &BTreeMap<Key, f64>,
) -> Vec<(f64, f64)> {
    let mut terms = Vec::new();
    for g in world.catalog.goods.keys() {
        let good = world.catalog.get(g);
        if good.in_food_basket != food || good.basket_weight <= 0.0 {
            continue;
        }
        let key = (node.to_string(), g.clone());
        let s_star = star[&key];
        if s_star <= 0.0 {
            continue;
        }
        terms.push((good.basket_weight, state.stock[&key] / s_star));
    }
    terms
}

fn prosperity(world: &World, state: &mut SimState, star: &BTreeMap<Key, f64>) {
    let p = &world.params;
    for n in world.nodes.keys().cloned().collect::<Vec<_>>() {
        let food_fill = f::basket_fill(&basket_terms(world, state, &n, true, star));
        let market_fill = f::basket_fill(&basket_terms(world, state, &n, false, star));
        let target = f::prosperity_target(food_fill, market_fill, p.w_food, p.w_market);
        let cur = state.prosperity[&n];
        state
            .prosperity
            .insert(n, f::smooth_prosperity(cur, target, p.mu));
    }
}

fn prices(world: &World, state: &mut SimState, star: &BTreeMap<Key, f64>) {
    let lam = world.params.lam;
    for n in world.nodes.keys().cloned().collect::<Vec<_>>() {
        for g in world.catalog.goods.keys().cloned().collect::<Vec<_>>() {
            let good = world.catalog.get(&g);
            let key = (n.clone(), g.clone());
            let mstar = f::target_multiplier(good, state.stock[&key], star[&key]);
            let cur = state.multiplier[&key];
            state
                .multiplier
                .insert(key, f::smooth_multiplier(cur, mstar, lam));
        }
    }
}

/// Продвинуть состояние на один день. `shocks` — шаблоны фоновых шоков (§5.11);
/// пустой срез = генератор выключен. Снимок для отката делает вызывающий код.
pub fn step_day(world: &World, state: &mut SimState, shocks: &[ShockTemplate]) {
    if !shocks.is_empty() {
        let new = shocks::generate(world, state, shocks); // 0
        state.effects.extend(new);
    }
    drop_expired(state); // 1
    arrivals(world, state); // 2
    let m0 = state.multiplier.clone(); // снимок цен на начало дня
    let mut demand = BTreeMap::new();
    let mut star = BTreeMap::new();
    let mut produced = BTreeMap::new();
    production_consumption_spoilage(world, state, &m0, &mut demand, &mut star, &mut produced); // 3–5
    trade::dispatch(world, state, &star, &produced); // 6
    prosperity(world, state, &star); // 7
    prices(world, state, &star); // 8
    caravans::materialize(world, state); // 9
    state.day += 1;
}

/// Прокрутить N дней (REQUIREMENTS §5.7).
pub fn run(world: &World, state: &mut SimState, days: u32, shocks: &[ShockTemplate]) {
    for _ in 0..days {
        step_day(world, state, shocks);
    }
}
