# Журнал решений

Формат: `## D<n> · YYYY-MM-DD · заголовок` — что решили, почему, какой `§` затронут.
Решения D1–D3 приняты в M1 и записаны задним числом (дата — коммит, где они воплощены).

## D1 · 2026-11-04 · Формат мира и сохранений — TOML
Описание мира `world.toml` (редактируется и шарится) и автосохраняемый `state.toml` хранятся
раздельно. Автосейв периодический по wall-clock, не на каждый тик. Откат и повтор работают на
in-memory снимках, поэтому потеря бит float в TOML детерминизму не мешает.
Почему: читаемо человеком, годится и для миров сообщества, и как save-формат.
Затронуто: REQUIREMENTS §10.1, §5.9; MODEL §8.4. Коммит `cf61058`, план `m1-python-prototype.md`.

## D2 · 2026-11-04 · Базовый прайс-лист P0_g — из `references/gear.csv`
CSV генерируется скриптом `scripts/extract_gear.py` из сохранённого каталога снаряжения D&D.
Монеты приведены к золоту (pp=10, gp=1, ep=0.5, sp=0.1, cp=0.01). К этим ценам привязан
`worlds/example_region.toml`.
Почему: готовый полный прайс PHB-уровня, ручной ввод не нужен.
Затронуто: REQUIREMENTS §10.4; MODEL §2 (P0_g), §10.6 (частично). Коммит `85457e8`. **Заменено D5** (SRD 5.1).

## D3 · 2026-11-04 · Производственные цепочки отложены до v1.1
В v1 товары независимы. В коде оставлена точка расширения: спрос на входы цепочки добавляется
к D_ig без переделки тика.
Почему: сначала нужно добиться устойчивости одноуровневой модели, цепочки её усложняют.
Затронуто: REQUIREMENTS §10.2; MODEL §5.13, §10.2. План `m1-python-prototype.md`.

## D4 · 2026-11-18 · Название — Dungeonomics, лицензия — MIT
Публичное имя проекта и репозитория — Dungeonomics (`dungeonomics`), без амперсанда.
Внутренние имена пакетов (`ddeco`, `ddeco-core`, `ddeco-wasm`, `ddeco-web`) не меняются.
Лицензия MIT (было `MIT OR Apache-2.0` в Cargo, TBD в README).
Почему: проект открывается на GitHub; MIT — самая простая для инструмента ГМов.
Затронуто: REQUIREMENTS §10.7.

## D5 · 2026-11-18 · Project goes English; UI and reasons log get EN + RU i18n
Since the project is public OSS: specs, docs, code comments and new commit messages are in
English (old history stays as is). The core stops emitting Russian text: the reasons log becomes
structured (message key + params), and the web UI renders it via an EN/RU catalog, EN by default.
The base price list moves from the dm-stuff.ru catalog to the SRD 5.1 equipment list (CC-BY-4.0),
superseding D2. Commits and PRs carry no Claude attribution.
Why: an English-speaking audience on GitHub, a clean licence for the data, RU users keep their UI.
Affects: REQUIREMENTS §5.12 (reasons log), §10.4; MODEL §2 (P0_g), §5.12. Plan `m4-open-source.md`.
