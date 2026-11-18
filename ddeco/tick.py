"""Orchestration of one simulated day (MODEL.md §4).

The order of steps is fixed and must not change. All nodes and arcs are processed from
a synchronous snapshot of the state at the start of the respective step, so the traversal
order does not affect the result.

    0. Events (effects are added from outside before calling step_day)
    1. Recompute effective parameters; expired effects are removed
    2. Arrivals (pipelines shift, cargo with d=0 goes into the stock)
    3. Production
    4. Consumption
    5. Spoilage
    6. Trade dispatch
    7. Prosperity
    8. Price
    9. Caravan materialisation
    10. Reasons log

Trade (6), prosperity (7), caravans (9) and the log (10) are wired in by their own
modules.
"""

from __future__ import annotations

from . import caravans as caravans_mod
from . import formulas as F
from . import shocks as shocks_mod
from . import trade
from .trade import effective_capacity, effective_difficulty  # noqa: F401 (re-export)
from .effects import TargetKind, effective
from .state import SimState
from .world import World


# --- Effective parameters (base from the world + active effects, §3) -----------

def effective_pcap(world: World, state: SimState, node: str, good: str) -> float:
    base = world.nodes[node].pcap(good)
    return effective(
        base, state.effects, "pcap", TargetKind.NODE, node, state.day,
        good_id=good, category=world.catalog[good].category, lo=0.0,
    )


def effective_population(world: World, state: SimState, node: str) -> float:
    base = world.nodes[node].population
    return effective(
        base, state.effects, "population", TargetKind.NODE, node, state.day, lo=0.0,
    )


def effective_garrison(world: World, state: SimState, node: str, good: str) -> float:
    base = world.nodes[node].garrison.get(good, 0.0)
    return effective(
        base, state.effects, "garrison", TargetKind.NODE, node, state.day,
        good_id=good, category=world.catalog[good].category, lo=0.0,
    )


def effective_spread(world: World, state: SimState, node: str) -> float:
    base = world.nodes[node].spread
    return effective(
        base, state.effects, "spread", TargetKind.NODE, node, state.day, lo=0.0,
    )


# --- Steps of the day ----------------------------------------------------------

def _drop_expired(state: SimState) -> None:
    """Step 1: remove effects whose weight is 0 from now on."""
    day = state.day
    state.effects = [e for e in state.effects if not e.expired(day)]


def _arrivals(world: World, state: SimState) -> None:
    """Step 2: cargo with d=0 arrives at the destination node, the pipeline shifts."""
    for e in world.edge_ids():
        slots = state.pipeline.get(e)
        if not slots:
            continue
        arrived = slots.pop(0)  # index 0 = arrives today; the rest shift
        dst = world.edges[e].dst
        for g, amt in arrived.items():
            state.stock[(dst, g)] = state.stock.get((dst, g), 0.0) + amt


def _production_consumption_spoilage(
    world: World, state: SimState, m0: dict, demand: dict, star: dict, produced: dict
) -> None:
    """Steps 3–5: production, consumption, spoilage from the start-of-day snapshot (m0).

    produced[(n,g)] — daily output (needed for the mandatory hub delivery, §5.6)."""
    p = world.params
    for n in world.node_ids():
        pros = state.prosperity[n]
        pop = effective_population(world, state, n)
        for g in world.catalog.ids():
            good = world.catalog[g]
            D = F.base_demand(good, pop, pros, p.pi_ref, effective_garrison(world, state, n, g))
            demand[(n, g)] = D
            star[(n, g)] = F.target_stock(good, D)

            prod = F.production(good, effective_pcap(world, state, n, g), m0[(n, g)], p.lo_prod, p.hi_prod)
            produced[(n, g)] = prod
            s = state.stock[(n, g)] + prod
            s -= F.consumption(good, D, m0[(n, g)], s)
            s = F.after_spoilage(good, s)
            state.stock[(n, g)] = s


def _prices(world: World, state: SimState, star: dict) -> None:
    """Step 8: target multiplier from provisioning and smoothing towards it."""
    lam = world.params.lam
    for n in world.node_ids():
        for g in world.catalog.ids():
            good = world.catalog[g]
            mstar = F.target_multiplier(good, state.stock[(n, g)], star[(n, g)])
            state.multiplier[(n, g)] = F.smooth_multiplier(state.multiplier[(n, g)], mstar, lam)


def _basket_terms(world: World, state: SimState, node: str, good_ids: list[str], star: dict):
    """List of (basket_weight, r) over basket goods with non-zero demand."""
    terms = []
    for g in good_ids:
        good = world.catalog[g]
        if good.basket_weight <= 0.0:
            continue
        s_star = star[(node, g)]
        if s_star <= 0.0:
            continue  # no demand — the good is not in this node's basket
        terms.append((good.basket_weight, state.stock[(node, g)] / s_star))
    return terms


def _prosperity(world: World, state: SimState, star: dict) -> None:
    """Step 7: prosperity π as a slow variable (§5.8).

    Natural ceiling: higher π → higher demand (§5.3) → lower r → lower π*, so
    π levels off where consumption eats up the production surplus."""
    p = world.params
    catalog = world.catalog
    food_ids = catalog.food_ids()
    market_ids = catalog.market_ids()
    for n in world.node_ids():
        food_fill = F.basket_fill(_basket_terms(world, state, n, food_ids, star))
        market_fill = F.basket_fill(_basket_terms(world, state, n, market_ids, star))
        target = F.prosperity_target(food_fill, market_fill, p.w_food, p.w_market)
        state.prosperity[n] = F.smooth_prosperity(state.prosperity[n], target, p.mu)


def step_day(world: World, state: SimState, shocks=()) -> SimState:
    """Advance the state by one day. Mutates state and returns it.

    shocks — a list of ShockTemplate for background shocks (§5.11); empty = generator
    off. The caller takes the undo snapshot via state.clone()."""
    if shocks:
        state.effects.extend(shocks_mod.generate(world, state, shocks))  # 0
    _drop_expired(state)                              # 1
    _arrivals(world, state)                           # 2
    m0 = dict(state.multiplier)                       # price snapshot at the start of the day
    demand: dict = {}
    star: dict = {}
    produced: dict = {}
    _production_consumption_spoilage(world, state, m0, demand, star, produced)  # 3–5
    trade.dispatch(world, state, star, produced)      # 6
    _prosperity(world, state, star)                   # 7
    _prices(world, state, star)                       # 8
    caravans_mod.materialize(world, state)            # 9
    state.day += 1
    return state


def run(world: World, state: SimState, days: int, shocks=()) -> SimState:
    """Run N days (REQUIREMENTS §5.7)."""
    for _ in range(days):
        step_day(world, state, shocks)
    return state
