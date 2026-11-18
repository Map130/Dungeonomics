"""Reasons log (MODEL.md §5.12).

For every price change the system names a cause. The chain is:
    price ← provisioning r ← {production, demand, flows, spoilage}
          ← {active effects, road difficulty}.

The report gives the node's provisioning and the main causes ranked by contribution,
with the effect's source. A contribution is estimated by the relative deviation the
effect induces in its parameter (no exact counterfactual in v1; the graph is small,
this can be strengthened later).

The core emits **structured** causes only (D5): a `kind` plus parameters. Rendering to
text (and translation) is the UI's job, so the core contains no natural-language strings.
"""

from __future__ import annotations

from dataclasses import dataclass

from .effects import Effect, Op, TargetKind
from .state import SimState
from .world import World

# Verdict codes (r = s/s*): below 0.9 — shortage, above 1.1 — surplus.
SHORTAGE, SURPLUS, NORMAL = "shortage", "surplus", "normal"

# Cause kinds.
NODE_EFFECT = "node_effect"   # an effect on a node parameter (pcap, garrison, population, ...)
EDGE_EFFECT = "edge_effect"   # an effect on an inbound road (difficulty, capacity)
STOCK = "stock"               # no active effects: the stock balance itself (value = r)


@dataclass(frozen=True, slots=True)
class Cause:
    kind: str           # NODE_EFFECT | EDGE_EFFECT | STOCK
    magnitude: float    # size of the contribution (for ranking)
    value: float        # MULT: current factor; ADD: current addend; STOCK: r
    op: str = ""        # "mult" | "add" for effects, "" for STOCK
    param: str = ""     # effect parameter
    src: str = ""       # EDGE_EFFECT: road endpoints
    dst: str = ""
    source: str = ""    # free-text origin of the effect as given by the GM / template


@dataclass(frozen=True, slots=True)
class PriceReport:
    node: str
    good: str
    multiplier: float
    provisioning: float      # r = s/s*
    verdict: str             # SHORTAGE | SURPLUS | NORMAL
    causes: list[Cause]

    def top(self, n: int = 3) -> list[Cause]:
        return self.causes[:n]


def _effect_factor(e: Effect, day: int) -> tuple[float, float]:
    """Current contribution of an effect: (magnitude, value)."""
    w = e.weight(day)
    if e.op is Op.MULT:
        factor = 1.0 + (e.value - 1.0) * w
        return abs(factor - 1.0), factor
    return abs(e.value * w), e.value * w


def explain_price(world: World, state: SimState, node: str, good: str) -> PriceReport:
    """Explain the current price multiplier of a good at a node (§5.12)."""
    g = world.catalog[good]
    day = state.day
    s = state.stock[(node, good)]

    # Provisioning r uses the base demand (without effects — for the verdict).
    from . import formulas as F
    pop = world.nodes[node].population
    demand = F.base_demand(g, pop, state.prosperity[node], world.params.pi_ref,
                           world.nodes[node].garrison.get(good, 0.0))
    star = F.target_stock(g, demand)
    r = s / star if star > 0.0 else float("inf")
    verdict = SHORTAGE if r < 0.9 else SURPLUS if r > 1.1 else NORMAL

    inbound = set(world.incoming(node))
    causes: list[Cause] = []
    for e in state.effects:
        if e.weight(day) == 0.0:
            continue
        src = dst = ""
        if e.target_kind is TargetKind.NODE and e.target_id == node:
            if e.param in ("pcap", "garrison") and not e.matches_good(good, g.category):
                continue
            kind = NODE_EFFECT
        elif e.target_kind is TargetKind.EDGE and e.target_id in inbound and e.param in ("difficulty", "capacity"):
            kind = EDGE_EFFECT
            edge = world.edges[e.target_id]
            src, dst = edge.src, edge.dst
        else:
            continue
        mag, value = _effect_factor(e, day)
        causes.append(Cause(kind=kind, magnitude=mag, value=value, op=e.op.value,
                            param=e.param, src=src, dst=dst, source=e.source))

    # With no effects, the cause is the stock balance itself.
    if not causes:
        causes.append(Cause(kind=STOCK, magnitude=abs(r - 1.0), value=r))

    causes.sort(key=lambda c: c.magnitude, reverse=True)
    return PriceReport(
        node=node, good=good,
        multiplier=state.multiplier[(node, good)],
        provisioning=r, verdict=verdict, causes=causes,
    )
