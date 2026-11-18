# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

Milestones closed (plans in `docs/plans/`):
- **M1** `m1-python-prototype.md` — Python prototype of the core: single-region model `MODEL.md`
  §4–5.12, stability autotests §8.1–8.4. Stays the reference oracle (golden fixtures).
- **M2** `m2-rust-port.md` — Rust port of the core, R0–R6.
- **M3** `m3-wasm-web-ui.md` — WASM facade, wasm-pack packaging, offline PWA web UI, R7–R9.

Production chains (§5.13) are deferred to v1.1. **Current milestone: M4** `m4-open-source.md` —
English + i18n + SRD price list, stages R10–R14; R10 (SRD price list), R11 (structured reasons log), R12 (UI i18n EN/RU) and R13 (code in English)
done, next R14 (specs and docs in English). Commits from now on are in
English and carry no Claude attribution (D5). Decisions go to
`DECISIONS.md` (D1 TOML, D2 price list, D3 chains → v1.1, D4 name Dungeonomics + MIT, D5 English + i18n + SRD). Specs remain the source of truth:

- `REQUIREMENTS.md` — product requirements v1 (goal, scope, functional/non-functional reqs, risks, open questions).
- `MODEL.md` — the mathematical model v1 (variables, per-tick update order, formulas, stability analysis, default parameters, acceptance tests). Complements `REQUIREMENTS.md`.

Both are marked *черновик* (draft). Items tagged *(предложено)* are proposals the author has
not yet confirmed — do not treat them as decided. Code references spec sections as `§5.6` etc.
The specs are Russian; so are code comments and commit discussion. When specs disagree with code,
the specs win unless a decision has been recorded.

## Commands

```bash
python3 -m venv .venv && .venv/bin/python -m pip install -e ".[dev]"   # setup
.venv/bin/python -m pytest -q                       # all tests (stability suite §8 lives here)
.venv/bin/python -m pytest tests/test_single_node.py -q   # one test file
.venv/bin/python -m pytest -q -k runaway             # tests matching a name
.venv/bin/python -m jupyter nbconvert --to notebook --execute notebooks/calibration.ipynb  # run notebook
python3 -I scripts/build_srd_gear.py                 # regenerate references/srd_gear.csv (needs references/srd/*.json, see script)
```

## Code map (`ddeco/`, stdlib-only core)

- `goods.py` / `world.py` — data model: good params (§7 price-class defaults), graph of nodes/edges, `GlobalParams`.
- `state.py` — `SimState` (stocks, multipliers, prosperity, transit pipelines, effects, caravans) with `clone()` for rollback.
- `effects.py` — parametric effects: decay weights and the `clamp(base·Π+Σ)` composition (§3); `effective()` resolves a parameter.
- `formulas.py` — pure functions §5.1–5.8 (demand, production, spoilage, price, prosperity).
- `tick.py` — `step_day` orchestrates the fixed §4 order; `run()` loops; also the effective-parameter helpers.
- `trade.py` — arbitrage with in-transit damping + hub delivery (§5.6); conveyor insertion.
- `calibration.py` — equilibrium seed + `warmup()` (§5.14). `rng.py` — counter-based RNG (§5.11). `shocks.py` — background shocks.
- `purchase.py` (§5.7), `caravans.py` (§5.10), `reasons.py` (§5.12), `events.py` (one-shot events), `loader.py`/`persistence.py` (TOML I/O + autosave).

Tests mirror the spec: `tests/test_single_node.py` (§8.1), `test_multi_node.py` (§8.2),
`test_runaway.py` (§8.3), `test_determinism.py` (§8.4). `tests/single_node.py` is a shared helper, not a test.

## What this is

An open-source tool for tabletop RPG (D&D) game masters to run a **living regional economy**.
The GM describes a region once (nodes, roads, goods); thereafter player actions and world events
drive prices, availability, and trade flows automatically — no manual bookkeeping. Inspired by
Mount & Blade: Bannerlord's economy. Target scope v1: one region, 3–4 cities, ~10–20 nodes,
local single-user operation.

## Architecture

- **Core:** currently the Python prototype in `ddeco/` (stdlib-only, UI-independent). It is the
  reference implementation; its "golden runs" will verify the planned Rust port.
- **Rust core (next):** same logic ported to Rust for WASM. Pure Python with explicit, sorted
  iteration was chosen precisely so the port is mechanical and determinism (§9) is preserved.
- **Web UI (later):** Rust core compiled to WASM, static offline PWA; world saved/loaded as a file.
- **Desktop (later):** same UI packaged as an executable (e.g. Tauri) at release time.

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
`MODEL.md` §10; already decided: D1–D5 in `DECISIONS.md`): goods units (pieces/weight/"batches"), caravan materialization rules, and
the stabilization parameters (β, spoilage rate, prosperity ceiling, list of sinks).
