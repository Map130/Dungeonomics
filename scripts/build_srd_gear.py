#!/usr/bin/env python3
"""Build the base price list `references/srd_gear.csv` from SRD 5.1 (D5, MODEL §2 P0_g).

Sources (all SRD 5.1 content, CC-BY-4.0 — see the attribution in README.md):

* Equipment (adventuring gear, tools, armor, weapons, mounts and vehicles) — the
  `5e-SRD-Equipment.json` file from the 5e-bits/5e-database project (`src/2014/en`).
  It is not committed; fetch it once with:

      curl -sSfL -o references/srd/5e-SRD-Equipment.json \\
        https://raw.githubusercontent.com/5e-bits/5e-database/main/src/2014/en/5e-SRD-Equipment.json

* "Trade Goods" and "Food, Drink, and Lodging" tables (SRD 5.1, pp. 72–73). The JSON does
  not contain them, so they are transcribed below from the official SRD 5.1 PDF.

Coins are normalised to gold, as before (D2): pp=10, gp=1, ep=0.5, sp=0.1, cp=0.01.

Usage:
    python scripts/build_srd_gear.py [equipment.json] [out.csv]
"""

from __future__ import annotations

import csv
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_JSON = ROOT / "references" / "srd" / "5e-SRD-Equipment.json"
DEFAULT_OUT = ROOT / "references" / "srd_gear.csv"

COIN_TO_GP = {"pp": 10.0, "gp": 1.0, "ep": 0.5, "sp": 0.1, "cp": 0.01}

# SRD 5.1, "Trade Goods" table (p. 72): (id, name, quantity, coin).
TRADE_GOODS = [
    ("wheat-1-lb", "Wheat (1 lb.)", 1, "cp"),
    ("flour-1-lb", "Flour (1 lb.)", 2, "cp"),
    ("chicken", "Chicken", 2, "cp"),
    ("salt-1-lb", "Salt (1 lb.)", 5, "cp"),
    ("iron-1-lb", "Iron (1 lb.)", 1, "sp"),
    ("canvas-1-sq-yd", "Canvas (1 sq. yd.)", 1, "sp"),
    ("copper-1-lb", "Copper (1 lb.)", 5, "sp"),
    ("cotton-cloth-1-sq-yd", "Cotton cloth (1 sq. yd.)", 5, "sp"),
    ("ginger-1-lb", "Ginger (1 lb.)", 1, "gp"),
    ("goat", "Goat", 1, "gp"),
    ("cinnamon-1-lb", "Cinnamon (1 lb.)", 2, "gp"),
    ("pepper-1-lb", "Pepper (1 lb.)", 2, "gp"),
    ("sheep", "Sheep", 2, "gp"),
    ("cloves-1-lb", "Cloves (1 lb.)", 3, "gp"),
    ("pig", "Pig", 3, "gp"),
    ("silver-1-lb", "Silver (1 lb.)", 5, "gp"),
    ("linen-1-sq-yd", "Linen (1 sq. yd.)", 5, "gp"),
    ("silk-1-sq-yd", "Silk (1 sq. yd.)", 10, "gp"),
    ("cow", "Cow", 10, "gp"),
    ("saffron-1-lb", "Saffron (1 lb.)", 15, "gp"),
    ("ox", "Ox", 15, "gp"),
    ("gold-1-lb", "Gold (1 lb.)", 50, "gp"),
    ("platinum-1-lb", "Platinum (1 lb.)", 500, "gp"),
]

# SRD 5.1, "Food, Drink, and Lodging" table (p. 73), food and drink rows only.
FOOD_DRINK = [
    ("ale-gallon", "Ale (gallon)", 2, "sp"),
    ("ale-mug", "Ale (mug)", 4, "cp"),
    ("banquet-per-person", "Banquet (per person)", 10, "gp"),
    ("bread-loaf", "Bread (loaf)", 2, "cp"),
    ("cheese-hunk", "Cheese (hunk)", 1, "sp"),
    ("meat-chunk", "Meat (chunk)", 3, "sp"),
    ("wine-common-pitcher", "Wine, common (pitcher)", 2, "sp"),
    ("wine-fine-bottle", "Wine, fine (bottle)", 10, "gp"),
]

FIELDS = ["id", "name", "category", "subcategory", "price_gp", "price_raw", "weight_lb", "source"]


def to_gp(quantity: float, unit: str) -> float:
    return round(quantity * COIN_TO_GP[unit], 4)


def equipment_rows(path: Path) -> list[dict]:
    rows = []
    for e in json.loads(path.read_text(encoding="utf-8")):
        cost = e.get("cost") or {}
        if not cost or cost.get("quantity", 0) <= 0:
            continue  # no price in the SRD (e.g. an alms box)
        sub = e.get("gear_category") or e.get("weapon_category") or e.get("armor_category") \
            or e.get("tool_category") or e.get("vehicle_category") or ""
        if isinstance(sub, dict):
            sub = sub.get("name", "")
        rows.append({
            "id": e["index"],
            "name": e["name"],
            "category": e["equipment_category"]["index"],
            "subcategory": sub,
            "price_gp": to_gp(cost["quantity"], cost["unit"]),
            "price_raw": f'{cost["quantity"]} {cost["unit"]}',
            "weight_lb": e.get("weight", ""),
            "source": "SRD 5.1 Equipment (5e-bits/5e-database)",
        })
    return rows


def table_rows(table: list[tuple], category: str, source: str) -> list[dict]:
    return [{
        "id": id_, "name": name, "category": category, "subcategory": "",
        "price_gp": to_gp(q, unit), "price_raw": f"{q} {unit}", "weight_lb": "",
        "source": source,
    } for id_, name, q, unit in table]


def main(argv: list[str]) -> None:
    src = Path(argv[1]) if len(argv) > 1 else DEFAULT_JSON
    out = Path(argv[2]) if len(argv) > 2 else DEFAULT_OUT
    rows = (
        table_rows(TRADE_GOODS, "trade-goods", "SRD 5.1 Trade Goods table")
        + table_rows(FOOD_DRINK, "food-drink", "SRD 5.1 Food, Drink, and Lodging table")
        + equipment_rows(src)
    )
    ids = [r["id"] for r in rows]
    dupes = {i for i in ids if ids.count(i) > 1}
    if dupes:
        raise SystemExit(f"duplicate ids: {sorted(dupes)}")
    rows.sort(key=lambda r: (r["category"], r["id"]))  # stable, diff-friendly order
    with out.open("w", newline="", encoding="utf-8") as f:
        f.write("# Base price list (P0_g) from SRD 5.1, CC-BY-4.0 (attribution: README.md). "
                "Generated by scripts/build_srd_gear.py — do not edit by hand.\n")
        w = csv.DictWriter(f, fieldnames=FIELDS)
        w.writeheader()
        w.writerows(rows)
    print(f"{len(rows)} items -> {out}")


if __name__ == "__main__":
    main(sys.argv)
