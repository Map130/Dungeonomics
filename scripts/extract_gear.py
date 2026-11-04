#!/usr/bin/env python3
"""Извлечь товары и цены из HTML-справочника (dm-stuff.ru) в CSV.

Источник — сохранённая страница «Товары и снаряжение». Таблица одна, 20 колонок
в фиксированном порядке (см. HEADERS). Для экономики важны название, категория,
цена и масса; остальное вытаскиваем по возможности.

Цены в D&D выражены в монетах разного достоинства. Приводим к золотым (gp):
    pp=10, gp=1, ep=0.5, sp=0.1, cp=0.01

Запуск:
    python scripts/extract_gear.py \
        "references/Товары и снаряжение _ Инструменты мастера D&D.html" \
        references/gear.csv
"""

from __future__ import annotations

import csv
import html
import re
import sys
from pathlib import Path

# Индексы колонок (порядок <td> совпадает с шапкой таблицы).
NAME, PROPS, AC_BONUS, AC, STEALTH, DAMAGE, DMG_TYPE, HP, LENGTH, VOLUME, \
    POISON, QTY, RANGE, REQ, SAVE, MASS, PRICE, CATEGORY, NAME_ALT, NOTE = range(20)

COIN_TO_GP = {
    "platinum": 10.0,
    "gold": 1.0,
    "electrum": 0.5,
    "silver": 0.1,
    "copper": 0.01,
}
COIN_ABBR = {
    "platinum": "pp",
    "gold": "gp",
    "electrum": "ep",
    "silver": "sp",
    "copper": "cp",
}

_ROW = re.compile(r'<tr class="GearTable_row">(.*?)</tr>', re.S)
_CELL = re.compile(r'<td class="GearTable_cell[^"]*">(.*?)</td>', re.S)
_COIN = re.compile(
    r'<span class="Monet Monet-(\w+)"[^>]*>\s*<span class="Monet_text">([^<]*)</span>', re.S
)
_TAG = re.compile(r"<[^>]+>")
_LINK = re.compile(r'<a class="GearTable_link" title="([^"]*)"\s+href="([^"]*)"[^>]*>(.*?)</a>', re.S)
_CATS = re.compile(r'<span class="GearCategory" title="([^"\n]*)[^"]*">', re.S)
_KG = re.compile(r"\(([\d,\.]+)\s*кг\)")


def strip_tags(s: str) -> str:
    return html.unescape(_TAG.sub("", s)).strip()


def num(s: str) -> float | None:
    s = s.strip().replace(",", ".").replace(" ", "")
    s = s.replace(" ", "")
    try:
        return float(s)
    except ValueError:
        return None


def parse_price(cell: str) -> tuple[float | None, str]:
    """Вернуть (цена в gp, человекочитаемая сырьевая строка)."""
    parts = _COIN.findall(cell)
    if not parts:
        return None, ""
    total = 0.0
    raw = []
    ok = False
    for coin, text in parts:
        v = num(text)
        if v is None:
            continue
        total += v * COIN_TO_GP.get(coin, 0.0)
        raw.append(f"{text.strip()} {COIN_ABBR.get(coin, coin)}")
        ok = True
    return (round(total, 4) if ok else None), ", ".join(raw)


def parse_name(cell: str) -> tuple[str, str, str]:
    """(name_ru, name_en, url). title вида «Русское (English)»."""
    m = _LINK.search(cell)
    if not m:
        return strip_tags(cell), "", ""
    title, url, text = m.group(1), m.group(2), strip_tags(m.group(3))
    en = ""
    mt = re.search(r"\(([^()]*)\)\s*$", title)
    if mt:
        en = mt.group(1).strip()
    return text, en, url


def main(src: str, out: str) -> None:
    text = Path(src).read_text(encoding="utf-8")
    rows = _ROW.findall(text)

    records = []
    for row in rows:
        cells = _CELL.findall(row)
        if len(cells) < 20:
            continue  # шапка (th) или нестандартная строка
        name_ru, name_en, url = parse_name(cells[NAME])
        if not name_ru:
            continue
        price_gp, price_raw = parse_price(cells[PRICE])
        cats = "|".join(dict.fromkeys(html.unescape(c).strip() for c in _CATS.findall(cells[CATEGORY])))
        mass_kg = _KG.search(cells[MASS])
        records.append({
            "name_ru": name_ru,
            "name_en": name_en,
            "category": cats,
            "price_gp": price_gp if price_gp is not None else "",
            "price_raw": price_raw,
            "mass_kg": num(mass_kg.group(1)) if mass_kg else "",
            "damage": strip_tags(cells[DAMAGE]),
            "damage_type": strip_tags(cells[DMG_TYPE]),
            "url": url,
        })

    records.sort(key=lambda r: (r["category"], r["name_ru"]))
    fields = ["name_ru", "name_en", "category", "price_gp", "price_raw",
              "mass_kg", "damage", "damage_type", "url"]
    with open(out, "w", encoding="utf-8", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        w.writerows(records)

    priced = sum(1 for r in records if r["price_gp"] != "")
    print(f"Строк: {len(records)}  с ценой: {priced}  → {out}")


if __name__ == "__main__":
    args = sys.argv[1:]
    src = args[0] if args else "references/Товары и снаряжение _ Инструменты мастера D&D.html"
    out = args[1] if len(args) > 1 else "references/gear.csv"
    main(src, out)
