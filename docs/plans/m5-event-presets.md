> **Status:** planned — stages R15–R18.

# Event Presets Implementation Plan (M5)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** the GM applies a typical story event ("craftsman killed", "bandits on the road") in one click instead of composing effects by hand.

**Architecture:** presets are UI data (`web/src/presets.ts`) expanded into ordinary parametric effects (§3) and applied through a new atomic facade call `add_effects`, so one preset is one `undo`. The core gains no new concept; it only starts honouring road effects in trade, which the spec already requires (§2, §5.6).

**Tech Stack:** Python oracle `ddeco/`, Rust `ddeco-core` + `ddeco-wasm` (wasm-bindgen, serde-wasm-bindgen), Svelte 5 + TypeScript, Vitest (new, web unit tests).

**Spec:** the approved short design (below, "Context"); `MODEL.md` §2, §3, §5.6, §5.10; `REQUIREMENTS.md` §5.4, §5.8.

## Context

The user considered production chains (§5.13) for M5 and decided their table value ("the party killed the miller") is already covered by an effect on the good, while the cost (both cores, golden, stability risk) is high. Chains stay in v1.1; M5 delivers **event presets** instead — "90% of the benefit of chains without touching the model".

Approved design:
- Presets live in `web/src/presets.ts`: an i18n name, target kind `node | edge`, a list of effects `{param, op, value, decay, good | category filter}`, a default duration.
- Initial set: craftsman killed (node, chosen good, `pcap ×0.2`, 30 d, linear); blight (node, `pcap ×0.4` on category `food`, 60 d, linear); army billeted (node, `population ×1.3`, 20 d, step); epidemic (node, `population ×0.8` + `pcap ×0.8`, 45 d, linear); bandits (edge, `difficulty ×2.5`, 14 d, linear); bridge collapsed (edge, `capacity ×0.1`, 30 d, step).
- UI: pick preset + target (+ good when the preset needs one), optionally change the duration, apply. The effect `source` is the preset's localised name.
- Atomic undo: a new facade method `add_effects(list)` records one snapshot for the whole list.
- Spec: REQUIREMENTS §5.8 note + `DECISIONS.md` D6 (chains stay v1.1, presets instead).
- Non-goals: custom presets in the world TOML, a strength slider, one-shot events (prosperity hit) inside presets.

Found while planning (user approved fixing it in this milestone): **trade ignores road effects.** `ddeco/trade.py` and `crates/ddeco-core/src/trade.rs` read `edge.difficulty` / `edge.capacity` / `edge.delay_days()` from the world description, and `caravans` does the same for the guard and arrival. The spec defines k_e and cap_e as effective values (§2), and §5.6 uses them for γ_e, φ_e and τ_e. Today a road effect only shows up in the reasons log; the background "bandits" shock is a no-op too. R15 fixes this in both cores.

Planner's calls (not discussed with the user, easy to revisit):
- **Bridge collapsed** gets a second effect `difficulty ×3` (same duration, step): roads without a capacity limit have `capacity = inf`, and `inf × 0.1` is still `inf`, so the preset would do nothing on them.
- Army billeted uses `decay = "step"` (the army leaves at once); all other presets use the durations/decays above.
- Web unit tests use Vitest (`^3.2.4`), added to CI as `npm test`.
- The `source` is stored in the language active at apply time; switching language later does not rewrite existing effects.

## Global Constraints

- Determinism (§9): fixed traversal order, counter-based RNG, Python ↔ Rust golden parity with relative tolerance `1e-9`.
- The per-day order of `MODEL.md` §4 does not change.
- Everything in English (D5); UI strings only via `web/src/i18n/en.ts` + `ru.ts` (same keys, `ru.ts` typed as `Messages`).
- Commits: `type(scope): description (Rn)`, English, no Claude attribution.
- The facade returns plain JSON; host tests of `ddeco-wasm` cannot construct `JsValue` errors, so host-callable methods return `Result<_, String>`.

## Review Focus

1. **A preset that needs a good, applied without one** — nothing is applied and the user sees why (Apply disabled; `expandPreset` throws). Test in R17.
2. **One invalid effect in a batch** (unknown node/arc/good, bad op) — the whole batch is rejected, the state and the undo history are untouched. Test in R16.
3. **Undo right after a two-effect preset** — a single `undo` removes both effects. Test in R16.
4. **An empty batch** — rejected with an error, no snapshot recorded. Test in R16.
5. **Road effect expires** — after the effect's duration the delay and the capacity return to the base values (decay brings the multiplier back to 1). Test in R15.

---

### Task 1 (R15): Trade and caravans honour road effects (both cores)

**Files:**
- Modify: `ddeco/world.py` (`Edge.delay_days`), `ddeco/trade.py`, `ddeco/caravans.py`, `ddeco/tick.py` (move the two road helpers)
- Modify: `crates/ddeco-core/src/world.rs`, `trade.rs`, `caravans.rs`, `tick.rs` (header comment)
- Create: `tests/test_road_effects.py`, `crates/ddeco-core/tests/road_effects.rs`
- Regenerate: `golden/*.json` (expected change: `shocks_run.json` only — it is the only scenario with a road effect)

**Interfaces:**
- Produces (Python): `trade.effective_difficulty(world, state, edge: str) -> float`, `trade.effective_capacity(world, state, edge: str) -> float` (moved from `tick.py`; `tick.py` re-exports them for existing callers); `Edge.delay_days(self, difficulty: float | None = None) -> int` — uses `difficulty` when given, else `self.difficulty`.
- Produces (Rust): `pub fn eff_difficulty(world: &World, effects: &[Effect], day: i64, edge: &str) -> f64` and `pub fn eff_capacity(...) -> f64` in `trade.rs` (built on `effects::effective`, `lo = 0.0`, `hi = f64::INFINITY`, `good/category = None`); `Edge::delay_days_with(&self, difficulty: f64) -> usize`, with `delay_days()` delegating to it.

- [ ] **Step 1: Write the failing Python tests** in `tests/test_road_effects.py` (example world prepared like `_traded_world()` in `tests/test_caravans_reasons.py`):
  - `test_difficulty_effect_lengthens_delay`: add `Effect(param="difficulty", target_kind=EDGE, target_id="eastford__redwater", op=MULT, value=2.5, duration=10, decay=STEP)`; after one `step_day`, any cargo dispatched that day on that arc sits at pipeline index `ceil(2 · 2.5) − 1 = 4` (assert `len(state.pipeline["eastford__redwater"]) >= 5` and the last slot is non-empty), whereas without the effect the length stays `<= 2`.
  - `test_capacity_effect_limits_flow`: with `capacity ×0.1` (step, 30 d) on `redwater__eastford` (a city→village arc, so the flow is arbitrage only — the mandatory hub delivery of §5.6 is not capacity-limited today and stays so), over 10 days the volume added to that arc per day never exceeds `600 · 0.1 = 60` (diff of the pipeline total before/after each day, plus arrivals), and the 10-day total is lower than in the baseline run.
  - `test_road_effect_expires`: a `difficulty ×2.5` effect with `duration=5`, `decay=STEP`; on day 6 newly dispatched cargo lands at index `ceil(2 · 1.0) − 1 = 1` again.
- [ ] **Step 2: Run** `.venv/bin/python -m pytest tests/test_road_effects.py -q` — expected: FAIL (delay stays 2, flow unchanged).
- [ ] **Step 3: Implement in Python.** Move the two helpers into `trade.py`; use them in `hub_deliveries` and `arbitrage` (γ_e uses effective k_e, φ_e uses effective cap_e, τ_e = `edge.delay_days(k_eff)`), and in `caravans.materialize` (guard and arrival use effective k_e). Effective values are taken from the effects at `state.day` (the synchronous start-of-step snapshot).
- [ ] **Step 4: Run** `.venv/bin/python -m pytest -q` — expected: all green (35 + 3).
- [ ] **Step 5: Regenerate golden** `.venv/bin/python scripts/dump_golden.py`; `git diff --stat golden` — expected: only `golden/shocks_run.json` changes.
- [ ] **Step 6: Port to Rust** the same changes; add `crates/ddeco-core/tests/road_effects.rs` with the three tests above (same names, same assertions). Update the header comments of `tick.rs`/`trade.rs` that say arc parameters are taken at base values.
- [ ] **Step 7: Run** `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` — expected: green, including `multi_node.rs` against the regenerated `shocks_run.json`.
- [ ] **Step 8: Commit** `fix(core): trade and caravans use effective road parameters (R15)` — body: §2, §5.6, §5.10; golden `shocks_run.json` regenerated.

**Done when:** a road effect changes flows, delays and caravans in both cores; golden parity holds.

### Task 2 (R16): Atomic `add_effects` in the WASM facade

**Files:**
- Modify: `crates/ddeco-wasm/src/lib.rs`, `crates/ddeco-wasm/tests/facade.rs`, `crates/ddeco-wasm/smoke/smoke.cjs`, `web/src/types.ts`, `web/src/store.svelte.ts`

**Interfaces:**
- Produces (Rust):
  ```rust
  #[derive(Deserialize, Clone)]
  pub struct EffectSpec {
      pub target_kind: String, pub target_id: String, pub param: String,
      pub op: String, pub value: f64, pub duration: f64, pub decay: String,
      #[serde(default = "one")] pub tau: f64,
      #[serde(default)] pub good: Option<String>,
      #[serde(default)] pub category: Option<String>,
      #[serde(default)] pub source: String,
  }
  ```
  `fn build_effect(&self, spec: &EffectSpec) -> Result<Effect, String>` (all current `add_effect` validation, unchanged messages); host-callable `pub fn add_effect_specs(&mut self, specs: Vec<EffectSpec>) -> Result<(), String>` in a plain `impl Sim` block; exported `pub fn add_effects(&mut self, specs: JsValue) -> Result<(), JsValue>` (deserialise via serde-wasm-bindgen, delegate). `add_effect` keeps its signature and delegates to `build_effect`.
- Produces (TS): `export interface EffectSpec { target_kind: "node" | "edge"; target_id: string; param: string; op: "mult" | "add"; value: number; duration: number; decay: "step" | "linear" | "exp"; tau?: number; good?: string | null; category?: string | null; source: string }` in `types.ts`; `AppStore.addEffects(specs: EffectSpec[]): void` in `store.svelte.ts` (via `guard`, then `refresh()`).

- [ ] **Step 1: Write the failing host tests** in `facade.rs`:
  - `add_effect_specs_is_one_undo_step`: two specs on `redwater` (`pcap ×0.05` on `food`, `population ×1.3`); after `add_effect_specs` and `run(10)`, two `undo()` calls return `true` and the third returns `false`; the state equals the snapshot taken before `add_effect_specs`.
  - `invalid_spec_rejects_whole_batch`: specs `[valid, {target_id: "nowhere", ..}]` → `Err` containing `"no node nowhere"`; `save()` unchanged and `undo()` returns `false`.
  - `empty_batch_is_rejected`: `add_effect_specs(vec![])` → `Err("no effects")`; `undo()` returns `false`.
- [ ] **Step 2: Run** `cargo test -p ddeco-wasm` — expected: FAIL (no `EffectSpec`).
- [ ] **Step 3: Implement** `EffectSpec`, `build_effect`, `add_effect_specs` (validate all specs first, then one `record()`, then push all), `add_effects`; make `add_effect` use `build_effect`.
- [ ] **Step 4: Run** `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` — expected: green.
- [ ] **Step 5: Smoke + TS.** In `smoke.cjs` call `sim.add_effects([...two specs...])` and assert `sim.effects().length` grew by 2 and one `sim.undo()` brings it back. Add `EffectSpec` and `addEffects` to the web layer. Run the wasm-pack node build + `node crates/ddeco-wasm/smoke/smoke.cjs` (expected `wasm smoke: OK`) and `npm --prefix web run check` (0 errors).
- [ ] **Step 6: Commit** `feat(wasm): atomic add_effects for effect batches (R16)`.

**Done when:** a list of effects is applied all-or-nothing with a single undo step, from Rust and from JS.

### Task 3 (R17): Preset catalogue and expansion (web, unit-tested)

**Files:**
- Create: `web/src/presets.ts`, `web/src/presets.test.ts`
- Modify: `web/package.json` (devDependency `vitest ^3.2.4`, script `"test": "vitest run"`), `.github/workflows/ci.yml` (run `npm test` in `web` after `npm ci`), `web/src/i18n/en.ts`, `web/src/i18n/ru.ts`

**Interfaces:**
- Produces:
  ```ts
  export type PresetId = "craftsman_killed" | "blight" | "army_billeted" | "epidemic" | "bandits" | "bridge_collapsed";
  export interface PresetEffect { param: string; op: "mult" | "add"; value: number; decay: "step" | "linear" | "exp"; category?: string; perGood?: boolean }
  export interface Preset { id: PresetId; target: "node" | "edge"; needsGood: boolean; duration: number; effects: PresetEffect[] }
  export const PRESETS: Preset[];
  export function presetName(id: PresetId): string;   // t(`preset.${id}`)
  export function expandPreset(p: Preset, targetId: string, opts: { good?: string; duration?: number; source: string }): EffectSpec[];
  ```
  `expandPreset`: one `EffectSpec` per preset effect; `target_kind = p.target`; `duration = opts.duration ?? p.duration`; `good = perGood ? opts.good : null`; `category = effect.category ?? null`; `tau = 1`; throws `Error("preset needs a good")` when `p.needsGood && !opts.good`.
- i18n keys (EN / RU): `preset.craftsman_killed` "Craftsman killed" / "Убит ремесленник"; `preset.blight` "Blight" / "Неурожай"; `preset.army_billeted` "Army billeted" / "Армия на постое"; `preset.epidemic` "Epidemic" / "Эпидемия"; `preset.bandits` "Bandits on the road" / "Бандиты на дороге"; `preset.bridge_collapsed` "Bridge collapsed" / "Мост рухнул"; `presets.title` "Quick events" / "Быстрые события"; `presets.apply` "apply" / "применить"; `presets.good` "good" / "товар"; `presets.needsGood` "choose a good" / "выберите товар".
- Preset values (verbatim): `craftsman_killed` node, needsGood, 30 d, `[pcap mult 0.2 linear perGood]`; `blight` node, 60 d, `[pcap mult 0.4 linear category "food"]`; `army_billeted` node, 20 d, `[population mult 1.3 step]`; `epidemic` node, 45 d, `[population mult 0.8 linear, pcap mult 0.8 linear]`; `bandits` edge, 14 d, `[difficulty mult 2.5 linear]`; `bridge_collapsed` edge, 30 d, `[capacity mult 0.1 step, difficulty mult 3 step]`.

- [ ] **Step 1: Install Vitest** (`npm --prefix web install -D vitest@^3.2.4`) and add the `test` script.
- [ ] **Step 2: Write the failing tests** in `presets.test.ts`:
  - `every preset has EN and RU names` — for each id, `en["preset." + id]` and `ru["preset." + id]` are non-empty.
  - `craftsman_killed expands with the chosen good` — `expandPreset(get("craftsman_killed"), "redwater", {good: "grain", source: "x"})` equals `[{target_kind: "node", target_id: "redwater", param: "pcap", op: "mult", value: 0.2, duration: 30, decay: "linear", tau: 1, good: "grain", category: null, source: "x"}]`.
  - `craftsman_killed without a good throws` — `/needs a good/`.
  - `duration override applies to every effect` — `epidemic` with `{duration: 7}` → both specs have `duration: 7`.
  - `bridge_collapsed targets the edge with two effects` — `target_kind: "edge"`, params `["capacity", "difficulty"]`.
- [ ] **Step 3: Run** `npm --prefix web test` — expected: FAIL (module missing).
- [ ] **Step 4: Implement** `presets.ts` with the verbatim values and the i18n keys.
- [ ] **Step 5: Run** `npm --prefix web test` and `npm --prefix web run check` — expected: 5 passed, 0 errors. Add `npm test` to the CI web job.
- [ ] **Step 6: Commit** `feat(web): event preset catalogue with unit tests (R17)`.

**Done when:** the six presets expand into correct effect specs, both languages name them, tests run in CI.

### Task 4 (R18): Presets panel, docs, close M5

**Files:**
- Create: `web/src/components/PresetsPanel.svelte`
- Modify: `web/src/App.svelte` (place the panel above `EffectsPanel` in the right column), `REQUIREMENTS.md` §5.8, `DECISIONS.md` (D6), `CLAUDE.md` (status), `README.md` (one line under "How it works"), this plan (status)

**Interfaces:**
- Consumes: `PRESETS`, `presetName`, `expandPreset` (R17); `app.addEffects` (R16); `app.world` for node ids / edge ids / goods.

- [ ] **Step 1: Build the panel.** A `<select>` of presets (by `presetName`), a target `<select>` that lists node ids or edge ids by `preset.target` and stays valid when the preset changes (same `$effect` pattern as `EffectsPanel`), a goods `<select>` shown only when `needsGood`, a duration `<input type="number">` prefilled with the preset default and reset on preset change, and an apply button disabled when `needsGood` and no good is chosen (tooltip `presets.needsGood`). Apply calls `app.addEffects(expandPreset(p, target, {good, duration, source: presetName(p.id)}))`.
- [ ] **Step 2: Check** `npm --prefix web run build` — expected: 0 errors, built.
- [ ] **Step 3: Browser check** (dev server, `.claude/launch.json` → `web`): apply "Bandits on the road" on `eastford__redwater`, run 3 days, open reasons for grain in redwater → a road cause with `difficulty ×…`; the flows panel shows lower/later traffic on that road than before. Apply "Epidemic" on `redwater`, one `undo` removes both effects. Switch to RU: preset names and labels translated.
- [ ] **Step 4: Docs.** REQUIREMENTS §5.8: "Quick addition of an effect from the interface" gains "including one-click event presets (craftsman killed, blight, army billeted, epidemic, bandits, bridge collapsed)". DECISIONS: `## D6 · <date> · Event presets instead of production chains in M5` — chains stay v1.1 (D3); presets in the UI cover the table use case; road effects now reach trade (spec §2/§5.6 compliance, R15). Mark R15–R18 done here, set the status to closed, update the CLAUDE.md status line (M5 closed, next plan `m6-*.md` from R19).
- [ ] **Step 5: Full verification** (below) and **commit** `feat(web): event presets panel; close M5 (R18)`.

**Done when:** the GM can apply any preset in one click, undo it in one step, in EN and RU; docs and status updated.

---

## Verification

- `.venv/bin/python -m pytest -q` — green (incl. `tests/test_road_effects.py`).
- `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` — green; golden changed only in `shocks_run.json`.
- `wasm-pack build crates/ddeco-wasm --target nodejs --out-dir pkg-node && node crates/ddeco-wasm/smoke/smoke.cjs` — `wasm smoke: OK`.
- `wasm-pack build crates/ddeco-wasm --target web --out-dir ../../web/src/pkg && npm --prefix web test && npm --prefix web run build` — tests pass, 0 errors.
- Manual browser check from R18 step 3.
