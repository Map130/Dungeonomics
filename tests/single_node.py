"""Helper: a single-node, single-good world for tests and calibration (§6, §8.1).

Isolates the price–stock–production loop: no trade, no prosperity
(c0 = 1 → demand does not depend on π), no spoilage (δ = 0, as in the §6.1 linearisation).
With Pcap = D the equilibrium is exactly s = s*, m = 1 (§5.14).
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
        m_min=0.1, m_max=10.0,          # wide corridor: no clipping in the test
        spoilage=0.0,                   # the §6.1 loop without spoilage
        coverage_days=T,
        c0=1.0, kappa=0.0,              # demand does not depend on π
        per_capita=1.0,
    )
    node = Node(
        id=NODE, kind=NodeKind.CITY, population=demand, prosperity0=0.5,
        capacity={GOOD: demand},        # Pcap = D → equilibrium s=s*, m=1
    )
    return World(
        catalog=Catalog({GOOD: good}),
        nodes={NODE: node},
        edges={},
        params=GlobalParams(lam=lam),
    )


def perturbed_state(world: World, ratio: float):
    """State with a starting stock of ratio·s*, m = 1."""
    st = blank_state(world)
    good = world.catalog[GOOD]
    demand = world.nodes[NODE].population * good.per_capita
    star = good.coverage_days * demand
    st.stock[(NODE, GOOD)] = ratio * star
    st.multiplier[(NODE, GOOD)] = 1.0
    return st, star
