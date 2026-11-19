> **Status:** closed — stages R29–R32 done.

# Plan: M9 — overview badges by price, honest shop prices, calmer drivers

## Context

M8 left these GM-visible issues (final review, parked):
- The overview shows a "shortage" badge on all 10 settlements even after warm-up: the core verdict
  is `r = s/s* < 0.9`, and importers settle at r ≈ 0.4–0.9 in equilibrium.
- Plate armour is "too big for this market" almost everywhere: the `weapons` stock is thin.
- The shop list's sell price can overstate what a large sale pays (up to ~5×): the list uses
  `base·m·(1 ± spread)`, while a real deal walks the §5.7 curve.
- Generic category items (perfume → luxury, bread → food) are priced off whichever category good
  has the most stock in that town, so the same item's price jumps between towns for no visible reason.
- Facade errors ("cannot buy the node's whole stock") reach the RU UI in English.
- The manual effect form takes a good / category as free text.

User decisions: the overview badge is **by price** — show the most expensive good if its multiplier
is ≥ 1.5 ("⚠ expensive: Fish ×1.8"); no badge otherwise. The core verdict and the reasons log stay
as they are (§5.12). The rest is the planner's list above, approved as "do it".

Planner's calls:
- Category roles resolve **world-wide**: the category's good with the largest total base production
  capacity in the world (ties by id) — the same driver in every town. Exact good-id roles are unchanged.
- Shop list prices come from a real quote of **one item** (`app.quote(node, driver, units(1), sell)`);
  if the quote fails (exceeds the stock), the buy cell shows "—" and the row is "too big". No-driver
  rows keep the mean-multiplier formula.
- Weapons and luxury get deeper stocks in the example world (data only, `coverage_days`), with the
  M8 stability test still passing and plate armour buyable in every city after warm-up.

## Global Constraints

- No model-logic change in `ddeco/` or `crates/ddeco-core/src` (data, facade, UI only); golden changes only from world data.
- Python and Rust in lockstep; determinism; golden parity tolerance 1e-9.
- UI strings via `en.ts` + `ru.ts`; English elsewhere; commits `type(scope): description (Rn)`, NO Claude attribution.

## Review Focus

1. A settlement where no good reaches ×1.5 — no badge at all (not an empty tag).
2. A shop row whose one-item quote fails — "—" in the buy column, Sell still quoted, no error banner from listing.
3. A world whose category has no producer anywhere (total capacity 0 for all its goods) — resolution still deterministic (by id), or null if the category has no goods.
4. An unknown facade error string — shown as-is (no crash, no empty banner).
5. Switching language with an error banner shown — the banner re-renders in the new language.

---

### Task 1 (R29): Overview badge by price ✅

**Files:** `web/src/overview.ts` (new, pure), `web/src/overview.test.ts`, `web/src/components/SettlementsTable.svelte`, `web/src/i18n/en.ts`, `ru.ts`.

- `export const EXPENSIVE = 1.5;` and `export function mostExpensive(goods: { good: string; multiplier: number }[], threshold = EXPENSIVE): { good: string; multiplier: number } | null` — the good with the highest multiplier if ≥ threshold, ties by good id; else null.
- SettlementsTable: replace the shortage badge with `⚠ {t("settlements.expensive")}: {goodName(g)} ×{m.toFixed(2)}`; key `settlements.expensive` "expensive" / "дорого"; remove `settlements.alert`.
- Tests: empty → null; all < 1.5 → null; exactly 1.5 → that good; two goods tie → lower id; the max wins.

Commit `feat(web): overview badge shows the most expensive good (R29)`.

### Task 2 (R30): Deeper weapons and luxury stocks (world data) ✅

**Files:** `worlds/example_region.toml` (header comment + data), `tests/test_example_world.py`, `crates/ddeco-core/tests/example_world.rs`, `golden/` (regenerated), `DECISIONS.md` (append a line to D9 about the new values).

- Raise stock depth for `weapons` (and luxury goods if needed) so that after warm-up the `weapons` stock in every city (redwater, stonehaven, greenvale) exceeds 110 units (plate armour = 1500 gp / 15 gp = 100 units, plus margin). Prefer `coverage_days`; keep the M8 acceptance (no node×good pinned within 1e-3 of the corridor edges, `PINNED_OK` empty).
- Add to both stability tests: `weapons` stock > 110 in each city after warm-up.

Commit `feat(world): deeper weapons and luxury stocks (R30)`.

### Task 3 (R31): World-wide category drivers and quoted list prices ✅

**Files:** `crates/ddeco-wasm/src/lib.rs` (`GoodInfo.total_capacity: f64` = sum of base capacities over nodes), `crates/ddeco-wasm/smoke/smoke.cjs`, `web/src/types.ts`, `web/src/shop/drivers.ts`, `web/src/shop/shop.test.ts`, `web/src/components/ShopPanel.svelte`.

- `resolveDriver(role: Role | null, goods: { id: string; category: string; total_capacity: number }[]): string | null` — exact id first; else the category's good with the largest `total_capacity`, ties by id; null if none. The `nodeGoods` parameter is removed (update all callers).
- ShopPanel list prices: per row, `units = equivalentUnits(item.priceGp, 1, driverBase)`; buy = `app.quote(node, driver, units, false)?.total`, sell = `app.quote(node, driver, units, true)?.total`. A failed buy quote shows "—" and marks the row too big. Listing must not leave an error banner: add `app.quoteQuiet(...)` in the store that does not set `app.error`. No-driver rows keep the mean-multiplier formula.
- Tests: resolveDriver for `food` → the food good with the largest total capacity regardless of stocks; tie → lower id; exact id unaffected; category with goods all at capacity 0 → lowest id.

Commit `feat(web): world-wide category drivers and quoted shop prices (R31)`.

### Task 4 (R32): Localised errors, effect-form selects, docs, close M9 ✅

**Files:** `web/src/i18n/errors.ts` (new, pure) + test, `web/src/App.svelte` (render the error via the translator), `web/src/i18n/en.ts`, `ru.ts`, `web/src/components/EffectsPanel.svelte`, `DECISIONS.md` (D10), `REQUIREMENTS.md` §5.8 note, `CLAUDE.md`, this plan.

- `translateError(msg: string, t): string` maps the facade's known messages to keys: `qty must be > 0` → `err.qty`; `cannot buy the node's whole stock` → `err.wholeStock`; `no node X` → `err.noNode {id}`; `no arc X` → `err.noArc {id}`; `no good X` → `err.noGood {id}`; `no category X` → `err.noCategory {id}`; anything else → the original text. EN/RU texts in both catalogs. Tests for each mapping and the passthrough.
- EffectsPanel: good and category inputs become `<select>`s (empty option = none) listing world goods (by `goodName`) and categories (by `labelOf("category", …)`).
- D10: the overview badge is by price (≥ ×1.5), the core verdict unchanged; category roles resolve world-wide by total capacity; shop list prices are one-item quotes. REQUIREMENTS §5.8 one-line note. CLAUDE.md: M9 closed, next `m10-<name>.md` from R33, D10 in the list.
- Controller browser check after review: overview shows few/no badges after warm-up; plate armour buyable in stonehaven; a sell quote for plate armour in a village matches its list price; an error shows in RU.

Commit `feat(web): localised errors, effect-form selects; D10, close M9 (R32)`.

## Verification

- `.venv/bin/python -m pytest -q`; `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- wasm-pack node smoke; `npm --prefix web test && npm --prefix web run build`
- Browser check from Task 4.
