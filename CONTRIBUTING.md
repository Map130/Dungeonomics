# Contributing to Dungeonomics

Thanks for your interest! Issues and pull requests are welcome.

## Ground rules

- **The specs are the source of truth.** `REQUIREMENTS.md` (what and why) and `MODEL.md` (the maths)
  are numbered with `§`. When code and spec disagree, the spec wins unless `DECISIONS.md` says
  otherwise. Items marked *(proposed)* are not decided yet — open an issue before building on them.
- **Reference spec sections** in code, tests and commits: `// §5.6 arbitrage`, `fix(trade): ... §5.6`.
- **Record decisions.** A change that settles an open question or alters a spec gets an entry in
  `DECISIONS.md` (`## D<n> · YYYY-MM-DD · title`, 2–4 lines: what, why, which `§`).
- **English everywhere** — code, comments, docs, commit messages. UI strings go through the
  catalogs in `web/src/i18n/` (add the key to `en.ts` and every other language).

## Invariants you must not break

See the "Core model concepts" in `CLAUDE.md`. In short: parameters change only through effects,
the daily tick is synchronous with a fixed step order (`MODEL.md` §4), prices are multipliers on a
base list, the RNG is counter-based, and the run is deterministic (§9).

The Python package `ddeco/` is the **reference oracle**. Any change to model logic goes into both
the Python and the Rust core, then:

```bash
.venv/bin/python -m pytest -q
.venv/bin/python scripts/dump_golden.py   # regenerate golden/ from Python
cargo test --workspace                    # Rust must match the golden runs
```

## Checks (what CI runs)

```bash
.venv/bin/python -m pytest -q
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --benches
wasm-pack build crates/ddeco-wasm --target nodejs --out-dir pkg-node && node crates/ddeco-wasm/smoke/smoke.cjs
wasm-pack build crates/ddeco-wasm --target web --out-dir ../../web/src/pkg && npm --prefix web ci && npm --prefix web run build
```

## Commits

[Conventional commits](https://www.conventionalcommits.org/) in English:
`type(scope): description (Rn)`, where `type` is one of `feat`, `fix`, `refactor`, `test`, `docs`,
`build`, `chore`, and `(Rn)` is the stage of the current milestone plan in `docs/plans/` when the
change belongs to one.

## Plans

Larger work is planned as a milestone in `docs/plans/m<N>-<name>.md`: context → stages `R…` with a
"done when" criterion → verification. Close a stage by marking it in the plan.

## Licence

By contributing you agree that your contribution is licensed under the MIT licence (`LICENSE`).
Data derived from the SRD 5.1 keeps its CC-BY-4.0 attribution (see `README.md`).
