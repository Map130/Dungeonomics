//! Именные караваны (MODEL.md §5.10). Порт `ddeco/caravans.py`.
//!
//! Караван — это лицо уже идущего потока, а не источник новых товаров. Когда
//! накопленная стоимость потока по дуге превышает порог θ_car, система
//! «материализует» крупный поток в именной караван: груз — партии, лежащие сейчас в
//! конвейере дуги, сила охраны случайна и зависит от опасности дороги (k_e).
//! Ограбление применяет §5.9 к конвейеру этой дуги.

use std::collections::BTreeMap;

use crate::rng;
use crate::state::SimState;
use crate::world::World;

/// Материализованный караван — лицо уже идущего потока по дуге.
#[derive(Debug, Clone, PartialEq)]
pub struct Caravan {
    pub edge: String,
    pub day_departed: i64,
    pub day_arrival: i64,
    pub value: f64,
    pub guard: f64,
    pub cargo: Vec<(String, f64)>, // отсортированный снимок груза в пути
}

/// Снимок груза в пути по дуге: суммарный объём каждого товара, порядок по id (§9).
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

/// Шаг 9: породить караваны по дугам, где накоплен поток ≥ θ_car.
///
/// Детерминированно: сила охраны берётся из счётчикового RNG (seed, день, дуга).
pub fn materialize(world: &World, state: &mut SimState) -> Vec<Caravan> {
    let theta = world.params.caravan_threshold;
    let mut made = Vec::new();
    for e in world.edge_ids().cloned().collect::<Vec<_>>() {
        let accum = state.caravan_accum.get(&e).copied().unwrap_or(0.0);
        if accum < theta {
            continue;
        }
        let edge = &world.edges[&e];
        // Опасность дороги k_e масштабирует силу охраны; шум из счётчикового RNG.
        let guard =
            edge.difficulty * (0.5 + rng::uniform(world.seed, state.day, &[&e, "caravan_guard"]));
        let tau = edge.delay_days() as i64;
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
        state.caravan_accum.insert(e, accum - theta); // остаток переносим
    }
    made
}

/// Округление до 3 знаков (повторяет `round(x, 3)` Python-эталона для поля guard).
fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}
