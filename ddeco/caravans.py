"""Materialisation of named caravans (MODEL.md §5.10).

A caravan is the face of a flow already under way, not a source of new goods. When
the accumulated value of the flow along an arc exceeds the threshold θ_car, the system
"materialises" the large flow into a named caravan: the cargo is the batches currently in
the arc's pipeline, the guard strength is random and depends on road danger (k_e). Looting
applies §5.9 to that arc's pipeline. The GM feed gets a warning 1–3 days before arrival.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from . import rng
from .state import SimState
from .world import World


@dataclass(frozen=True, slots=True)
class Caravan:
    edge: str
    day_departed: int
    day_arrival: int
    value: float                       # total cargo value
    guard: float                       # guard strength (0..~3), higher on dangerous roads
    cargo: tuple = field(default=())   # ((good, amount), ...) snapshot of the cargo in transit


def _cargo_snapshot(state: SimState, edge: str) -> tuple:
    totals: dict[str, float] = {}
    for slot in state.pipeline.get(edge, ()):
        for g, amt in slot.items():
            totals[g] = totals.get(g, 0.0) + amt
    return tuple(sorted(totals.items()))


def materialize(world: World, state: SimState) -> list[Caravan]:
    """Step 9: spawn caravans on arcs where the accumulated flow is ≥ θ_car.

    Deterministic: the guard strength comes from the counter-based RNG (seed, day, arc)."""
    theta = world.params.caravan_threshold
    made: list[Caravan] = []
    for e in world.edge_ids():
        accum = state.caravan_accum.get(e, 0.0)
        if accum < theta:
            continue
        edge = world.edges[e]
        # Road danger k_e scales the guard strength; noise from the counter-based RNG.
        guard = edge.difficulty * (0.5 + rng.uniform(world.seed, state.day, e, "caravan_guard"))
        tau = edge.delay_days()
        caravan = Caravan(
            edge=e,
            day_departed=state.day,
            day_arrival=state.day + tau,
            value=accum,
            guard=round(guard, 3),
            cargo=_cargo_snapshot(state, e),
        )
        made.append(caravan)
        state.caravans.append(caravan)
        state.caravan_accum[e] = accum - theta   # carry the remainder over
    return made
