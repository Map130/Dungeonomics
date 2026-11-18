"""Stability autotest §8.1: the single-node price–stock–production loop.

For each category from the §7/§6.3 table, a run with a ±20% perturbation of the starting
stock. Acceptance: by day 300 |s/s* − 1| < 0.01 and the multiplier is not at the corridor
edge. The numbers come from the numerically verified §6.3 table.
"""

import pytest

from ddeco.tick import step_day
from tests.single_node import GOOD, NODE, single_node_world, perturbed_state

# (name, α, β, ε, T) — the §7 / §6.3 table
CATEGORIES = [
    ("necessity", 0.5, 0.3, 0.2, 10.0),
    ("common", 1.0, 0.3, 0.6, 10.0),
    ("luxury", 1.5, 0.3, 1.5, 14.0),
]


@pytest.mark.parametrize("name,alpha,beta,eps,T", CATEGORIES)
@pytest.mark.parametrize("ratio", [0.8, 1.2])
def test_converges_to_equilibrium(name, alpha, beta, eps, T, ratio):
    world = single_node_world(alpha, beta, eps, T)
    state, star = perturbed_state(world, ratio)

    for _ in range(300):
        step_day(world, state)

    s = state.stock[(NODE, GOOD)]
    m = state.multiplier[(NODE, GOOD)]
    good = world.catalog[GOOD]

    assert abs(s / star - 1.0) < 0.01, f"{name}: s/s*={s/star:.4f} did not converge"
    # The multiplier did not stick to the corridor edge.
    assert good.m_min + 1e-3 < m < good.m_max - 1e-3, f"{name}: m={m:.4f} at the edge"
    # In equilibrium with Pcap=D the multiplier is ≈ 1.
    assert abs(m - 1.0) < 0.02, f"{name}: m={m:.4f} far from 1"


@pytest.mark.parametrize("name,alpha,beta,eps,T", CATEGORIES)
def test_no_overshoot_past_corridor(name, alpha, beta, eps, T):
    """Over the whole run the multiplier stays inside the corridor (there must be no clipping)."""
    world = single_node_world(alpha, beta, eps, T)
    state, _ = perturbed_state(world, 0.8)
    good = world.catalog[GOOD]
    for _ in range(300):
        step_day(world, state)
        m = state.multiplier[(NODE, GOOD)]
        assert good.m_min <= m <= good.m_max


def test_equilibrium_is_fixed_point():
    """Start exactly at equilibrium (s=s*, m=1) — the state must not drift."""
    world = single_node_world(1.0, 0.3, 0.6, 10.0)
    state, star = perturbed_state(world, 1.0)
    for _ in range(100):
        step_day(world, state)
    assert abs(state.stock[(NODE, GOOD)] / star - 1.0) < 1e-9
    assert abs(state.multiplier[(NODE, GOOD)] - 1.0) < 1e-9
