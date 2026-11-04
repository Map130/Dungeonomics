//! Торговые потоки: арбитраж и поставка в хаб (MODEL.md §5.6). Порт `ddeco/trade.py`.
//!
//! В R3 — заглушка: одноузловые сценарии не имеют дуг, поэтому торговли нет. Полная
//! реализация (арбитраж с учётом товара в пути, поставка в хаб, конвейеры) — в R5.

use std::collections::BTreeMap;

use crate::state::{Key, SimState};
use crate::world::World;

/// Шаг 6: торговая отправка. Заглушка до R5.
pub fn dispatch(
    world: &World,
    state: &mut SimState,
    star: &BTreeMap<Key, f64>,
    produced: &BTreeMap<Key, f64>,
) {
    let _ = (world, state, star, produced);
}
