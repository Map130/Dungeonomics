"""Simulation state (MODEL.md §2). It evolves on its own; effects change only
the world's parameters, not this.

Holds stocks s_ig, price multipliers m_ig, prosperity π_i, pipelines of goods in
transit q[e][d] and the list of active effects. Everything needed to undo/replay a day
(§7, §8.4) lives here — a snapshot is taken with clone().

Arc pipeline: pipeline[edge_id] — a list of dicts {good_id: amount}, where index p
means "arrives in p arrival steps". The arrival step (§4, step 2) takes
index 0 and shifts the list. Dispatch with delay τ puts the cargo at index τ−1
(see trade.py).
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .effects import Effect
from .world import World


Key = tuple[str, str]  # (node_id, good_id)


@dataclass(slots=True)
class SimState:
    day: int = 0
    stock: dict[Key, float] = field(default_factory=dict)        # s_ig
    multiplier: dict[Key, float] = field(default_factory=dict)   # m_ig
    prosperity: dict[str, float] = field(default_factory=dict)   # π_i
    pipeline: dict[str, list[dict[str, float]]] = field(default_factory=dict)  # q[e][d]
    effects: list[Effect] = field(default_factory=list)
    # Accumulated value of the flow along an arc since the last materialisation (§5.10).
    caravan_accum: dict[str, float] = field(default_factory=dict)
    # Materialised caravans (the GM feed, §5.10). Elements are frozen dataclasses.
    caravans: list = field(default_factory=list)

    def clone(self) -> "SimState":
        """A deep snapshot for undo/replay. Effect/Caravan are immutable → lists
        are copied shallowly; the rest by hand, to avoid copy.deepcopy and
        keep the behaviour explicit and deterministic."""
        return SimState(
            day=self.day,
            stock=dict(self.stock),
            multiplier=dict(self.multiplier),
            prosperity=dict(self.prosperity),
            pipeline={e: [dict(slot) for slot in slots] for e, slots in self.pipeline.items()},
            effects=list(self.effects),
            caravan_accum=dict(self.caravan_accum),
            caravans=list(self.caravans),
        )

    def in_transit_to(self, world: World, node_id: str, good_id: str) -> float:
        """Q_jg: total good_id in transit to node node_id (§5.6).

        Summed in a fixed order of arcs and slots (§9)."""
        total = 0.0
        for e in world.edge_ids():
            if world.edges[e].dst != node_id:
                continue
            for slot in self.pipeline.get(e, ()):
                total += slot.get(good_id, 0.0)
        return total


def blank_state(world: World) -> SimState:
    """Empty state: zero stocks, m = 1, π = π0, empty pipelines."""
    st = SimState(day=0)
    for n in world.node_ids():
        node = world.nodes[n]
        st.prosperity[n] = node.prosperity0
        for g in world.catalog.ids():
            st.stock[(n, g)] = 0.0
            st.multiplier[(n, g)] = 1.0
    for e in world.edge_ids():
        st.pipeline[e] = []
        st.caravan_accum[e] = 0.0
    return st
