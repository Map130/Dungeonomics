# D&D Eco — веб-UI

Статический офлайн-PWA поверх WASM-ядра (`crates/ddeco-wasm`). Vite + TypeScript +
Svelte 5. Всё считается в браузере; мир и состояние — файлы TOML.

## Запуск

```bash
# 1) собрать WASM-пакет (нужен wasm-pack: `cargo install wasm-pack`):
npm run wasm        # → web/src/pkg (генерируется, не версионируется)

# 2) зависимости и dev-сервер:
npm install
npm run dev         # http://localhost:5173

# прод-сборка (type-check + vite build → web/dist):
npm run build
npm run preview
```

При старте загружается пример региона (`worlds/example_region.toml`). Свой мир/состояние
загружаются файлом в шапке; кнопка «сохранить» выгружает текущее состояние в `state.toml`.

## Устройство

- `src/store.svelte.ts` — реактивная обёртка над фасадом `Sim` (Svelte 5 runes): держит
  экземпляр и перечитывает снимки после каждого действия.
- `src/types.ts` — TS-зеркало DTO фасада (методы возвращают `any` от serde-wasm-bindgen).
- `src/components/*` — панели §5.8: таблица цен (клик → лог причин), потоки/караваны,
  эффекты (+форма добавления), действия мастера (запас, процветание, сделки).
