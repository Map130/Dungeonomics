//! `ddeco-core` — the simulation core of a "living economy" for tabletop RPGs.
//!
//! A port of the Python prototype (`ddeco/` at the repository root), which stays the reference:
//! golden runs from Python verify this core (see `tests/golden.rs`). References like
//! "§5.6" point to sections of `MODEL.md`.
//!
//! Invariants (see CLAUDE.md): parameters change only through effects, the state
//! evolves on its own; a synchronous tick from the start-of-day snapshot; a fixed traversal
//! order (ordered maps instead of hash maps, §9); prices are multipliers on a base price list;
//! determinism through [`mathf`] and a counter-based RNG.

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
