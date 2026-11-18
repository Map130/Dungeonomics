"""Parametric effects and their composition (MODEL.md §3).

An effect modifies a *parameter* (node production, road difficulty k_e,
population N_i, capacity cap_e, spread σ). The resulting parameter value:

    x_eff = clamp( x_base · Π_k (1 + (mult_k − 1)·w_k) + Σ_k add_k·w_k , lo, hi )

where w_k(t) ∈ [0,1] is the decay weight. The multiplier is interpolated towards 1 as it
decays, so the end of an effect causes no jump, and two effects on one parameter
combine independently. Removing one effect does not break the others.

One-shot events (caravan robbery, stock edits, prosperity hits) change the
*state* once and are not effects — see events.py.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import math


class TargetKind(str, Enum):
    NODE = "node"
    EDGE = "edge"


class Op(str, Enum):
    MULT = "mult"   # multiplier
    ADD = "add"     # addend


class Decay(str, Enum):
    STEP = "step"          # 1 until the end of the duration, then 0
    LINEAR = "linear"      # max(0, 1 − age/duration)
    EXP = "exp"            # exp(−age/τ)


@dataclass(frozen=True, slots=True)
class Effect:
    """A single parametric effect (§3, §5.4 attributes).

    good / category filter which goods a node effect applies to (for the
    pcap/garrison parameters). None in both fields means a good-independent parameter
    (population, difficulty, capacity, spread).
    """

    param: str                 # "pcap" | "population" | "spread" | "difficulty" | "capacity" | "garrison"
    target_kind: TargetKind
    target_id: str             # node or arc id
    op: Op
    value: float
    start_day: int
    duration: int              # hard end of the effect (days); math.inf is allowed
    decay: Decay = Decay.STEP
    tau: float = 1.0           # time constant for exponential decay
    good: str | None = None    # filter by a specific good
    category: str | None = None  # filter by good category
    source: str = ""           # free-text cause for the log (§5.12)

    def weight(self, day: int) -> float:
        """Decay weight w_k(t) on the given day (§3)."""
        age = day - self.start_day
        if age < 0 or age >= self.duration:
            return 0.0
        if self.decay is Decay.STEP:
            return 1.0
        if self.decay is Decay.LINEAR:
            return max(0.0, 1.0 - age / self.duration)
        # EXP
        return math.exp(-age / self.tau)

    def expired(self, day: int) -> bool:
        """The effect can be removed: its weight is 0 from this day on (§4, step 1)."""
        return day - self.start_day >= self.duration

    def matches_good(self, good_id: str | None, category: str | None) -> bool:
        """Whether the effect applies to this good (by the good/category filters)."""
        if self.good is not None:
            return self.good == good_id
        if self.category is not None:
            return self.category == category
        return True


def effective(
    base: float,
    effects: list[Effect],
    param: str,
    target_kind: TargetKind,
    target_id: str,
    day: int,
    *,
    good_id: str | None = None,
    category: str | None = None,
    lo: float = 0.0,
    hi: float = math.inf,
) -> float:
    """Effective parameter value after all applicable effects (§3).

    The order of effects does not affect the result: multipliers multiply,
    addends add up. For determinism the addends are summed in a fixed
    order (the list order — controlled by the caller).
    """
    mult = 1.0
    add = 0.0
    for e in effects:
        if (
            e.param != param
            or e.target_kind is not target_kind
            or e.target_id != target_id
            or not e.matches_good(good_id, category)
        ):
            continue
        w = e.weight(day)
        if w == 0.0:
            continue
        if e.op is Op.MULT:
            mult *= 1.0 + (e.value - 1.0) * w
        else:
            add += e.value * w
    # 0 · inf = 0: a zero factor closes an unlimited base instead of giving NaN.
    scaled = 0.0 if mult == 0.0 else base * mult
    return _clamp(scaled + add, lo, hi)


def _clamp(x: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, x))
