> **Статус:** закрыт — Milestone A/B/C = R7–R9 (коммиты `6b203fd`…`af3e509`).

# План: от WASM-биндингов к рабочему веб-UI

## Контекст

Ядро симуляции портировано в Rust (`crates/ddeco-core`, тесты зелёные, golden-паритет с
Python). В этой сессии добавлен крейт `crates/ddeco-wasm` — фасад `Sim` над ядром
(`wasm-bindgen` + `serde-wasm-bindgen`): конструктор из TOML, `step/run/undo`, `snapshot`,
`world_info`, `explain`, `quote/trade`, разовые события, `save/load_state`. Собирается под
`wasm32`, 5 smoke-тестов проходят.

Цель этого этапа — довести биндинги до UI-готовности и поднять минимальный, но настоящий
веб-UI (статический офлайн-PWA, §7 / REQUIREMENTS §5.8), чтобы мастер за столом видел цены,
потоки, ленту событий и лог причин, мог вручную вмешиваться и сохранять мир файлом.

Решения пользователя: порядок **фасад → упаковка → UI**; стек UI **Vite + TypeScript +
Svelte**; фоновые шоки — **выключаемый тумблер** (по умолчанию off).

Пробел, выявленный при сверке с REQUIREMENTS §5.8: UI требует «быстрое добавление эффекта из
интерфейса» и ленту активных эффектов/караванов/потоков — сейчас фасад этого не умеет.

---

## Milestone A — доведение фасада (`ddeco-wasm` + 1 аддитивная правка ядра)

Почти всё в `crates/ddeco-wasm/src/lib.rs`; типы ядра (`effects::Effect`, `state.effects`,
`state.caravans`, `caravans::Caravan`) уже публичны с публичными полями — новых структур в
ядре не требуется.

1. **`add_effect(...)`** — параметрический эффект из UI (§5.8: неурожай на `pcap`, дорога
   ×2.5). Фасад строит `effects::Effect` напрямую, принимая строки для `op` (`mult`/`add`),
   `decay` (`step`/`linear`/`exp`), `target_kind` (`node`/`edge`); `start_day = self.state.day`.
   Валидировать существование `target_id` (узел/дуга) и товара/категории → `Err(JsValue)`.
   Пишет снимок (`record()`), поддерживает `undo`.
2. **`effects()`** — лента активных эффектов: для каждого с ненулевым весом на текущий день
   отдать `{target_kind, target_id, param, op, value, source, weight, factor, days_left}`
   (фактор — как в `reasons::effect_factor`, переиспользовать логику/формат текста).
   Опционально `remove_effect(index)` для снятия вручную.
3. **`caravans()` и `flows()`** — для «потоки по дорогам / лента караванов» (§5.8):
   `caravans()` отдаёт `state.caravans` (edge, departed/arrival, value, guard, cargo, source);
   `flows()` — по каждой дуге суммарный груз в пути (использовать `SimState::in_transit_to`
   или прямой проход по `pipeline`) + накопитель `caravan_accum` против порога θ_car.
4. **Цены покупки/продажи в снапшоте** — добавить `spread` узла в `world_info` и в `GoodView`
   поля `buy_price = base·m·(1+spread)`, `sell_price = base·m·(1−spread)` (§5.8 «покупка и
   продажа»). Точную цену крупной сделки по-прежнему даёт `quote`.
5. **Тумблер шоков** — аддитивная функция `loader::shocks_from_toml(text) -> Vec<ShockTemplate>`
   в `crates/ddeco-core/src/loader.rs`, парсящая секцию `[[shocks]]` (поля из
   `shocks::ShockTemplate`). **Структуру `World` не трогаем** (иначе каскад правок struct-литералов
   в тестах ядра и риск golden) — шаблоны живут отдельно в фасаде: `Sim` хранит
   `shocks: Vec<ShockTemplate>` + `shocks_enabled: bool` (default false), конструктор парсит их
   из того же world-TOML, `step/run` передают `if enabled { &self.shocks } else { &[] }`.
   Метод `set_shocks_enabled(bool)`. Golden не затрагивается: пример мира шоков не содержит,
   генератор по умолчанию выключен.

Тесты: расширить `crates/ddeco-wasm/tests/facade.rs` (host, без JS): `add_effect`/`undo`
меняют и откатывают состояние; `shocks_from_toml` парсит образец; тумблер влияет на прогон.
Юнит-тест парсинга `[[shocks]]` рядом с загрузчиком в `ddeco-core`.

Опционально: добавить в `worlds/example_region.toml` закомментированный пример `[[shocks]]`.

## Milestone B — упаковка в JS-пакет

1. `wasm-pack build crates/ddeco-wasm --target web --out-dir <web>/src/pkg` — генерит
   `.wasm` + JS-glue + `.d.ts`. Зафиксировать команду в README и npm-скрипте UI.
2. **Node smoke-тест**: отдельная сборка `--target nodejs` + крошечный скрипт
   (`new Sim(world, 0)`, `step()`, `snapshot()`, `explain()`), проверяющий, что JSON от
   `serde-wasm-bindgen` имеет ожидаемую форму (именно этот слой нельзя проверить host-тестами).
3. **CI** (`.github/workflows/ci.yml`): новая джоба — установка `wasm-pack`
   (`jetli/wasm-pack-action`), `wasm-pack build` + node-smoke. Существующая wasm-джоба
   (cargo build под `wasm32`) остаётся как быстрый барьер компиляции.

## Milestone C — веб-UI (Svelte PWA)

Новый каталог `web/` (Vite + TS + Svelte, `vite-plugin-pwa` для офлайна/манифеста/SW).
Загружает пакет из `web/src/pkg`. Хранит состояние только в памяти + файловое save/load.

Экраны/панели под REQUIREMENTS §5.8:
- **Таблица цен** по узлам × товарам: buy/sell, запас, множитель, индикатор дефицит/избыток.
- **Потоки и караваны**: по дугам — груз в пути, прогресс к θ_car; лента `caravans()`.
- **Лента эффектов/событий**: `effects()` с источником и временем жизни; снятие эффекта.
- **Лог причин**: клик по ячейке цены → `explain(node, good)` (цепочка §5.12).
- **Действия мастера**: форма `add_effect`; правка запаса (`set_stock/add_stock`);
  `prosperity_hit`; предпросмотр сделки (`quote`) и проведение (`trade`); `loot_caravan`.
- **Управление временем**: шаг дня / прогон N дней / `undo`; тумблер фоновых шоков.
- **Мир/состояние**: загрузка world-TOML и state-TOML файлом, `save()` на скачивание.

Каркас сознательно минимальный (таблицы + панели, без тяжёлого дизайна) — проверяем сквозной
путь ядро→wasm→UI. Tauri-обёртка остаётся на этап релиза (спеки §7), сейчас не трогаем.

---

## Проверка

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` — зелёные (включая новые тесты фасада и парсинга шоков).
- `cargo build -p ddeco-wasm --target wasm32-unknown-unknown` — компиляция под WASM.
- `wasm-pack build crates/ddeco-wasm --target web` — успешная генерация пакета; node-smoke
  проходит (форма JSON снапшота/отчёта корректна).
- `npm run build` в `web/` — PWA собирается; `npm run dev` — ручная проверка в браузере
  (через skill `/run`): загрузить `worlds/example_region.toml`, прокрутить дни, увидеть
  изменение цен, открыть лог причин, добавить эффект и откатить, сохранить/загрузить файл.
- Детерминизм сохранён: `undo` + повтор дня дают тот же `save()` (уже покрыто host-тестом,
  добавить аналог для шоков-on).

## Затронутые файлы

- `crates/ddeco-wasm/src/lib.rs` — методы `add_effect`, `effects`, `caravans`, `flows`,
  `set_shocks_enabled`; поля цен/спреда в DTO; хранение шаблонов шоков.
- `crates/ddeco-core/src/loader.rs` — аддитивная `shocks_from_toml` (+ юнит-тест).
- `crates/ddeco-wasm/tests/facade.rs` — новые host-тесты.
- `.github/workflows/ci.yml` — джоба wasm-pack + node-smoke.
- `worlds/example_region.toml` — опциональный пример `[[shocks]]`.
- `web/**` — новый UI-каркас (Vite+TS+Svelte, PWA), `web/src/pkg` — генерируемый пакет.
- `README.md` — команды сборки wasm-pack и запуска UI.
