"""Autotest §8.3: runaway growth is bounded.

A node with excess production (Pcap = 1.5·D). The supply response to price (§5.2:
output falls at a low price) plus spoilage as a sink must bring the stock to a finite
ceiling, and the multiplier does not stick to the corridor edge.
"""

import math

from ddeco.goods import Catalog, GoodParams
from ddeco.state import blank_state
from ddeco.tick import step_day
from ddeco.world import GlobalParams, Node, NodeKind, World

NODE, GOOD = "n", "x"


def _world():
    good = GoodParams(
        id=GOOD, category="common", base_price=1.0,
        alpha=1.0, epsilon=0.6, beta=0.3, m_min=0.5, m_max=3.0,
        spoilage=0.002, coverage_days=10.0, c0=1.0, kappa=0.0, per_capita=1.0,
    )
    node = Node(id=NODE, kind=NodeKind.CITY, population=100.0, prosperity0=0.5,
                capacity={GOOD: 150.0})  # Pcap = 1.5·D
    return World(catalog=Catalog({GOOD: good}), nodes={NODE: node}, edges={},
                 params=GlobalParams(lam=0.3))


def test_overproduction_hits_ceiling():
    world = _world()
    st = blank_state(world)
    star = 10.0 * 100.0  # T·D
    trace = []
    for _ in range(2000):
        step_day(world, st)
        trace.append(st.stock[(NODE, GOOD)])

    final_r = trace[-1] / star
    assert final_r < 5.0, f"stock ran away: s/s*={final_r:.2f}"

    # Slope of log(s) over the last 300 days ≈ 0 (reached a plateau, not growing).
    a, b = trace[-300], trace[-1]
    slope = (math.log(b) - math.log(a)) / 300.0
    assert abs(slope) < 1e-4, f"stock is still growing, slope of log s = {slope:.2e}"

    # Production fell (m < 1), but the multiplier did not stick to the lower bound.
    m = st.multiplier[(NODE, GOOD)]
    assert world.catalog[GOOD].m_min + 1e-3 < m < 1.0, f"m={m:.3f}"
