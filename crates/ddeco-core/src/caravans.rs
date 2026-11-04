//! Именные караваны (MODEL.md §5.10). В R3 — тип данных и заглушка материализации;
//! полная версия (с RNG-охраной) подключается в R5. Порт `ddeco/caravans.py`.

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
    pub source: String,
}

/// Шаг 9: материализация караванов. Заглушка до R5.
pub fn materialize(world: &World, state: &mut SimState) {
    let _ = (world, state);
}
