> **Статус:** закрыт — этапы R0–R6 (коммиты `0b7bf36`…`660aa61`).

# План: порт ядра `ddeco` на Rust (с бенчами и CI)

## Context

Milestone 1 завершён: Python-прототип `ddeco/` реализует модель MODEL.md §4–5.12 и проходит
§8.1–8.4 (35 тестов). Ядро специально написано на чистом stdlib-Python с явным отсортированным
обходом (§9) ради механического порта. Следующая веха — порт в Rust (далее → WASM/UI, отдельно).

Python остаётся **постоянным эталоном-оракулом**: генерирует golden-прогоны, на которых
проверяется Rust.

## Детерминизм (определяет стратегию проверки)

- **Внутри одной сборки Rust — бит-в-бит** (нативный x86 == WASM): фиксированный порядок
  обхода/суммирования + единая реализация трансцендентных через крейт `libm` (не `std::f64`),
  чтобы `powf/exp/ln` совпадали на всех таргетах.
- **Rust ↔ Python — «с допуском»** (разные libm): относительный порог `1e-9` на старте, можно
  ослабить точечно, если где-то накапливается.
- **RNG/хеши — точны межъязыково**: `blake2` целочисленный. Повторяем ровно кодировку входа
  Python (`seed`, `day`, части как десятичные строки через `0x1f`; u64 из первых 8 байт
  little-endian / 2^64) → последовательности шоков и охраны караванов совпадают бит-в-бит.

## Структура (cargo workspace)

```
Cargo.toml                 # workspace: members = crates/*
crates/
  ddeco-core/              # вся логика, без UI, WASM-совместима
    src/{lib,goods,world,state,effects,formulas,tick,trade,
         calibration,rng,shocks,purchase,caravans,reasons,events,loader,mathf}.rs
    benches/sim.rs         # criterion: step_day на примере мира, прогрев
    tests/{stability,golden}.rs
  ddeco-wasm/              # R7, отдельная веха: wasm-bindgen обёртка
golden/                    # эталонные фикстуры (JSON) от Python, в репозитории
scripts/dump_golden.py     # генератор golden из ddeco (Python)
.github/workflows/ci.yml   # fmt + clippy + test + bench-build + wasm-build + python pytest
```

Крейты: `serde`+`toml` (мир), `serde_json` (golden), `blake2`, `libm`, `criterion` (dev).
Детерминизм: в горячем пути **нет `HashMap`** — id интернируем в индексы (один раз отсортировав
строки, §9), состояние держим в плотных `Vec`. `mathf.rs` — тонкая обёртка над `libm`.

## Golden-run контракт (якорь порта)

`scripts/dump_golden.py` прогоняет сценарии и пишет `golden/*.json` (день → stock/multiplier/
prosperity/pipeline + списки шоков и караванов). Сценарии:
- одноузловой на каждую категорию §7 (возмущение ±20%);
- `example_region.toml`: прогрев + 300 дней;
- детерминизм с шоками (200 дней) — тут сверка RNG бит-в-бит;
- покупка игрока, грабёж каравана, удар по процветанию.

Rust-тест (`tests/golden.rs`) грузит те же JSON и сверяет: числа — с допуском `1e-9`, RNG/караваны — точно.

## Фазы

- **R0 Каркас:** workspace, `ddeco-core`, зависимости, `mathf` над `libm`, пустой CI (fmt/clippy/test).
- **R1 Golden:** `scripts/dump_golden.py` + фиксация формата JSON; сгенерировать `golden/`.
- **R2 Данные+loader:** goods/world/state, интернирование id, загрузка TOML == Python.
- **R3 Ядро:** `effects` (§3) + `formulas` (§5.1–5.8) + `tick` (§4). Golden одно/многоузловой в
  допуске; порт §8.1/§8.2.
- **R4 RNG+шоки:** `rng` (blake2, бит-в-бит с Python, отдельный тест до логики) + `shocks` +
  откат/повтор; §8.4.
- **R5 Остальное:** `trade` (§5.6), `caravans` (§5.10), `purchase` (§5.7), `reasons` (§5.12),
  `events`; все golden-сценарии в допуске; §8.3.
- **R6 Автотесты+бенчи:** §8.1–8.4 как нативные Rust-тесты; criterion-бенчи step_day/прогрев.
- **R7 (отдельная веха):** `ddeco-wasm` wasm-bindgen API; сборка в WASM. В порт не входит, но API
  проектируем в R0 (состояние живёт в Rust, JS дёргает методы).

## CI (GitHub Actions, с самого начала)

`.github/workflows/ci.yml`, job'ы:
1. **rust**: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`.
2. **bench-build**: `cargo build --benches` (не гоняем бенчи в CI, только компиляция).
3. **wasm**: `rustup target add wasm32-unknown-unknown` + `cargo build -p ddeco-core --target wasm32-unknown-unknown` (проверка WASM-совместимости ядра; `ddeco-wasm` подключится в R7).
4. **python**: `pip install -e ".[dev]"` + `pytest -q` (эталон не должен ломаться).

## Verification

- `cargo test --workspace` зелёный, включая `tests/golden.rs` (допуск 1e-9) и порт §8.1–8.4.
- `cargo build -p ddeco-core --target wasm32-unknown-unknown` собирается (ядро WASM-совместимо).
- `cargo bench` локально запускается (в CI только сборка бенчей).
- RNG-тест: первые N значений `uniform`/`chance` совпадают с Python бит-в-бит.
- Python `pytest -q` остаётся зелёным (эталон).
