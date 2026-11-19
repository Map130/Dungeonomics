> **Status:** planned — stages R25–R28.

# Plan: M8 — a richer example world and goods display names

## Context

After M7 the shop sells all 261 SRD items, but the example world simulates only five goods
(grain, fish, cloth, tools, wine) in seven settlements, so every weapon and armour is priced off
"tools" and the RU UI shows English good ids. The user approved this composition:

| Settlement | Kind | Produces |
|---|---|---|
| redwater | city | cloth, wine, tools |
| stonehaven | city | tools, weapons |
| greenvale | city | wine, cloth, silk |
| eastford (→ redwater) | village | grain, fish |
| millbrook (→ redwater) | village | grain, meat |
| greyrock (→ stonehaven) | village | grain, iron |
| lakeside (→ greenvale) | village | fish, salt |
| ironhold (→ stonehaven) | village (mine) | iron |
| saltmarsh (→ greenvale) | village | salt, fish |
| eastport | camp (trading post) | spices, silk |

Goods (SRD prices): food — grain 1 cp, fish 1 sp (proxy, as before), meat 3 sp (meat chunk),
salt 5 cp; materials — iron 1 sp (1 lb), cloth 5 sp (cotton cloth); crafts — tools 5 gp,
weapons 15 gp (longsword); luxury — wine 10 gp, spices 2 gp (pepper), silk 10 gp. No production
chains (v1.1, D3): iron does not turn into weapons.

Also approved: good display names written in the world file, `name = { en = "Grain", ru = "Зерно" }`;
the shop routes weapons/armour to the new `weapons` good and SRD trade goods to their own goods;
after warm-up no good is pinned to a corridor edge; golden fixtures of the example world are
regenerated.

Planner's calls:
- Display names are parsed by an additive loader function (`good_names_from_toml`), like
  `shocks_from_toml`; the `World` struct is not touched (struct literals in many tests). The Python
  oracle only learns to ignore the `name` key (it is UI data, not model data).
- Categories: `food` (grain, fish, meat, salt), `cloth` (cloth), `materials` (iron), `tools`
  (tools), `weapons` (weapons), `luxury` (wine, spices, silk). Food basket: grain, fish, meat, salt.
- Existing roads and their parameters stay (tests use `eastford__redwater`, travel time 2,
  capacity 600); new settlements get new roads.
- Stability acceptance for the example world (MODEL §8 spirit): after `warmup()` converges, for every
  node × good with demand, the multiplier is not within 1e-3 of `m_min`/`m_max` — or, if a good
  cannot avoid it (no producer reachable), that case is listed explicitly in the test with a reason.

## Global Constraints

- Python and Rust stay in lockstep; golden parity tolerance 1e-9; regenerate `golden/` from Python.
- Determinism (§9), per-day order (§4) and the core model are unchanged — only data, loaders, facade, UI.
- UI strings via `web/src/i18n/en.ts` + `ru.ts`; good names come from the world file.
- Commits `type(scope): description (Rn)`, English, no Claude attribution (`Co-Authored-By` lines are NOT added in this repo).

## Review Focus

1. A world without `name` tables (older/custom worlds) — loads as before, UI falls back to the id.
2. A name table missing the current language — falls back to `en`, then to the id.
3. The Python loader with a `name` table — no crash (it used to cast every unknown key to float).
4. A good with no producer anywhere — it may sit at `m_max`; the stability test must say so explicitly, not hide it.
5. Shop items whose role now resolves to a different driver (swords → weapons) — prices change; the shop tests reflect the new mapping.

---

### Task 1 (R25): Good display names

**Files:** `ddeco/loader.py` (ignore `name`), `crates/ddeco-core/src/loader.rs` (`pub fn good_names_from_toml(text: &str) -> Result<BTreeMap<String, BTreeMap<String, String>>, String>` + unit test; the world loader ignores `name`), `crates/ddeco-wasm/src/lib.rs` (parse names in `Sim::new`, keep in `Sim`, expose `world_info().goods[].names: Record<lang, string>`), smoke check of `names`, `web/src/types.ts` (`GoodInfo.names`), `web/src/i18n/index.svelte.ts` (`goodName(id: string): string` — current language, then `en`, then id), use `goodName` wherever a good id is shown: NodeView market table, SettlementsTable "produces", ShopPanel driver label and `shop.via`, ReasonsPanel header, EffectsPanel good filter, PresetsPanel good select. Tests: Python `tests/test_loader.py` loads a world string with a `name` table; Rust unit test for `good_names_from_toml` (present / absent / partial); Vitest is not needed for `goodName` unless extracted as a pure helper (then test it).

**Done when:** pytest, cargo test, smoke, `npm test`, `npm run build` green; a world with names shows them in RU and EN. Commit `feat: good display names from the world file (R25)`.

### Task 2 (R26): The richer example world

**Files:** `worlds/example_region.toml` (rewrite per Context; keep existing node ids, roads and their parameters; add ironhold, saltmarsh, eastport and their roads; every good gets `name = { en, ru }`), tests that count nodes/arcs or name goods (`tests/test_loader.py`, `crates/ddeco-core/tests/loader_parity.rs`, others as needed), a new stability test for the example world (`tests/test_example_world.py` + Rust mirror in `crates/ddeco-core/tests/`) per the acceptance rule in Context, `golden/` regenerated.

**Done when:** all Python and Rust suites green (including §8.1–8.4 and golden parity), the new stability test passes, the warm-up converges. Commit `feat(world): richer example region — 10 settlements, 11 goods (R26)`.

### Task 3 (R27): Shop roles for the new goods

**Files:** `web/src/shop/drivers.ts`, `web/src/shop/shop.test.ts`, `web/src/presets.test.ts` if it names goods.

Role changes: weapon, armor, Ammunition → `weapons` (resolves to the `weapons` good now);
trade goods: `iron-1-lb` → `iron`, `copper-1-lb` → `materials`, `salt-1-lb` → `salt`,
`silk-1-sq-yd` → `silk`, `ginger|cinnamon|pepper|cloves|saffron-1-lb` → `spices`,
`chicken|goat|sheep|pig|cow|ox` → `meat`; food-drink: `meat-chunk` → `meat`, `bread-loaf`/`banquet`/`cheese` → `food`.
Update tests to the new example-world goods (resolveDriver: `weapons` → `weapons`, `food` → largest stock).

**Done when:** `npm test`, `npm run build` green. Commit `feat(web): shop roles for the new goods (R27)`.

### Task 4 (R28): Docs and close M8

`DECISIONS.md` D9 (good display names live in the world file, UI-only data; parsed additively);
REQUIREMENTS §5.2 note on display names; README example-world line; plan closed; CLAUDE.md M8 under
closed milestones, next plan `m9-<name>.md` from R29. Controller browser check: overview lists 10
settlements, RU names, ironhold shows iron, a longsword in stonehaven is priced off weapons.
Commit `docs: D9, close M8 (R28)`.

## Verification

- `.venv/bin/python -m pytest -q`; `.venv/bin/python scripts/dump_golden.py`
- `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- wasm-pack node smoke; `npm --prefix web test && npm --prefix web run build`
- Browser check from Task 4.
