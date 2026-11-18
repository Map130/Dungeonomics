> **Status:** closed — stages R19–R21.

# Plan: M6 — world overview + node pages, prices in coins

## Context

After M5 the web UI shows everything on one screen: a nodes × goods table of multipliers, flows,
reasons, GM actions, presets and effects. The user (the GM) finds it too much at once and cannot
read prices: cells show the multiplier (×1.80), the price in gold is only in a hover tooltip.

User decisions:
- **Two views.** A world overview lists the settlements, road flows and the main events; clicking a
  settlement opens its page with goods grouped by category and the player-trade form.
- **Settlement row** shows prosperity and what the settlement produces.
- **Prices in coins.** Computed from gp in the UI (`cp = gp × 100`); tables show one coin chosen by
  magnitude (≥ 100 cp → gp, ≥ 10 cp → sp, else cp; fractions below 1 cp are shown, e.g. `0.7 cp`);
  trade totals show mixed coins (`1 gp 4 sp 7 cp`). Electrum and platinum are not used.
- The core keeps prices as gp floats (multipliers on the base list, §5.5); no core or golden change.

Calls made by the planner (easy to revisit):
- Overview rows also carry a small shortage badge (the worst `r < 0.9` good), since the first
  proposal had it and the user said "норм".
- Navigation is a URL hash (`#/node/<id>`), so the browser Back button works; no router library.
- The facade snapshot also gained `provisioning` and `verdict` per good (for the badge and the stock column).
- The cross-node comparison table is removed. A per-good "cheapest at X" hint is not built (not asked).
- RU coin labels: зм / см / мм; EN: gp / sp / cp.

## R19 — Coin formatting ✅

- `web/src/coins.ts`: `formatCoin(gp: number, labels: CoinLabels): string` (one coin) and
  `formatCoinsMixed(gp: number, labels: CoinLabels): string` (whole cp, `gp sp cp` parts, zero parts
  omitted; a positive amount below 1 cp falls back to `formatCoin`). One coin: gp with up to 2
  decimals, sp and cp with up to 1 decimal, trailing zeros trimmed; below 1 cp up to 2 significant
  digits.
- i18n keys `coin.gp`, `coin.sp`, `coin.cp`; a `coins()` helper in `i18n/index.svelte.ts` returns the labels.
- Vitest cases: 0.01 gp → `1 cp`; 0.007 → `0.7 cp`; 0.5 → `5 sp`; 0.47 → `4.7 sp`; 1.5 → `1.5 gp`;
  10 → `10 gp`; mixed 1.47 → `1 gp 4 sp 7 cp`; mixed 2 → `2 gp`; mixed 0.007 → `0.7 cp`.

**Done when:** tests pass, both catalogs have the labels.

## R20 — World overview ✅

- Facade: `world_info().nodes[].produces: string[]` — goods with base capacity > 0, sorted; smoke
  checks the field.
- `app.view` in the store: `{ kind: "world" } | { kind: "node"; id: string }`, synced with the URL hash.
- `WorldView.svelte`: settlements table (name, kind, population, π, produces, shortage badge;
  click → node page), roads & flows (existing FlowsPanel content), events column (active effects
  feed, caravans, road presets only). Time controls stay in the header.

**Done when:** the overview shows no per-good price table; clicking a settlement opens its page.

## R21 — Node page ✅

- `NodeView.svelte`: back link, node header (kind, population, π, hub), goods grouped by category
  with buy / sell in one coin, stock % of target via `explain`, shortage/surplus colour; a row click
  opens the reasons for that good inline; a trade form per selected good (quote → mixed-coin total,
  execute buy / sell); node actions: node presets for this node, set/add stock, prosperity hit.
- Remove `PriceTable.svelte` and `GmPanel.svelte` (their features move here); `ReasonsPanel` and
  `PresetsPanel` become parametrised by the node / target kind.

**Done when:** everything the old screen could do is reachable; `npm test`, `npm run build` green;
browser check in EN and RU.

## Verification

- `npm --prefix web test`, `npm --prefix web run build` (0 errors).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, node smoke.
- Browser: overview → click redwater → buy 50 grain (mixed-coin total) → undo; road preset from the
  overview; RU labels and coin names.
