//! Автосохранение состояния сессии (REQUIREMENTS §5.9). Порт `ddeco/persistence.py`.
//!
//! Состояние сбрасывается на диск периодически (по умолчанию раз в минуту wall-clock),
//! а НЕ на каждый тик: тиков за вечер игры тысячи, диску достаточно изредка — защита
//! от падения. Откат/повтор внутри сессии идёт по in-memory снимкам (`SimState::clone`),
//! файл в этот путь не вовлечён.
//!
//! Ядро целится в WASM и не трогает ФС/системные часы: `maybe_save` лишь решает, пора
//! ли сохранять, по переданному времени, и, если да, отдаёт сериализованное состояние.
//! Атомарную запись (tmp + rename) и источник времени обеспечивает вызывающий — тот же
//! принцип, что и с чтением мира ([`crate::loader::world_from_toml`]).

use crate::loader::state_to_toml;
use crate::state::SimState;

/// Троттлинг автосейва: отдаёт состояние не чаще, чем раз в `interval` секунд.
pub struct Autosaver {
    interval: f64,
    last: f64,
}

impl Autosaver {
    /// `interval` — минимальная пауза между записями (сек), `now` — текущее время.
    pub fn new(interval: f64, now: f64) -> Self {
        Self {
            interval,
            last: now,
        }
    }

    /// TOML состояния, если прошёл интервал (или `force`); иначе `None`. При выдаче
    /// точка отсчёта сдвигается на `now`, чтобы следующая запись ждала новый интервал.
    pub fn maybe_save(&mut self, state: &SimState, now: f64, force: bool) -> Option<String> {
        if !force && now - self.last < self.interval {
            return None;
        }
        self.last = now;
        Some(state_to_toml(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttles_until_interval_passes() {
        let st = SimState::default();
        let mut saver = Autosaver::new(60.0, 0.0);

        assert!(
            saver.maybe_save(&st, 30.0, false).is_none(),
            "рано — не пишем"
        );
        assert!(
            saver.maybe_save(&st, 60.0, false).is_some(),
            "интервал прошёл — пишем"
        );
        assert!(
            saver.maybe_save(&st, 90.0, false).is_none(),
            "отсчёт сдвинулся на последнюю запись"
        );
        assert!(
            saver.maybe_save(&st, 120.0, false).is_some(),
            "снова прошёл интервал"
        );
    }

    #[test]
    fn force_saves_regardless() {
        let st = SimState::default();
        let mut saver = Autosaver::new(60.0, 0.0);
        assert!(
            saver.maybe_save(&st, 1.0, true).is_some(),
            "force игнорирует интервал"
        );
    }
}
