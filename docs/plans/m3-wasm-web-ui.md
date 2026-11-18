> **Status:** closed — Milestone A/B/C = R7–R9 (commits `112801b`…`1dbcc37`).

# Plan: from WASM bindings to a working web UI

## Context

The simulation core is ported to Rust (`crates/ddeco-core`, tests green, golden parity with
Python). This session added the `crates/ddeco-wasm` crate — the `Sim` facade over the core
(`wasm-bindgen` + `serde-wasm-bindgen`): a constructor from TOML, `step/run/undo`, `snapshot`,
`world_info`, `explain`, `quote/trade`, one-shot events, `save/load_state`. It builds for
`wasm32`, 5 smoke tests pass.

The goal of this stage is to make the bindings UI-ready and stand up a minimal but real web UI
(a static offline PWA, §7 / REQUIREMENTS §5.8), so the GM at the table sees prices, flows, the
event feed and the reasons log, can intervene by hand and save the world as a file.

User decisions: the order is **facade → packaging → UI**; the UI stack is **Vite + TypeScript +
Svelte**; background shocks are **a toggle** (off by default).

A gap found when checking against REQUIREMENTS §5.8: the UI needs "quick addition of an effect from
the interface" and a feed of active effects/caravans/flows — the facade cannot do that yet.

---

## Milestone A — finishing the facade (`ddeco-wasm` + 1 additive core change)

Almost everything is in `crates/ddeco-wasm/src/lib.rs`; the core types (`effects::Effect`, `state.effects`,
`state.caravans`, `caravans::Caravan`) are already public with public fields — no new structures in
the core are needed.

1. **`add_effect(...)`** — a parametric effect from the UI (§5.8: blight on `pcap`, a road
   ×2.5). The facade builds `effects::Effect` directly, taking strings for `op` (`mult`/`add`),
   `decay` (`step`/`linear`/`exp`), `target_kind` (`node`/`edge`); `start_day = self.state.day`.
   Validate that `target_id` (node/arc) and the good/category exist → `Err(JsValue)`.
   Records a snapshot (`record()`), supports `undo`.
2. **`effects()`** — the feed of active effects: for each with a non-zero weight on the current day
   return `{target_kind, target_id, param, op, value, source, weight, factor, days_left}`
   (the factor as in `reasons::effect_factor`, reuse the logic/text format).
   Optionally `remove_effect(index)` for manual removal.
3. **`caravans()` and `flows()`** — for "flows along roads / caravan feed" (§5.8):
   `caravans()` returns `state.caravans` (edge, departed/arrival, value, guard, cargo, source);
   `flows()` — per arc, the total cargo in transit (use `SimState::in_transit_to`
   or walk `pipeline` directly) + the `caravan_accum` accumulator against the θ_car threshold.
4. **Buy/sell prices in the snapshot** — add the node `spread` to `world_info` and to `GoodView`
   the fields `buy_price = base·m·(1+spread)`, `sell_price = base·m·(1−spread)` (§5.8 "buy and
   sell"). The exact price of a large deal still comes from `quote`.
5. **Shock toggle** — an additive function `loader::shocks_from_toml(text) -> Vec<ShockTemplate>`
   in `crates/ddeco-core/src/loader.rs` that parses the `[[shocks]]` section (fields from
   `shocks::ShockTemplate`). **The `World` struct is not touched** (otherwise a cascade of struct-literal
   edits in the core tests and a golden risk) — the templates live separately in the facade: `Sim` keeps
   `shocks: Vec<ShockTemplate>` + `shocks_enabled: bool` (default false), the constructor parses them
   from the same world TOML, `step/run` pass `if enabled { &self.shocks } else { &[] }`.
   A `set_shocks_enabled(bool)` method. Golden is not affected: the example world has no shocks and
   the generator is off by default.

Tests: extend `crates/ddeco-wasm/tests/facade.rs` (host, no JS): `add_effect`/`undo`
change and revert the state; `shocks_from_toml` parses a sample; the toggle affects the run.
A unit test for `[[shocks]]` parsing next to the loader in `ddeco-core`.

Optional: add a commented-out `[[shocks]]` example to `worlds/example_region.toml`.

## Milestone B — packaging as a JS package

1. `wasm-pack build crates/ddeco-wasm --target web --out-dir <web>/src/pkg` — generates
   `.wasm` + JS glue + `.d.ts`. Record the command in the README and the UI npm script.
2. **Node smoke test**: a separate `--target nodejs` build + a tiny script
   (`new Sim(world, 0)`, `step()`, `snapshot()`, `explain()`) that checks that the JSON from
   `serde-wasm-bindgen` has the expected shape (exactly the layer host tests cannot check).
3. **CI** (`.github/workflows/ci.yml`): a new job — install `wasm-pack`
   (`jetli/wasm-pack-action`), `wasm-pack build` + the node smoke. The existing wasm job
   (cargo build for `wasm32`) stays as a quick compilation gate.

## Milestone C — web UI (Svelte PWA)

A new `web/` directory (Vite + TS + Svelte, `vite-plugin-pwa` for offline/manifest/SW).
It loads the package from `web/src/pkg`. It keeps the state only in memory + file save/load.

Screens/panels for REQUIREMENTS §5.8:
- **Price table** by nodes × goods: buy/sell, stock, multiplier, shortage/surplus indicator.
- **Flows and caravans**: per arc — cargo in transit, progress to θ_car; the `caravans()` feed.
- **Effects/events feed**: `effects()` with source and lifetime; removing an effect.
- **Reasons log**: click on a price cell → `explain(node, good)` (the §5.12 chain).
- **GM actions**: the `add_effect` form; stock edits (`set_stock/add_stock`);
  `prosperity_hit`; deal preview (`quote`) and execution (`trade`); `loot_caravan`.
- **Time control**: step a day / run N days / `undo`; the background shock toggle.
- **World/state**: load world TOML and state TOML from a file, `save()` as a download.

The scaffold is deliberately minimal (tables + panels, no heavy design) — we check the end-to-end
path core→wasm→UI. The Tauri wrapper stays for the release stage (specs §7), not touched now.

---

## Verification

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` — green (including the new facade and shock-parsing tests).
- `cargo build -p ddeco-wasm --target wasm32-unknown-unknown` — compiles for WASM.
- `wasm-pack build crates/ddeco-wasm --target web` — the package is generated; the node smoke
  passes (the JSON shape of the snapshot/report is correct).
- `npm run build` in `web/` — the PWA builds; `npm run dev` — a manual check in the browser
  (via the `/run` skill): load `worlds/example_region.toml`, run some days, see
  prices change, open the reasons log, add an effect and undo it, save/load a file.
- Determinism is preserved: `undo` + replaying a day give the same `save()` (already covered by a host test;
  add an analogue with shocks on).

## Files touched

- `crates/ddeco-wasm/src/lib.rs` — the `add_effect`, `effects`, `caravans`, `flows`,
  `set_shocks_enabled` methods; price/spread fields in the DTOs; storing shock templates.
- `crates/ddeco-core/src/loader.rs` — the additive `shocks_from_toml` (+ a unit test).
- `crates/ddeco-wasm/tests/facade.rs` — new host tests.
- `.github/workflows/ci.yml` — the wasm-pack + node smoke job.
- `worlds/example_region.toml` — an optional `[[shocks]]` example.
- `web/**` — the new UI scaffold (Vite+TS+Svelte, PWA), `web/src/pkg` — the generated package.
- `README.md` — wasm-pack build and UI run commands.
