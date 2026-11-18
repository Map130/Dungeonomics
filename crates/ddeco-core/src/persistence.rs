//! Autosaving the session state (REQUIREMENTS §5.9). Port of `ddeco/persistence.py`.
//!
//! The state is flushed to disk periodically (by default once a minute of wall-clock time),
//! NOT on every tick: an evening of play has thousands of ticks, occasional flushes are enough
//! to survive a crash. Undo/replay within a session uses in-memory snapshots (`SimState::clone`),
//! the file is not involved in that path.
//!
//! The core targets WASM and does not touch the FS/system clock: `maybe_save` only decides
//! whether it is time to save, given the passed-in time, and if so returns the serialised state.
//! The atomic write (tmp + rename) and the time source are up to the caller — the same
//! principle as reading the world ([`crate::loader::world_from_toml`]).

use crate::loader::state_to_toml;
use crate::state::SimState;

/// Autosave throttling: returns the state no more often than once every `interval` seconds.
pub struct Autosaver {
    interval: f64,
    last: f64,
}

impl Autosaver {
    /// `interval` — minimum pause between writes (s), `now` — the current time.
    pub fn new(interval: f64, now: f64) -> Self {
        Self {
            interval,
            last: now,
        }
    }

    /// State TOML if the interval has passed (or `force`); otherwise `None`. When returned,
    /// the reference point moves to `now`, so the next write waits a new interval.
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
            "too early — no write"
        );
        assert!(
            saver.maybe_save(&st, 60.0, false).is_some(),
            "interval passed — write"
        );
        assert!(
            saver.maybe_save(&st, 90.0, false).is_none(),
            "reference moved to the last write"
        );
        assert!(
            saver.maybe_save(&st, 120.0, false).is_some(),
            "interval passed again"
        );
    }

    #[test]
    fn force_saves_regardless() {
        let st = SimState::default();
        let mut saver = Autosaver::new(60.0, 0.0);
        assert!(
            saver.maybe_save(&st, 1.0, true).is_some(),
            "force ignores the interval"
        );
    }
}
