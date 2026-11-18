"""Initial state and world warm-up (MODEL.md §5.14).

Warm-up ("warm start") is a run without events until convergence, so the world starts in
equilibrium instead of twitching for the first weeks.

Auto-calibration of the starting multiplier: without trade, a single node's equilibrium solves
    m^(β+ε) = D / Pcap
(production = consumption). We take this m and the matching stock s = s*·m^(1/α)
as the starting point — it is close to the multi-node equilibrium, so the warm-up
is short. Trade and prosperity then bring it to the steady state.
"""

from __future__ import annotations

from . import formulas as F
from .state import SimState, blank_state
from .tick import step_day
from .world import World


def equilibrium_multiplier(beta: float, eps: float, demand: float, pcap: float) -> float:
    """Solve m^(β+ε) = D/Pcap. If Pcap = 0 the price goes up (no production)."""
    if pcap <= 0.0:
        return float("inf")
    ratio = demand / pcap
    return ratio ** (1.0 / (beta + eps))


def seed_state(world: World) -> SimState:
    """Starting state from the no-trade equilibrium auto-calibration (§5.14)."""
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
            # r from m*: m = r^(-α) ⇒ r = m^(-1/α); s = r·s*
            r = m_eq ** (-1.0 / good.alpha) if good.alpha > 0.0 else 1.0
            st.multiplier[(n, g)] = m_eq
            st.stock[(n, g)] = r * star
    return st


def warmup(world: World, max_days: int = 2000, tol: float = 1e-4) -> SimState:
    """Warm the world up: run without events until the multipliers converge.

    Returns the settled state. Converged when the largest daily multiplier
    change drops below tol."""
    st = seed_state(world)
    for _ in range(max_days):
        prev = dict(st.multiplier)
        step_day(world, st)
        delta = max(abs(st.multiplier[k] - prev[k]) for k in prev)
        if delta < tol:
            break
    return st
