"""Начальное состояние и прогрев мира (MODEL.md §5.14).

Прогрев («warm start») — прогон без событий до сходимости, чтобы мир стартовал в
равновесии, а не дёргался первые недели.

Авто-калибровка стартового множителя: без торговли равновесие одного узла решает
    m^(β+ε) = D / Pcap
(производство = потребление). Берём это m и соответствующий запас s = s*·m^(1/α)
как стартовую точку — она близка к многоузловому равновесию, так что прогрев
короткий. Торговля и процветание дальше доводят до стационара.
"""

from __future__ import annotations

from . import formulas as F
from .state import SimState, blank_state
from .tick import step_day
from .world import World


def equilibrium_multiplier(beta: float, eps: float, demand: float, pcap: float) -> float:
    """Решение m^(β+ε) = D/Pcap. Если Pcap = 0, цена уходит вверх (нет производства)."""
    if pcap <= 0.0:
        return float("inf")
    ratio = demand / pcap
    return ratio ** (1.0 / (beta + eps))


def seed_state(world: World) -> SimState:
    """Стартовое состояние из авто-калибровки равновесия без торговли (§5.14)."""
    st = blank_state(world)
    pi_ref = world.params.pi_ref
    for n in world.node_ids():
        node = world.nodes[n]
        pros = node.prosperity0
        for g in world.catalog.ids():
            good = world.catalog[g]
            demand = F.base_demand(good, node.population, pros, pi_ref, node.garrison.get(g, 0.0))
            star = F.target_stock(good, demand)
            m_eq = equilibrium_multiplier(good.beta, good.epsilon, demand, node.pcap(g))
            m_eq = F.clamp(m_eq, good.m_min, good.m_max)
            # r из m*: m = r^(-α) ⇒ r = m^(-1/α); s = r·s*
            r = m_eq ** (-1.0 / good.alpha) if good.alpha > 0.0 else 1.0
            st.multiplier[(n, g)] = m_eq
            st.stock[(n, g)] = r * star
    return st


def warmup(world: World, max_days: int = 2000, tol: float = 1e-4) -> SimState:
    """Прогреть мир: прогон без событий до сходимости множителей.

    Возвращает установившееся состояние. Сходимость — когда максимальное за день
    изменение множителя падает ниже tol."""
    st = seed_state(world)
    for _ in range(max_days):
        prev = dict(st.multiplier)
        step_day(world, st)
        delta = max(abs(st.multiplier[k] - prev[k]) for k in prev)
        if delta < tol:
            break
    return st
