> **Status:** planned — stages R22–R24.

# Plan: M7 — the settlement shop (all SRD items, priced and traded through simulated goods)

## Context

The example world simulates five goods (grain, fish, cloth, tools, wine), while players buy rope,
lanterns and longswords. The user chose: (1) a shop listing all SRD 5.1 items on each settlement
page, then (2) a richer world in M8. Shop purchases must move the economy ("давай делать хорошо").

User decisions:
- Every SRD item with a price (`references/srd_gear.csv`, 261 rows) can be bought and sold at a settlement.
- A shop deal affects the simulation: buying 20 longswords makes weapons dearer in that town.

Planner's calls (D8, easy to revisit):
- **Driver.** Each item maps to a *role* (`food`, `cloth`, `tools`, `weapons`, `luxury`, or a specific
  good id such as `grain`, `wine`). At a node the role resolves to a simulated good: a good whose id
  equals the role, else the goods whose category equals the role — the one with the largest current
  stock at that node (ties by id). No match → the item has no driver.
- **Value-equivalent units.** A shop deal of `qty` items is a §5.7 deal on the driver good of
  `units = item_base_gp · qty / driver_base_gp`, via the existing facade `quote` / `trade`. So the
  shop total equals `item_base · qty · m_avg · (1 ± spread)` with the purchase curve, and the stock
  limit applies ("cannot buy the whole stock" for a warship in a village).
- **No driver** → the price is shown with the node's mean multiplier and trading is disabled (with a note).
- Item names stay English (the SRD is English-only); the UI around them is translated.
- No core or golden change; everything lives in `web/src/shop/`.

## Global Constraints

- Prices displayed via `price()` (one coin) and totals via `total()` (mixed coins) from `web/src/i18n/index.svelte.ts`.
- UI strings only via `web/src/i18n/en.ts` + `ru.ts` (same keys).
- Unit price shown in the list: `item_base · m_driver · (1 + spread)` (buy) and `· (1 − spread)` (sell),
  `spread` from `world_info().nodes[].spread`; `m_driver` from the snapshot.
- Commits `type(scope): description (Rn)`, English, no Claude attribution.

## Review Focus

1. An item whose equivalent units exceed the driver's stock — the quote shows the facade error, nothing changes.
2. An item with no driver in the current world — price shown, trade buttons disabled, note visible.
3. Switching nodes or loading another world — the selected item and any preview reset; roles re-resolve.
4. Selling items — units are added to the driver's stock (u < 0), the price falls; a sale never fails on stock.
5. Searching with an empty string or no matches — the list shows all items / a "nothing found" line.

---

### Task 1 (R22): Shop catalogue and drivers (pure, unit-tested)

**Files:** Create `web/src/shop/catalog.ts`, `web/src/shop/drivers.ts`, `web/src/shop/shop.test.ts`.

**Interfaces (produces):**
```ts
// catalog.ts — parses references/srd_gear.csv imported with `?raw`
export interface ShopItem { id: string; name: string; category: string; subcategory: string; priceGp: number }
export function parseCatalog(csv: string): ShopItem[];   // skips the leading "#" comment line, handles quoted fields with commas, drops price ≤ 0
export const CATALOG: ShopItem[];                          // parseCatalog(raw), sorted by name
// drivers.ts
export type Role = string;                                 // "food" | "cloth" | "tools" | "weapons" | "luxury" | a good id
export function roleOf(item: ShopItem): Role | null;
export function resolveDriver(role: Role | null, goods: { id: string; category: string }[], nodeGoods: { good: string; stock: number }[]): string | null;
export function equivalentUnits(itemGp: number, qty: number, driverBaseGp: number): number;
```

`roleOf` mapping (verbatim):
- `weapon` → `weapons`; `armor` → `weapons`; adventuring-gear `Ammunition` → `weapons`.
- `tools`: `Artisan's Tools`, `Other Tools` → `tools`; `Musical Instrument`, `Gaming Sets` → `luxury`.
- adventuring-gear: `Kits` → `tools`; `Equipment Packs` → `cloth`; `Arcane Foci`, `Druidic Foci`, `Holy Symbols` → `luxury`;
  `Standard Gear` by id: matches `/clothes|robe|cloak|blanket|bedroll|tent|sack|backpack|pouch|rope|cloth|net/` → `cloth`;
  `/rations|waterskin/` → `food`; `/perfume|signet|ring|mirror|spyglass|book|ink|paper|parchment/` → `luxury`; otherwise `tools`.
- `trade-goods`: `wheat-1-lb`, `flour-1-lb` → `grain`; `chicken`, `goat`, `sheep`, `pig`, `cow`, `ox`, `salt-1-lb` → `food`;
  `canvas-1-sq-yd`, `cotton-cloth-1-sq-yd`, `linen-1-sq-yd`, `silk-1-sq-yd` → `cloth`;
  `ginger-1-lb`, `cinnamon-1-lb`, `pepper-1-lb`, `cloves-1-lb`, `saffron-1-lb`, `silver-1-lb`, `gold-1-lb`, `platinum-1-lb` → `luxury`;
  `iron-1-lb`, `copper-1-lb` → `tools`.
- `food-drink`: ids starting `wine` → `wine`; others → `food`.
- `mounts-and-vehicles`: `Tack, Harness, and Drawn Vehicles` → `tools`; `Mounts and Other Animals`, `Waterborne Vehicles` → `null`.

Tests (with the example world's goods `grain/fish:food, cloth:cloth, tools:weapons, wine:luxury`):
- `parseCatalog` on the real CSV → 261 items; `"wine-fine-bottle"` has name `"Wine, fine (bottle)"` and `priceGp` 10.
- `roleOf`: `longsword`→`weapons`, `chain-mail`→`weapons`, `wheat-1-lb`→`grain`, `rope-hempen-50-feet`→`cloth`, `lantern-hooded`→`tools`, `rations-1-day`→`food`, `wine-common-pitcher`→`wine`, `horse-riding`→`null`.
- `resolveDriver`: `weapons` → `tools` (category match); `food` with stocks grain 50 / fish 80 → `fish`; `grain` → `grain`; `luxury` → `wine`; `null` → `null`; `food` in a world without food goods → `null`.
- `equivalentUnits(15, 2, 5)` → 6.

**Done when:** `npm --prefix web test` and `npm --prefix web run check` green. Commit `feat(web): shop catalogue and item drivers (R22)`.

### Task 2 (R23): Shop panel on the settlement page

**Files:** Create `web/src/components/ShopPanel.svelte`; modify `web/src/components/NodeView.svelte` (render `<ShopPanel node={id} />` below the goods panel), `web/src/i18n/en.ts`, `ru.ts`.

**Consumes:** Task 1 interfaces; `app.world`, `app.snapshot`, `app.quote(node, good, qty, sell)`, `app.trade(...)`, `price()`, `total()`.

- A search input (case-insensitive substring on name) and a category select (all / each SRD `category`, labelled via i18n keys `shopcat.<category>`), a list capped at 50 rows with a "showing 50 of N" line, each row: name, buy, sell, driver label (`← {good} ×{m}` or `← {mean} (no driver)`).
- Clicking a row selects the item: quantity input (default 1), quote buy / quote sell → `app.quote(node, driver, equivalentUnits(...), sell)`; show `total(r.total)`, per item `price(r.total / qty)`, the driver move `m→…`; execute → `app.trade(...)`; facade errors appear via the existing `app.error` banner; after executing, a line "bought 2 × Longsword for 3 gp 2 sp" (key `shop.done`).
- No driver → buttons disabled, `shop.noDriver` note. Selection and preview reset when `node` changes.
- i18n keys (EN / RU): `shop.title` "Shop" / "Лавка"; `shop.search` "search" / "поиск"; `shop.all` "all categories" / "все категории"; `shop.showing` "showing {n} of {total}" / "показано {n} из {total}"; `shop.none` "nothing found" / "ничего не найдено"; `shop.noDriver` "not traded here — price from the town average" / "здесь не торгуют — цена по среднему городу"; `shop.via` "via {good}" / "через {good}"; `shop.done` "{verb} {qty} × {item} for {total}" / "{verb} {qty} × {item} за {total}"; `shop.bought` "bought" / "куплено"; `shop.sold` "sold" / "продано"; `shopcat.adventuring-gear` "Adventuring gear" / "Снаряжение"; `shopcat.armor` "Armor" / "Доспехи"; `shopcat.weapon` "Weapons" / "Оружие"; `shopcat.tools` "Tools" / "Инструменты"; `shopcat.mounts-and-vehicles` "Mounts and vehicles" / "Скакуны и транспорт"; `shopcat.trade-goods` "Trade goods" / "Товары"; `shopcat.food-drink` "Food and drink" / "Еда и напитки".

**Done when:** `npm --prefix web run build` green. Commit `feat(web): settlement shop with value-equivalent trades (R23)`.

### Task 3 (R24): Docs and close M7

- `DECISIONS.md` D8 (date 2026-11-18): shop items are priced and traded through a driver good in value-equivalent units (MODEL §5.7), not simulated individually; resolves REQUIREMENTS §10.6 for shop items only. Affects REQUIREMENTS §5.5, §5.8; MODEL §5.7.
- REQUIREMENTS §5.8: add "A settlement shop lists every item of the base price list; a deal moves the price of the item's driver good (D8)."
- Plan status closed; `CLAUDE.md` M7 under closed milestones, next plan `m8-<name>.md` from R25 (M8 = richer world, already agreed).
- Controller browser check: redwater → shop → search "long" → buy 2 longswords → tools price rises, undo restores; a horse shows "not traded here".

Commit `docs: shop decision D8; close M7 (R24)`.

## Verification

- `npm --prefix web test`, `npm --prefix web run build` — green.
- Browser check from Task 3.
