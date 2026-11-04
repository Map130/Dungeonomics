"""Лог причин (MODEL.md §5.12).

Для каждого изменения цены система называет причину. Цепочка:
    цена ← обеспеченность r ← {производство, спрос, потоки, порча}
          ← {активные эффекты, сложность дорог}.

Лента показывает обеспеченность узла и три главные причины по величине вклада, с
источником эффекта. Вклад оцениваем по относительному отклонению параметра,
наведённому эффектом (в v1 — без точного контрфактуала; граф мал, это можно
усилить позже).
"""

from __future__ import annotations

from dataclasses import dataclass

from .effects import Effect, Op, TargetKind
from .state import SimState
from .world import World


@dataclass(frozen=True, slots=True)
class Cause:
    magnitude: float   # величина вклада (для ранжирования)
    text: str          # человекочитаемое описание с источником


@dataclass(frozen=True, slots=True)
class PriceReport:
    node: str
    good: str
    multiplier: float
    provisioning: float      # r = s/s*
    verdict: str             # "дефицит" | "избыток" | "норма"
    causes: list[Cause]

    def top(self, n: int = 3) -> list[str]:
        return [c.text for c in self.causes[:n]]


def _effect_factor(e: Effect, day: int) -> tuple[float, str]:
    """Мгновенный вклад эффекта и его текстовое представление."""
    w = e.weight(day)
    if e.op is Op.MULT:
        factor = 1.0 + (e.value - 1.0) * w
        return abs(factor - 1.0), f"×{factor:.2f}"
    return abs(e.value * w), f"+{e.value * w:.1f}"


def explain_price(world: World, state: SimState, node: str, good: str) -> PriceReport:
    """Объяснить текущий множитель цены товара в узле (§5.12)."""
    g = world.catalog[good]
    day = state.day
    s = state.stock[(node, good)]

    # Обеспеченность r считаем по базовому спросу (без эффектов — для вердикта).
    from . import formulas as F
    pop = world.nodes[node].population
    demand = F.base_demand(g, pop, state.prosperity[node], world.params.pi_ref,
                           world.nodes[node].garrison.get(good, 0.0))
    star = F.target_stock(g, demand)
    r = s / star if star > 0.0 else float("inf")
    verdict = "дефицит" if r < 0.9 else "избыток" if r > 1.1 else "норма"

    inbound = set(world.incoming(node))
    causes: list[Cause] = []
    for e in state.effects:
        if e.weight(day) == 0.0:
            continue
        relevant = False
        label = ""
        if e.target_kind is TargetKind.NODE and e.target_id == node:
            if e.param in ("pcap", "garrison") and not e.matches_good(good, g.category):
                continue
            relevant = True
            label = {"pcap": "производство", "garrison": "гарнизон",
                     "population": "население"}.get(e.param, e.param)
        elif e.target_kind is TargetKind.EDGE and e.target_id in inbound and e.param in ("difficulty", "capacity"):
            relevant = True
            edge = world.edges[e.target_id]
            label = f"дорога {edge.src}→{edge.dst}"
        if not relevant:
            continue
        mag, factor = _effect_factor(e, day)
        src = f" ({e.source})" if e.source else ""
        causes.append(Cause(magnitude=mag, text=f"{label} {factor}{src}"))

    # Если эффектов нет, причина — сам баланс запаса.
    if not causes:
        causes.append(Cause(magnitude=abs(r - 1.0),
                            text=f"запас {r*100:.0f}% от целевого"))

    causes.sort(key=lambda c: c.magnitude, reverse=True)
    return PriceReport(
        node=node, good=good,
        multiplier=state.multiplier[(node, good)],
        provisioning=r, verdict=verdict, causes=causes,
    )
