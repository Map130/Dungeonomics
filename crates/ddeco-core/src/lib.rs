//! `ddeco-core` — ядро симуляции «живой экономики» для настольных RPG.
//!
//! Порт Python-прототипа (`ddeco/` в корне репозитория), который остаётся эталоном:
//! golden-прогоны из Python проверяют это ядро (см. `tests/golden.rs`). Ссылки вида
//! «§5.6» указывают на разделы `MODEL.md`.
//!
//! Инварианты (см. CLAUDE.md): параметры меняются только эффектами, состояние
//! эволюционирует само; синхронный тик из снимка начала дня; фиксированный порядок
//! обхода (индексы вместо хеш-мап, §9); цены — множители к базовому прайс-листу;
//! детерминизм через [`mathf`] и счётчиковый RNG.

pub mod calibration;
pub mod caravans;
pub mod effects;
pub mod events;
pub mod formulas;
pub mod goods;
pub mod loader;
pub mod mathf;
pub mod persistence;
pub mod purchase;
pub mod reasons;
pub mod rng;
pub mod shocks;
pub mod state;
pub mod tick;
pub mod trade;
pub mod world;
