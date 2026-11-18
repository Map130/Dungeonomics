"""Trade flows: arbitrage and hub delivery (MODEL.md §5.6).

Background trade without agents: if the expected price difference between nodes exceeds
the travel cost, part of the stock goes into the arc's pipeline with a delay. The decision to
ship is taken on the *expected* multiplier, accounting for goods already in transit Q
(otherwise traders would ship every day while the first batch is on the road → overflow).

Everything is computed from a synchronous snapshot of stocks at the start of step 6: first the
mandatory village deliveries to the hub, then arbitrage on the remainder. The order of arcs/goods
is fixed (§9), so the result is reproducible.
"""

from __future__ import annotations

from . import formulas as F
from .state import SimState
from .world import World


def add_to_pipeline(state: SimState, edge_id: str, tau: int, good: str, amount: float) -> None:
    """Put cargo into the arc's pipeline with delay τ (arrives in τ days).

    Index τ−1: the arrival step (§4.2) takes index 0 at the start of each day, so
    cargo at index τ−1 is taken exactly τ days later."""
    slots = state.pipeline.setdefault(edge_id, [])
    idx = tau - 1
    while len(slots) <= idx:
        slots.append({})
    slots[idx][good] = slots[idx].get(good, 0.0) + amount


def hub_deliveries(world: World, state: SimState, produced: dict) -> None:
    """Mandatory delivery from a village to its hub city: a share η_v of production regardless
    of the price difference (§5.6). Arbitrage then works on the remainder."""
    for n in world.node_ids():
        node = world.nodes[n]
        if node.hub is None:
            continue
        edge_id = f"{n}__{node.hub}"
        edge = world.edges.get(edge_id)
        if edge is None:
            continue  # no village→hub road — skip the delivery
        tau = edge.delay_days()
        for g in world.catalog.ids():
            p = produced.get((n, g), 0.0)
            if p <= 0.0:
                continue
            amount = min(node.hub_share * p, state.stock[(n, g)])
            if amount <= 0.0:
                continue
            state.stock[(n, g)] -= amount
            add_to_pipeline(state, edge_id, tau, g, amount)
            state.caravan_accum[edge_id] = state.caravan_accum.get(edge_id, 0.0) + world.catalog[g].base_price * amount


def _expected_multiplier(world: World, state: SimState, node: str, good: str, star: float) -> float:
    """m̂ = clamp(((s + Q)/s*)^(−α), m_min, m_max) — accounting for goods in transit to the node."""
    good_p = world.catalog[good]
    if star <= 0.0:
        return good_p.m_min
    q = state.in_transit_to(world, node, good)
    r = (state.stock[(node, good)] + q) / star
    if r <= 0.0:
        return good_p.m_max
    return F.clamp(r ** (-good_p.alpha), good_p.m_min, good_p.m_max)


def arbitrage(world: World, state: SimState, star: dict) -> None:
    """Move stock along arcs where the gain exceeds the travel cost (§5.6)."""
    p = world.params
    nodes = world.node_ids()
    goods = world.catalog.ids()

    # Expected multipliers from the start-of-step snapshot.
    mhat = {
        (n, g): _expected_multiplier(world, state, n, g, star[(n, g)])
        for n in nodes for g in goods
    }

    # Available for export (the untouchable reserve ρ_res·s*).
    avail = {
        (n, g): max(0.0, state.stock[(n, g)] - p.rho_reserve * star[(n, g)])
        for n in nodes for g in goods
    }

    # Desired flow along each arc and good (u_eg).
    u: dict[tuple[str, str], float] = {}
    for e in world.edge_ids():
        edge = world.edges[e]
        gamma = p.c_gamma * edge.travel_time * edge.difficulty
        for g in goods:
            good_p = world.catalog[g]
            delta_over_p0 = mhat[(edge.dst, g)] - mhat[(edge.src, g)] - gamma * good_p.weight
            if delta_over_p0 <= 0.0:
                continue
            flow = p.kappa_trade * F.clamp(delta_over_p0, 0.0, 1.0) * avail[(edge.src, g)]
            if flow > 0.0:
                u[(e, g)] = flow

    # ψ_ig: do not ship out more than is available, summing over outgoing arcs (per good).
    psi: dict[tuple[str, str], float] = {}
    for n in nodes:
        out_edges = world.outgoing(n)
        for g in goods:
            demand_sum = sum(u.get((e, g), 0.0) for e in out_edges)
            a = avail[(n, g)]
            psi[(n, g)] = min(1.0, a / demand_sum) if demand_sum > 0.0 else 1.0

    # φ_e: road capacity limit (summing volume over goods).
    for e in world.edge_ids():
        edge = world.edges[e]
        vol_sum = sum(
            world.catalog[g].volume * psi[(edge.src, g)] * u.get((e, g), 0.0)
            for g in goods
        )
        phi = min(1.0, edge.capacity / vol_sum) if vol_sum > 0.0 else 1.0
        tau = edge.delay_days()
        for g in goods:
            ueg = u.get((e, g), 0.0)
            if ueg <= 0.0:
                continue
            f = phi * psi[(edge.src, g)] * ueg
            if f <= 0.0:
                continue
            state.stock[(edge.src, g)] -= f
            add_to_pipeline(state, e, tau, g, f)
            state.caravan_accum[e] = state.caravan_accum.get(e, 0.0) + world.catalog[g].base_price * f


def dispatch(world: World, state: SimState, star: dict, produced: dict) -> None:
    """The whole step 6: mandatory hub deliveries first, then arbitrage."""
    hub_deliveries(world, state, produced)
    arbitrage(world, state, star)
