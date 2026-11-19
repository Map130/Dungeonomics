"""The shipped example world is stable (MODEL §8 spirit, M8 plan): after the warm-up (§5.14)
converges, no node × good with demand sits on a corridor edge (m_min / m_max).

A pinned multiplier means the good never reaches that node (m_max) or piles up unsold
(m_min) — the price no longer tells the GM anything. Exceptions must be listed in
PINNED_OK with a reason; the example world needs none: every good has a producer and a
trade path to every settlement.
"""

from pathlib import Path

from ddeco import formulas as F
from ddeco.calibration import warmup
from ddeco.loader import load_world
from ddeco.tick import step_day

WORLD = Path(__file__).resolve().parent.parent / "worlds" / "example_region.toml"
MAX_DAYS = 2000
TOL = 1e-4
EDGE = 1e-3

# (node, good) → reason. Keep empty unless a good genuinely cannot avoid the edge.
PINNED_OK: dict[tuple[str, str], str] = {}


def _settled():
    world = load_world(WORLD)
    return world, warmup(world, max_days=MAX_DAYS, tol=TOL)


def test_warmup_converges():
    world, st = _settled()
    assert st.day < MAX_DAYS, "warm-up hit max_days without converging"
    prev = dict(st.multiplier)
    step_day(world, st)
    delta = max(abs(st.multiplier[k] - prev[k]) for k in prev)
    assert delta < TOL


def test_no_multiplier_pinned_to_corridor_edge():
    world, st = _settled()
    pinned = []
    for n in world.node_ids():
        node = world.nodes[n]
        for g in world.catalog.ids():
            good = world.catalog[g]
            demand = F.base_demand(
                good, node.population, st.prosperity[n], world.params.pi_ref,
                node.garrison.get(g, 0.0),
            )
            if demand <= 0.0 or (n, g) in PINNED_OK:
                continue
            m = st.multiplier[(n, g)]
            if m - good.m_min < EDGE or good.m_max - m < EDGE:
                pinned.append((n, g, round(m, 4)))
    assert pinned == []


def test_every_good_has_a_producer():
    world = load_world(WORLD)
    for g in world.catalog.ids():
        assert any(node.pcap(g) > 0.0 for node in world.nodes.values()), g


def test_weapons_stock_fits_plate_armour():
    # R30: plate armour = 1500 gp / 15 gp = 100 weapon units, plus margin (shop "too big", D8).
    _, st = _settled()
    for city in ("redwater", "stonehaven", "greenvale"):
        assert st.stock[(city, "weapons")] > 110.0, city
