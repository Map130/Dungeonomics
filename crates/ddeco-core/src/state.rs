//! Состояние симуляции (MODEL.md §2). Порт `ddeco/state.py`.
//!
//! Эволюционирует само; эффекты меняют только параметры мира. Всё, что нужно для
//! отката/повтора дня (§7, §8.4), лежит здесь — снимок через `Clone`.
//!
//! Ключи `(node, good)` в `BTreeMap` сортируются лексикографически по кортежу, что
//! совпадает с `sorted(...)` в Python и фиксирует порядок суммирования (§9).

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
    /// Q_jg: суммарный товар `good` в пути в узел `node` (§5.6). Фиксированный
    /// порядок обхода дуг и слотов (§9).
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

/// Пустое состояние: нулевые запасы, m = 1, π = π0, пустые конвейеры.
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
