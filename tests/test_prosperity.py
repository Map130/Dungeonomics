"""Prosperity (§5.8): convergence to a bounded point and the natural ceiling.

Prosperity is a slow variable with negative feedback: growth of π
raises demand (§5.3), which lowers provisioning r and pulls π* down. So
π must not run away (the anti-runaway loop §5.10).
"""

from ddeco.goods import Catalog, GoodParams
from ddeco.state import blank_state
from ddeco.tick import step_day
from ddeco.world import GlobalParams, Node, NodeKind, World

NODE = "n"


def _good(gid, cat, food, c0, kappa):
    return GoodParams(
        id=gid, category=cat, base_price=1.0,
        alpha=1.0, epsilon=0.6, beta=0.3, m_min=0.1, m_max=10.0,
        spoilage=0.0, coverage_days=10.0, c0=c0, kappa=kappa,
        per_capita=1.0, in_food_basket=food, basket_weight=1.0,
    )


def _world(grain_cap, cloth_cap, population=100.0):
    catalog = Catalog({
        "grain": _good("grain", "food", True, 0.8, 0.3),
        "cloth": _good("cloth", "common", False, 0.4, 1.0),
    })
    node = Node(
        id=NODE, kind=NodeKind.CITY, population=population, prosperity0=0.3,
        capacity={"grain": grain_cap, "cloth": cloth_cap},
    )
    return World(catalog=catalog, nodes={NODE: node}, edges={}, params=GlobalParams())


def _run(world, days=2000):
    st = blank_state(world)
    for g in world.catalog.ids():
        # start from the stock equilibrium to catch the π dynamics specifically
        st.stock[(NODE, g)] = world.catalog[g].coverage_days * world.nodes[NODE].population
    for _ in range(days):
        step_day(world, st)
    return st


def test_prosperity_converges_and_bounded():
    world = _world(grain_cap=150.0, cloth_cap=120.0)
    st = _run(world)
    pi = st.prosperity[NODE]
    assert 0.0 <= pi <= 1.0
    # another 50 days — the change is negligible (converged)
    before = pi
    for _ in range(50):
        step_day(world, st)
    assert abs(st.prosperity[NODE] - before) < 1e-4


def test_more_capacity_gives_higher_prosperity():
    poor = _run(_world(grain_cap=60.0, cloth_cap=40.0)).prosperity[NODE]
    rich = _run(_world(grain_cap=300.0, cloth_cap=250.0)).prosperity[NODE]
    assert rich > poor


def test_prosperity_never_exceeds_one_under_glut():
    """Even with a huge production surplus π ≤ 1 (no runaway)."""
    world = _world(grain_cap=5000.0, cloth_cap=5000.0)
    st = blank_state(world)
    for _ in range(3000):
        step_day(world, st)
        assert st.prosperity[NODE] <= 1.0 + 1e-12
