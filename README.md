# Dungeonomics — a living economy for tabletop RPGs

An open-source tool for tabletop RPG game masters (primarily D&D). The GM describes a region once — nodes (cities, villages, camps), the roads between them and the goods. From then on, player actions and world events change prices, availability and trade flows on their own. No manual bookkeeping needed.

The idea is borrowed from the economy of [Mount & Blade: Bannerlord](https://www.taleworlds.com/en/Games/Bannerlord).

## Status

**Working v1 core and web UI.** The Python prototype of the core passes the stability autotests and
serves as the reference. The core is ported to Rust (`crates/ddeco-core`) with golden parity to
Python and compiles to WASM; the `Sim` facade (`crates/ddeco-wasm`) drives an offline PWA
(`web/`) with an English and a Russian UI. Production chains (§5.13) are planned for v1.1.

Documentation:
- [`REQUIREMENTS.md`](REQUIREMENTS.md) — product requirements v1
- [`MODEL.md`](MODEL.md) — the mathematical model v1
- [`DECISIONS.md`](DECISIONS.md) — decision log
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — how to contribute

## How it works

- A simulation step is one in-game day.
- Prices are a multiplier on a base price list (the SRD or homebrew), clamped to a configurable corridor.
- Nodes are coupled only through trade flows: if the price difference exceeds the travel cost, part of the stock "travels" with a delay.
- Every price change is explained: "bread in Redwater went up because the road from Millbrook is blocked (×2.5, 7 days left) and the stock fell to 38%".
- The world is reproducible: one seed → the same run; undoing and replaying a day gives the same result.

## Architecture

```
Python prototype → Rust core → WASM → offline PWA
```

- **Reference core (`ddeco/`):** a Python library — simulation, effects, reasons log.
- **Rust core (`crates/ddeco-core`):** the port used in production, golden parity with Python.
- **WASM facade (`crates/ddeco-wasm`):** `Sim` over the core for the web UI (wasm-bindgen).
- **Web UI (`web/`):** Vite + TypeScript + Svelte 5, offline PWA, EN/RU.
- **Notebook (`notebooks/`):** parameter calibration, plots, comparison with the analysis.
- **World:** two TOML files — `world.toml` (description, versioned) and `state.toml` (state, autosaved).

## Build

```bash
cargo test --workspace                    # core + facade (Python reference tests: pytest -q)
cargo build -p ddeco-wasm --target wasm32-unknown-unknown   # WASM compatibility check

# Package WASM for the UI (needs wasm-pack: `cargo install wasm-pack`):
wasm-pack build crates/ddeco-wasm --target web --out-dir ../../web/src/pkg   # browser package (PWA)
wasm-pack build crates/ddeco-wasm --target nodejs --out-dir pkg-node        # Node package
node crates/ddeco-wasm/smoke/smoke.cjs    # smoke test of the packaged JSON shape

# Web UI (see web/README.md):
npm --prefix web install && npm --prefix web run dev
```

## Licence

MIT, see [LICENSE](LICENSE).

### Attribution

The base price list (`references/srd_gear.csv`) is derived from the SRD 5.1.

This work includes material taken from the System Reference Document 5.1 ("SRD 5.1") by Wizards of the Coast LLC and available at https://dnd.wizards.com/resources/systems-reference-document. The SRD 5.1 is licensed under the Creative Commons Attribution 4.0 International License available at https://creativecommons.org/licenses/by/4.0/legalcode.
