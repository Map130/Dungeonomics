> **Status:** closed — stages R0–R6 (commits `0b7bf36`…`660aa61`).

# Plan: porting the `ddeco` core to Rust (with benches and CI)

## Context

Milestone 1 is complete: the Python prototype `ddeco/` implements the MODEL.md §4–5.12 model and
passes §8.1–8.4 (35 tests). The core was deliberately written in pure stdlib Python with explicit
sorted traversal (§9) to make the port mechanical. The next milestone is the Rust port (then →
WASM/UI, separately).

Python stays the **permanent reference oracle**: it generates the golden runs the Rust code is
checked against.

## Determinism (defines the verification strategy)

- **Within one Rust build — bit for bit** (native x86 == WASM): a fixed traversal/summation
  order + a single implementation of transcendental functions via the `libm` crate (not `std::f64`),
  so that `powf/exp/ln` match on all targets.
- **Rust ↔ Python — "with tolerance"** (different libm): a relative threshold of `1e-9` to start
  with, which can be loosened locally if errors accumulate somewhere.
- **RNG/hashes — exact across languages**: `blake2` is integer-based. We repeat Python's input
  encoding exactly (`seed`, `day`, parts as decimal strings joined by `0x1f`; a u64 from the first
  8 bytes little-endian / 2^64) → the sequences of shocks and caravan guards match bit for bit.

## Structure (cargo workspace)

```
Cargo.toml                 # workspace: members = crates/*
crates/
  ddeco-core/              # all the logic, no UI, WASM-compatible
    src/{lib,goods,world,state,effects,formulas,tick,trade,
         calibration,rng,shocks,purchase,caravans,reasons,events,loader,mathf}.rs
    benches/sim.rs         # criterion: step_day on the example world, warm-up
    tests/{stability,golden}.rs
  ddeco-wasm/              # R7, a separate milestone: wasm-bindgen wrapper
golden/                    # reference fixtures (JSON) from Python, in the repository
scripts/dump_golden.py     # golden generator from ddeco (Python)
.github/workflows/ci.yml   # fmt + clippy + test + bench-build + wasm-build + python pytest
```

Crates: `serde`+`toml` (world), `serde_json` (golden), `blake2`, `libm`, `criterion` (dev).
Determinism: **no `HashMap`** in the hot path — ids are interned into indices (sorting the strings
once, §9), the state is kept in dense `Vec`s. `mathf.rs` is a thin wrapper over `libm`.

## Golden-run contract (the anchor of the port)

`scripts/dump_golden.py` runs scenarios and writes `golden/*.json` (day → stock/multiplier/
prosperity/pipeline + lists of shocks and caravans). Scenarios:
- single node for each §7 category (±20% perturbation);
- `example_region.toml`: warm-up + 300 days;
- determinism with shocks (200 days) — here the RNG is checked bit for bit;
- a player purchase, a caravan robbery, a prosperity hit.

The Rust test (`tests/golden.rs`) loads the same JSON and compares: numbers with a `1e-9` tolerance, RNG/caravans exactly.

## Phases

- **R0 Scaffold:** workspace, `ddeco-core`, dependencies, `mathf` over `libm`, an empty CI (fmt/clippy/test).
- **R1 Golden:** `scripts/dump_golden.py` + a fixed JSON format; generate `golden/`.
- **R2 Data + loader:** goods/world/state, id interning, TOML loading == Python.
- **R3 Core:** `effects` (§3) + `formulas` (§5.1–5.8) + `tick` (§4). Single/multi-node golden within
  tolerance; port of §8.1/§8.2.
- **R4 RNG + shocks:** `rng` (blake2, bit for bit with Python, a separate test before the logic) + `shocks` +
  undo/replay; §8.4.
- **R5 The rest:** `trade` (§5.6), `caravans` (§5.10), `purchase` (§5.7), `reasons` (§5.12),
  `events`; all golden scenarios within tolerance; §8.3.
- **R6 Autotests + benches:** §8.1–8.4 as native Rust tests; criterion benches for step_day/warm-up.
- **R7 (a separate milestone):** the `ddeco-wasm` wasm-bindgen API; the WASM build. Not part of the
  port, but the API is designed in R0 (the state lives in Rust, JS calls methods).

## CI (GitHub Actions, from the start)

`.github/workflows/ci.yml`, jobs:
1. **rust**: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`.
2. **bench-build**: `cargo build --benches` (benches are not run in CI, only compiled).
3. **wasm**: `rustup target add wasm32-unknown-unknown` + `cargo build -p ddeco-core --target wasm32-unknown-unknown` (checks the core's WASM compatibility; `ddeco-wasm` joins in R7).
4. **python**: `pip install -e ".[dev]"` + `pytest -q` (the reference must not break).

## Verification

- `cargo test --workspace` is green, including `tests/golden.rs` (tolerance 1e-9) and the §8.1–8.4 port.
- `cargo build -p ddeco-core --target wasm32-unknown-unknown` builds (the core is WASM-compatible).
- `cargo bench` runs locally (CI only builds the benches).
- RNG test: the first N values of `uniform`/`chance` match Python bit for bit.
- Python `pytest -q` stays green (the reference).
