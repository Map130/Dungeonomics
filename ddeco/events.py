"""Разовые события (MODEL.md §3, §5.9).

В отличие от параметрических эффектов, разовые события меняют *состояние* один раз
и в параметры не входят: правка запасов мастером, удар по процветанию, грабёж
каравана. Все они детерминированы и не зависят от RNG.
"""

from __future__ import annotations

from .state import SimState


def set_stock(state: SimState, node: str, good: str, amount: float) -> None:
    """Мастер вручную задаёт запас (игрок скупил/подвёз товар). REQUIREMENTS §5.8."""
    state.stock[(node, good)] = max(0.0, amount)


def add_stock(state: SimState, node: str, good: str, delta: float) -> None:
    """Сдвинуть запас на delta (может быть отрицательным), не ниже нуля."""
    state.stock[(node, good)] = max(0.0, state.stock.get((node, good), 0.0) + delta)


def prosperity_hit(state: SimState, node: str, rho: float) -> None:
    """Удар по процветанию: π ← π·(1 − ρ) (набег, §5.8)."""
    state.prosperity[node] *= 1.0 - rho


def loot_caravan(state: SimState, edge: str, rho: float, slot_index: int | None = None) -> float:
    """Ограбление: убрать долю ρ груза из конвейера дуги (§5.9).

    slot_index — конкретная партия; None = весь груз в пути по дуге. Возвращает
    суммарный объём награбленного (мастер решает, передать ли игрокам)."""
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
