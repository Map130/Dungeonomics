"""Хелпер: одноузловой односортовой мир для тестов и калибровки (§6, §8.1).

Изолирует контур цена–запас–производство: без торговли, без процветания
(c0 = 1 → спрос не зависит от π), без порчи (δ = 0, как в линеаризации §6.1).
При Pcap = D равновесие — ровно s = s*, m = 1 (§5.14).
"""

from __future__ import annotations

from ddeco.goods import Catalog, GoodParams
from ddeco.state import blank_state
from ddeco.world import GlobalParams, Node, NodeKind, World

GOOD = "x"
NODE = "n"


def single_node_world(alpha: float, beta: float, eps: float, T: float, *, lam: float = 0.3,
                      demand: float = 100.0) -> World:
    good = GoodParams(
        id=GOOD, category="test", base_price=1.0,
        alpha=alpha, epsilon=eps, beta=beta,
        m_min=0.1, m_max=10.0,          # широкий коридор: не клипуем в тесте
        spoilage=0.0,                   # контур §6.1 без порчи
        coverage_days=T,
        c0=1.0, kappa=0.0,              # спрос не зависит от π
        per_capita=1.0,
    )
    node = Node(
        id=NODE, kind=NodeKind.CITY, population=demand, prosperity0=0.5,
        capacity={GOOD: demand},        # Pcap = D → равновесие s=s*, m=1
    )
    return World(
        catalog=Catalog({GOOD: good}),
        nodes={NODE: node},
        edges={},
        params=GlobalParams(lam=lam),
    )


def perturbed_state(world: World, ratio: float):
    """Состояние со стартовым запасом ratio·s*, m = 1."""
    st = blank_state(world)
    good = world.catalog[GOOD]
    demand = world.nodes[NODE].population * good.per_capita
    star = good.coverage_days * demand
    st.stock[(NODE, GOOD)] = ratio * star
    st.multiplier[(NODE, GOOD)] = 1.0
    return st, star
