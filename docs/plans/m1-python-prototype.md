> **Status:** closed — Python prototype of the core (Milestone 1).

# Plan: Milestone 1 — Python prototype of the "living economy" core for tabletop RPGs

## Context

The project is at the design stage: the repository holds only two draft specs in Russian —
`REQUIREMENTS.md` (product requirements v1) and `MODEL.md` (mathematical model v1).
There is no code, build or tests. Implementation needs to start.

The tool's goal: a D&D GM describes a region once (nodes, roads, goods), and from then on player
actions and events move prices, availability and trade flows on their own — no manual
bookkeeping. Inspired by the Bannerlord economy.

**Decisions made for this plan (by the user):**
- **First step language — a Python prototype.** First we verify the maths and the stability loops
  in Python, then (a separate future milestone) port to Rust → WASM → web UI. The Python prototype
  stays the reference for golden-run checks of the Rust implementation (`MODEL.md` §9).
- **Milestone 1 scope — a full single-region world** (3–4 cities, ~15 nodes): trade flows,
  prosperity, caravans, the reasons log — everything from `MODEL.md` §4–5.12. **No UI** (the UI is a separate milestone).
- **Storage format — TOML with periodic autosave** (roughly once a minute of wall-clock time,
  **not on every tick**). The source of truth is the in-memory model; the GM edits the world through
  UI forms (in a future milestone). TOML is readable → fits both community reference worlds and a
  save format. The world description and the state are stored separately (§5.9): `world.toml`
  (description, editable and shareable) + an autosaved `state.toml` (session state snapshot).
- **Undo/replay (§7, §8.4) — on in-memory state snapshots** (`state.clone()`) within a session;
  the file is not involved in that path. So float bit-exactness is needed only in memory — losing a
  few bits when serialising to TOML on autosave is acceptable and does not break the determinism test.
- **Production chains (§5.13) — deferred to v1.1.** In M1 goods are independent; the code should
  leave an extension point, but not implement them.

M1 outcome: a Python package that simulates a region per the model, passes the stability autotests
`MODEL.md` §8.1–8.4, with an example world of 3 cities and a warm-up to equilibrium.

## Architectural invariants (do not break — they are the essence of the design)

Taken from `MODEL.md`; any implementation must keep them:

1. **Parameters vs state are strictly separate.** Parameters (Pcap, N, k_e, cap_e) are changed only
   by effects; the state (s, m, q, π) evolves on its own. The state is changed directly only by
   one-shot events (caravan robbery, GM stock edits, a hit on π).
2. **Synchronous tick** from the start-of-day snapshot → the node/arc traversal order does not affect
   the result. The order of steps within a day is fixed (`MODEL.md` §4) and must not change.
3. **Determinism.** One seed + inputs ⇒ reproducible, including undo/replay of a day. The RNG is
   **counter-based**: a value from `(seed, day, target_id, template_id)`, not from a call sequence
   (§5.11). A fixed float summation order (sorted by id, §9).
4. **Prices are multipliers on a base price list**, clamped to a corridor. A node's price depends
   only on its state; nodes are coupled only through trade flows (arbitrage), not a general equilibrium.
5. **Explainability (the reasons log) is first-class.** Every price/availability change is traced to
   its causes (§5.12).
6. **Stability without events** is an acceptance criterion (§8): the world converges to a steady
   state, prices do not stick to the corridor edges, stocks do not grow without bound. The main
   closing loop is the supply response to price (§5.2).

## Package structure

**A hybrid with a notebook:** the core and tests live in an importable package `ddeco/` (this goes
into the Rust port and CI). A Jupyter notebook is a separate layer on top: it imports `ddeco` and
handles plots, calibration and parametric runs. No simulation logic lives in the notebook.

```
pyproject.toml                # package ddeco, deps: numpy; dev: pytest, matplotlib, jupyter; tomllib (stdlib 3.11+)
ddeco/
  goods.py        # Good, Category; good parameters: α, ε, β, [m_min,m_max], δ, T, c0, κ, vol, ω (§7)
  world.py        # Node, Edge (directed arc), the World graph; village→city binding (hub)
  state.py        # SimState: s_ig, m_ig, pipelines q[e,g,d], π_i; clone()/snapshot for undo
  effects.py      # Effect(target, kind, mult|add, duration, decay); weight w_k(t) §3;
                  #   composition clamp(base·Π(1+(mult-1)·w)+Σ add·w, min, max); effect expiry
  rng.py          # counter-based RNG: a deterministic hash of (seed, day, target_id, template_id)
  formulas.py     # pure functions §5.1–5.8: s*, production, demand/consumption, spoilage,
                  #   price multiplier + smoothing, prosperity
  trade.py        # §5.6 arbitrage: m̂ with goods in transit, γ_e, Δ_eg, A/u/ψ/φ/f, pipeline insertion;
                  #   mandatory hub delivery η_v
  purchase.py     # §5.7 price impact of a player's purchase/sale (numerical summation over units)
  caravans.py     # §5.10 materialisation of named caravans by accumulated value V_e ≥ θ_car
  shocks.py       # §5.11 background shocks from templates via the counter-based RNG (can be turned off)
  reasons.py      # §5.12 reasons log: effective parameters, flows, active effects; top-3 contributions
  tick.py         # §4 orchestration: 10 steps per day in a fixed order
  calibration.py  # §5.14 initial state (auto-calibration of m when Pcap≠D) and world warm-up
  loader.py       # world.toml → World (import/edit/share);
                  #   state.toml ↔ SimState with periodic autosave (not on every tick)
  persistence.py  # autosave timer (~1 min wall-clock), atomic write (tmp+rename)
tests/
  test_single_node.py   # §8.1 single node: 1000 days, ±20% perturbation, |s/s*-1|<0.01 by day 300
  test_multi_node.py    # §8.2 3–4 cities/~15 nodes, |m-1|<0.05, flows without oscillation
  test_runaway.py       # §8.3 runaway: Pcap=1.5·D in the centre, reaches a ceiling, no unbounded growth
  test_determinism.py   # §8.4 two runs with one seed match; undoing N days and replaying are identical
worlds/
  example_region.toml   # an example world of 3 cities with numbers (REQUIREMENTS §11, step 2)
notebooks/
  calibration.ipynb     # imports ddeco; convergence plots (1000 days), fitting α/β/ε/T/λ,
                        #   comparing damped oscillations with the §6.3 analysis, checking that "prices feel right"
```

## Implementation order

1. **Scaffold:** `pyproject.toml` (package `ddeco`, numpy, pytest), module skeletons. Pin down the
   TOML schemas of `world.toml` (description) and `state.toml` (state) in `loader.py`, write
   `worlds/example_region.toml`. Autosave (`persistence.py`) can wait until the end of the stage.
2. **Single-node core (price–stock–production):** `goods.py`, `state.py`, `formulas.py`
   (§5.1–5.5), `effects.py` (§3), a minimal `tick.py` without trade. Immediately `test_single_node.py`
   (§8.1) — it checks the §6 loop numerically (the reference spectral radii are already computed in §6.3).
   Create `notebooks/calibration.ipynb`: convergence plots and comparison with the §6.3 analysis.
3. **Demand/prosperity/spoilage:** extend `formulas.py` (§5.3, §5.4, §5.8), add π to the state.
4. **Multi-node + trade:** `world.py` (graph, hubs), `trade.py` (§5.6, pipelines, hub
   delivery), the arrival/dispatch steps in `tick.py`. `test_multi_node.py` (§8.2).
5. **Determinism and stability:** `rng.py` (counter-based), `shocks.py` (§5.11), `calibration.py`
   (§5.14, warm-up). `test_determinism.py` (§8.4, including undo/replay) and `test_runaway.py` (§8.3).
6. **Caravans, player purchases, reasons log:** `caravans.py` (§5.10), `purchase.py` (§5.7),
   `reasons.py` (§5.12). Integration into `tick.py` (steps 9–10).
7. **Update `CLAUDE.md`:** add the Python commands (pytest, running a simulation), note that
   implementation of the core in Python has started.

## Extension points (lay down, do not implement)

- **Production chains (§5.13):** design `formulas.py`/demand so that demand for inputs
  is added to D_ig without reworking the tick (v1.1).
- **Money/taxes:** goods only in v1; taxes as a share of production or garrison demand (§10).
- **WASM reproducibility:** pow/exp/ln give different bits across platforms (§9) — in the Python
  reference we record golden runs "with tolerance" so we can check Rust later.

## Verification

- `pytest tests/` — all four stability tests (§8.1–8.4) are green. This is the model's acceptance
  criterion.
- Example run: load `worlds/example_region.toml`, "warm up the world" until convergence,
  then check by hand that the prices "feel" right (REQUIREMENTS §11, step 2); apply a
  test effect (a monster on the road, ×2.5 difficulty for 10 days) and make sure the reasons log
  names the right cause of the price rise at the affected node.
- Determinism: the same seed gives an identical run; undoing N days and replaying gives the same
  result (a manual check on top of `test_determinism.py`).
