# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

This is a **design-stage project**: there is no code, build system, or tests yet. The
repository currently holds two Russian-language specs that define what gets built:

- `REQUIREMENTS.md` — product requirements v1 (goal, scope, functional/non-functional reqs, risks, open questions).
- `MODEL.md` — the mathematical model v1 (variables, per-tick update order, formulas, stability analysis, default parameters, acceptance tests). Complements `REQUIREMENTS.md`.

Both are marked *черновик* (draft). Items tagged *(предложено)* are proposals the author has
not yet confirmed — do not treat them as decided. When these docs disagree with written code,
the docs are the source of truth until code exists.

## What this is

An open-source tool for tabletop RPG (D&D) game masters to run a **living regional economy**.
The GM describes a region once (nodes, roads, goods); thereafter player actions and world events
drive prices, availability, and trade flows automatically — no manual bookkeeping. Inspired by
Mount & Blade: Bannerlord's economy. Target scope v1: one region, 3–4 cities, ~10–20 nodes,
local single-user operation.

## Intended architecture (not yet built)

- **Core:** a Rust library — simulation, effects, and the "reasons log". UI-independent.
- **Web UI:** core compiled to WASM, static offline PWA; world saved/loaded as a single file.
- **Desktop:** same UI packaged as an executable (e.g. Tauri) at release time.
- **Math prototype:** optionally Python first, then ported to Rust. A reference prototype in
  another language is expected to produce "golden runs" used to verify the main implementation.

## Core model concepts (read before touching simulation logic)

These invariants are the heart of the design. Violating them breaks the whole approach:

- **Parameters vs. state — keep strictly separated.** *Parameters* (production capacity,
  population, road difficulty, edge capacity) are changed only by effects. *State* (stocks,
  price multipliers, goods-in-transit, prosperity) evolves on its own. Events never set state
  directly except for explicit one-shot events (caravan robbery, GM stock edits, prosperity hits).
- **Prices are multipliers on a base price list**, never absolute currency. Simulation moves only
  the multiplier `m`, clamped to a corridor. A node's price depends only on that node's state;
  nodes couple to each other only through trade flows (arbitrage), not a global equilibrium.
- **Discrete daily tick, computed synchronously** from the start-of-day snapshot, so node/edge
  traversal order never affects the result. The fixed per-day step order is in `MODEL.md` §4
  (events → effective params → arrivals → production → consumption → spoilage → trade dispatch →
  prosperity → price → caravan materialization → reasons log). Preserve this order.
- **Effects compose as** `clamp(base · Π(1 + (mult−1)·w) + Σ(add·w), min, max)` where `w` is a
  decay weight in [0,1]. Removing one effect must not disturb others; a decaying effect
  interpolates its multiplier back to 1 so expiry causes no jump.
- **Explainability (reasons log) is a first-class feature**, prioritized over numeric precision.
  Every price/availability change must be traceable to a cause chain back to active effects.
- **Determinism is required.** Same seed + same inputs ⇒ reproducible, including rollback/replay
  of a day. The RNG must be **counter-based** — values derived from `(seed, day, target_id,
  template_id)`, not from a call sequence (see `MODEL.md` §5.11).
- **Stability without external shocks** is a design goal and an acceptance criterion: a world with
  no events must converge to a steady state (prices not pinned to corridor edges, stocks not
  growing unboundedly). The main closing loop is supply reacting to price (`MODEL.md` §5.2),
  plus spoilage, sinks, and a prosperity ceiling. See the stability analysis in `MODEL.md` §6.

## Reproducibility constraints for any implementation

From `MODEL.md` §9 — these shape how the Rust/WASM code must be written:

- Fixed node/edge traversal order (sort by id); floating-point reductions only with a fixed
  summation order.
- `pow`/`exp`/`ln` differ in last bits across platforms and WASM. For cross-build reproducibility
  either use a single shared math implementation (e.g. `libm`) or record golden runs "with
  tolerance" instead of requiring bit-exact matches.

## Acceptance tests to implement alongside the first prototype

`MODEL.md` §8 defines the stability test suite that doubles as the correctness bar: single-node
convergence per good category, multi-node (3–4 cities, ~15 nodes) convergence with no events,
a runaway-growth test (central city overproduction must hit a ceiling, not grow forever), and
determinism (two runs with one seed match; rollback-and-replay reproduces).

## Open decisions not yet made

Before implementing areas they touch, check these are still open (see `REQUIREMENTS.md` §10 and
`MODEL.md` §10): world file format (YAML/JSON/TOML), whether production chains ship in v1 vs v1.1,
goods units (pieces/weight/"batches"), caravan materialization rules, base price-list source, and
the stabilization parameters (β, spoilage rate, prosperity ceiling, list of sinks).
