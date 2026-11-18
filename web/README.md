# Dungeonomics — web UI

A static offline PWA over the WASM core (`crates/ddeco-wasm`). Vite + TypeScript +
Svelte 5. Everything runs in the browser; the world and the state are TOML files.

## Running

```bash
# 1) build the WASM package (needs wasm-pack: `cargo install wasm-pack`):
npm run wasm        # → web/src/pkg (generated, not versioned)

# 2) dependencies and the dev server:
npm install
npm run dev         # http://localhost:5173

# production build (type-check + vite build → web/dist):
npm run build
npm run preview
```

The example region (`worlds/example_region.toml`) loads on startup. Your own world/state
is loaded as a file from the header; the "save" button downloads the current state as `state.toml`.

## Structure

- `src/store.svelte.ts` — a reactive wrapper over the `Sim` facade (Svelte 5 runes): holds
  the instance and re-reads the snapshots after each action.
- `src/types.ts` — a TS mirror of the facade DTOs (methods return `any` from serde-wasm-bindgen).
- `src/i18n/` — UI catalogs: `en.ts` holds the reference keys, `ru.ts` must have the same
  ones (checked by `svelte-check`); `index.svelte.ts` provides `t()`, the language switch
  (localStorage → browser language → EN) and the renderer for structured causes (§5.12).
- `src/components/*` — the §5.8 panels: price table (click → reasons log), flows/caravans,
  effects (+ add form), GM actions (stock, prosperity, deals).

To add a language: create `src/i18n/<lang>.ts` typed as `Messages`, then register it in
`CATALOGS` and `LANGS` in `index.svelte.ts`.
