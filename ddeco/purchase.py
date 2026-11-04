"""Влияние покупки/продажи игрока на цену (MODEL.md §5.7).

Крупная сделка сдвигает цену на каждой единице, игрок платит среднюю по кривой, а
не цену с ценника — защита от скупки. Предельный множитель после u единиц:
m·(s/(s−u))^α при покупке. Коридор [m_min, m_max] применяется в каждой точке,
поэтому считаем численным суммированием по шагам, а не замкнутой формулой. После
сделки m устанавливается равным предельному множителю на последней единице, дальше
работает обычное сглаживание.
"""

from __future__ import annotations

from dataclasses import dataclass

from . import formulas as F
from .state import SimState
from .world import World


@dataclass(frozen=True, slots=True)
class Trade:
    good: str
    qty: float
    total: float       # суммарная цена сделки (с учётом спреда)
    avg_price: float   # средняя цена за единицу
    new_multiplier: float


def _price_multiplier(m0: float, s0: float, u: float, alpha: float, m_min: float, m_max: float) -> float:
    """Предельный множитель после u изъятых единиц (u>0 покупка, u<0 продажа)."""
    denom = s0 - u
    if denom <= 0.0:
        return m_max
    return F.clamp(m0 * (s0 / denom) ** alpha, m_min, m_max)


def quote(world: World, state: SimState, node: str, good: str, qty: float,
          *, sell: bool = False, steps: int = 256) -> Trade:
    """Посчитать сделку, НЕ меняя состояние (для предпросмотра цены мастеру)."""
    if qty <= 0.0:
        raise ValueError("qty должно быть > 0")
    g = world.catalog[good]
    s0 = state.stock[(node, good)]
    m0 = state.multiplier[(node, good)]
    spread = world.nodes[node].spread
    sign = -1.0 if sell else 1.0   # покупка изымает (u>0), продажа добавляет (u<0)
    if not sell and qty >= s0:
        raise ValueError("нельзя купить весь запас (q < s)")

    du = qty / steps
    total = 0.0
    for i in range(steps):
        u = sign * (i + 0.5) * du                      # середина шага
        m = _price_multiplier(m0, s0, u, g.alpha, g.m_min, g.m_max)
        unit = g.base_price * m * (1.0 - spread if sell else 1.0 + spread)
        total += unit * du
    new_m = _price_multiplier(m0, s0, sign * qty, g.alpha, g.m_min, g.m_max)
    return Trade(good=good, qty=qty, total=total, avg_price=total / qty, new_multiplier=new_m)


def apply(world: World, state: SimState, node: str, good: str, qty: float,
          *, sell: bool = False, steps: int = 256) -> Trade:
    """Провести сделку: обновить запас и множитель. Возвращает детали сделки."""
    trade = quote(world, state, node, good, qty, sell=sell, steps=steps)
    delta = qty if sell else -qty
    state.stock[(node, good)] = max(0.0, state.stock[(node, good)] + delta)
    state.multiplier[(node, good)] = trade.new_multiplier
    return trade
