"""Background shocks (MODEL.md §5.11).

A generator of random shocks (blight, epidemic, raid, bandits on the road). For
each template and each matching target, an effect is created from the template on a day
with probability 1 − exp(−rate). The RNG is counter-based (rng.py), so undoing and replaying
a day gives the same shocks. The GM turns the generator off (pass no templates or an empty
list).
"""

from __future__ import annotations

import math
from dataclasses import dataclass

from . import rng
from .effects import Decay, Effect, Op, TargetKind
from .state import SimState
from .world import World


@dataclass(frozen=True, slots=True)
class ShockTemplate:
    """Shock template: which effect, on which targets, at what rate."""

    id: str
    target_kind: TargetKind
    param: str
    op: Op
    value: float
    duration: int
    rate: float = 1.0 / 120.0      # rate per day (§7: of order 1/120)
    decay: Decay = Decay.LINEAR
    tau: float = 1.0
    good: str | None = None
    category: str | None = None
    source: str = ""               # free-text description for the log

    def probability(self) -> float:
        """Probability of firing on a day: 1 − exp(−rate)."""
        return 1.0 - math.exp(-self.rate)


def generate(world: World, state: SimState, templates) -> list[Effect]:
    """Create effects from the templates that fire on the current day (§4, step 0).

    Returns the list of new effects; templates and targets are visited in a fixed
    order (§9). Determinism comes from the counter-based RNG on (seed, day, target,
    template)."""
    new: list[Effect] = []
    day = state.day
    prob_cache: dict[str, float] = {}
    for tpl in templates:
        targets = world.node_ids() if tpl.target_kind is TargetKind.NODE else world.edge_ids()
        p = prob_cache.setdefault(tpl.id, tpl.probability())
        for target_id in targets:
            if not rng.chance(world.seed, day, p, target_id, tpl.id):
                continue
            new.append(Effect(
                param=tpl.param,
                target_kind=tpl.target_kind,
                target_id=target_id,
                op=tpl.op,
                value=tpl.value,
                start_day=day,
                duration=tpl.duration,
                decay=tpl.decay,
                tau=tpl.tau,
                good=tpl.good,
                category=tpl.category,
                source=tpl.source or tpl.id,
            ))
    return new
