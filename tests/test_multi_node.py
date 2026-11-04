"""Автотест устойчивости §8.2 + проверка арбитража (§5.6).

Многоузловой мир без событий должен сходиться к стационару: при суммарном
Pcap = суммарный спрос и дешёвом транспорте арбитраж стягивает цены к m≈1,
запасы остаются в [0.5, 2]·s*, потоки по дорогам не колеблются.
"""

import statistics

from ddeco.goods import Catalog, GoodParams
from ddeco.state import blank_state
from ddeco.tick import step_day
from ddeco.world import Edge, GlobalParams, Node, NodeKind, World

GOOD = "x"


def _good():
    return GoodParams(
        id=GOOD, category="test", base_price=1.0,
        alpha=1.0, epsilon=0.6, beta=0.3, m_min=0.1, m_max=10.0,
        spoilage=0.0, coverage_days=10.0, c0=1.0, kappa=0.0, per_capita=1.0,
    )


def _connected_world(caps: list[float], *, c_gamma=0.001, capacity=1e9, pop=100.0) -> World:
    """Полносвязный граф с дешёвым транспортом. caps задаёт Pcap по узлам."""
    names = [f"n{i}" for i in range(len(caps))]
    nodes = {
        name: Node(id=name, kind=NodeKind.CITY, population=pop, prosperity0=0.5,
                   capacity={GOOD: cap})
        for name, cap in zip(names, caps)
    }
    edges = {}
    for a in names:
        for b in names:
            if a == b:
                continue
            eid = f"{a}__{b}"
            edges[eid] = Edge(id=eid, src=a, dst=b, travel_time=1, difficulty=1.0, capacity=capacity)
    params = GlobalParams(lam=0.3, c_gamma=c_gamma)
    return World(catalog=Catalog({GOOD: _good()}), nodes=nodes, edges=edges, params=params)


def _start(world, ratios):
    st = blank_state(world)
    for name, ratio in zip(world.node_ids(), ratios):
        star = world.catalog[GOOD].coverage_days * world.nodes[name].population
        st.stock[(name, GOOD)] = ratio * star
    return st


def test_arbitrage_equalizes_prices():
    # Суммарный Pcap = суммарный спрос (4·100 = 400); возмущение ±20% по узлам.
    world = _connected_world([120.0, 80.0, 110.0, 90.0])
    st = _start(world, [0.8, 1.2, 0.9, 1.1])
    for _ in range(300):
        step_day(world, st)

    ms = [st.multiplier[(n, GOOD)] for n in world.node_ids()]
    assert max(abs(m - 1.0) for m in ms) < 0.05, f"цены не сошлись к 1: {ms}"

    for n in world.node_ids():
        star = world.catalog[GOOD].coverage_days * world.nodes[n].population
        r = st.stock[(n, GOOD)] / star
        assert 0.5 <= r <= 2.0, f"{n}: s/s*={r:.3f} вне [0.5,2]"


def test_flows_do_not_oscillate():
    world = _connected_world([120.0, 80.0, 110.0, 90.0])
    st = _start(world, [0.8, 1.2, 0.9, 1.1])
    for _ in range(300):
        step_day(world, st)
    # Амплитуда суммарного товара в пути за последние 100 дней < 5% среднего.
    series = []
    for _ in range(100):
        step_day(world, st)
        series.append(sum(v for e in world.edge_ids() for slot in st.pipeline.get(e, ()) for v in slot.values()))
    mean = statistics.fmean(series)
    assert mean > 0, "торговля должна идти (узлы несбалансированы по Pcap)"
    assert statistics.pstdev(series) / mean < 0.05, "потоки колеблются"


def test_trade_narrows_gap_vs_no_trade():
    """С торговлей разброс цен меньше, чем без неё (дорогой транспорт глушит обмен)."""
    caps = [140.0, 60.0, 130.0, 70.0]
    with_trade = _connected_world(caps, c_gamma=0.001)
    no_trade = _connected_world(caps, c_gamma=100.0)  # транспорт запретительно дорог
    sw = _start(with_trade, [1.0, 1.0, 1.0, 1.0])
    sn = _start(no_trade, [1.0, 1.0, 1.0, 1.0])
    for _ in range(300):
        step_day(with_trade, sw)
        step_day(no_trade, sn)

    def spread(world, st):
        ms = [st.multiplier[(n, GOOD)] for n in world.node_ids()]
        return max(ms) - min(ms)

    assert spread(with_trade, sw) < spread(no_trade, sn)
