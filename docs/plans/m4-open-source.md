> **Status:** in progress — R10–R11 done; next R12. Decision: `DECISIONS.md` D5.

# Plan: Dungeonomics goes English (open-source readiness)

## Context

The project is public on GitHub as **Dungeonomics** (D4, MIT). Everything so far is in Russian:
specs, docs, code comments, commit messages, UI labels, and the reasons log, whose text is
hard-coded in the core (`ddeco/reasons.py`, `crates/ddeco-core/src/reasons.rs`, e.g.
`"дорога A→B"`, `"дефицит"`) and baked into golden fixtures (`golden/shocks_run.json`).
The base price list (`references/gear.csv`, D2) comes from a Russian fan catalog with an unclear
licence.

User decisions (D5):
- **UI and reasons log:** EN + RU via i18n. The core emits structured reasons (key + params),
  the web UI renders them from a catalog. EN is the default, RU stays available.
- **Price list:** switch to the **SRD 5.1** equipment list (CC-BY-4.0), superseding D2.
- **Translate:** specs and docs, code comments, commit messages from now on. Old history stays.
- **No Claude attribution** in commits or PRs.

Invariants that must survive: determinism (§9), Python↔Rust golden parity, the §4 tick order,
and reasons-log explainability (§5.12). Section numbering `§` in the specs stays unchanged so
existing code references remain valid.

---

## R10 — Price list from SRD 5.1 ✅

> Done: `references/srd_gear.csv` (261 items) from `scripts/build_srd_gear.py`. Equipment comes
> from 5e-bits JSON (`references/srd/`, gitignored), Trade Goods and Food/Drink tables are
> transcribed from the official SRD 5.1 PDF (pp. 72–73). All five example-world prices already
> matched SRD values, so only comments changed and the golden fixtures are byte-identical.

- New `references/srd_gear.csv` built from the SRD 5.1 "Adventuring Gear", "Tools", "Trade
  Goods" and "Food, Drink and Lodging" tables, coins normalised to gp as before (D2). Keep the
  generator script, pointed at the new source, and add CC-BY-4.0 attribution (README + file header).
- Remove `references/gear.csv` and `scripts/extract_gear.py` (the dm-stuff.ru extractor).
- Rewrite `worlds/example_region.toml` with English good ids/names priced from the new list.
- Regenerate golden fixtures with the Python oracle (`scripts/dump_golden.py`), then confirm the
  Rust suite passes against them.

**Done when:** no Cyrillic in `references/`, `worlds/`; pytest, `cargo test` and the WASM smoke
are green on the new world; README credits the SRD.

## R11 — Structured reasons log in the core ✅

> Done: `Cause` = `kind` (`node_effect | edge_effect | stock`) + `magnitude`, `value`, `op`,
> `param`, `src`/`dst`, `source`; verdict codes `shortage | surplus | normal` — in both cores
> and the WASM `explain`. The caravan's text `source` field was dropped; the facade's
> `caravans()` returns `src`/`dst` instead. The UI renders causes with a temporary RU renderer
> (`ReasonsPanel.svelte`) until R12. The reasons log was never part of the golden fixtures, so
> they are unchanged. Test-only effect sources (e.g. `"неурожай"`) are translated in R13.

- Replace text in `ReasonReport` / `Cause` with a key + params form in **both** cores, e.g.
  `{"key": "cause.edge", "src": "A", "dst": "B", "factor": 0.8, "source": "..."}`, verdict as an
  enum (`shortage | surplus | normal`). Keep it serialisable through the WASM facade.
- Python and Rust stay in lockstep; regenerate the affected golden files once.
- Update `REQUIREMENTS` §5.12 / `MODEL` §5.12 to describe the structured form.

**Done when:** the core contains no user-facing natural-language strings; golden parity holds;
`explain` in the facade returns structured causes.

## R12 — Web UI i18n (EN default, RU)

- String catalogs `web/src/i18n/en.ts` and `ru.ts`, a tiny `t(key, params)` helper (no heavy
  dependency), a language switch persisted in localStorage, default from `navigator.language`
  falling back to EN.
- All Svelte components, the PWA manifest and reasons rendering go through the catalog.

**Done when:** UI fully usable in both languages; no Cyrillic in `web/src` outside `ru.ts`;
`npm run build` and the node smoke pass.

## R13 — Code comments and test names in English

- Python (`ddeco/`, `tests/`, `scripts/`), Rust (`crates/`), Svelte/TS, the smoke script,
  `pyproject`/`Cargo`/`package.json` descriptions. WASM facade error messages in English.

**Done when:** `git grep -P '[\x{0400}-\x{04FF}]'` hits only `web/src/i18n/ru.ts`
and docs not yet translated; all test suites, clippy and fmt green.

## R14 — Specs and docs in English

- `README.md` (plus a short "Status" and SRD credit), `web/README.md`, `CLAUDE.md`,
  `REQUIREMENTS.md`, `MODEL.md`, `DECISIONS.md`, `docs/plans/m1–m3`. Keep `§` numbers, the
  *(proposed)* marker replaces *(предложено)* — update the CLAUDE.md wording accordingly.
- Add `CONTRIBUTING.md`: conventional commits in English, `(Rn)` stage suffix, spec references.

**Done when:** the Cyrillic grep hits only `ru.ts`; CLAUDE.md status marks M4 closed.

---

## Verification

- `.venv/bin/python -m pytest -q`
- `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- WASM pack + node smoke + `npm run build` (as in CI)
- `git grep -lP '[\x{0400}-\x{04FF}]'` → only `web/src/i18n/ru.ts`
- Manual: open the PWA, switch EN↔RU, step a few days, check the reasons panel.
