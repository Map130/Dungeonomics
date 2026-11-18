"""One-shot events (MODEL.md §3, §5.9).

Unlike parametric effects, one-shot events change the *state* once
and are not part of the parameters: GM stock edits, prosperity hits, caravan
robbery. All of them are deterministic and do not use the RNG.
"""

from __future__ import annotations

from .state import SimState


def set_stock(state: SimState, node: str, good: str, amount: float) -> None:
    """The GM sets a stock by hand (a player bought out / brought in goods). REQUIREMENTS §5.8."""
    state.stock[(node, good)] = max(0.0, amount)


def add_stock(state: SimState, node: str, good: str, delta: float) -> None:
    """Shift a stock by delta (may be negative), not below zero."""
    state.stock[(node, good)] = max(0.0, state.stock.get((node, good), 0.0) + delta)


def prosperity_hit(state: SimState, node: str, rho: float) -> None:
    """Prosperity hit: π ← π·(1 − ρ) (a raid, §5.8)."""
    state.prosperity[node] *= 1.0 - rho


def loot_caravan(state: SimState, edge: str, rho: float, slot_index: int | None = None) -> float:
    """Robbery: remove a share ρ of the cargo from the arc's pipeline (§5.9).

    slot_index — a specific batch; None = all cargo in transit on the arc. Returns
    the total volume looted (the GM decides whether the players get it)."""
    slots = state.pipeline.get(edge)
    if not slots:
        return 0.0
    targets = range(len(slots)) if slot_index is None else [slot_index]
    looted = 0.0
    for i in targets:
        if i < 0 or i >= len(slots):
            continue
        for g in list(slots[i]):
            take = slots[i][g] * rho
            slots[i][g] -= take
            looted += take
    return looted
