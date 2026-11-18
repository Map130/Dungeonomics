"""Материализация именных караванов (MODEL.md §5.10).

Караван — это лицо уже идущего потока, а не источник новых товаров. Когда
накопленная стоимость потока по дуге превышает порог θ_car, система «материализует»
крупный поток в именной караван: груз — партии, лежащие сейчас в конвейере дуги,
сила охраны случайна и зависит от опасности дороги (k_e). Ограбление применяет §5.9
к конвейеру этой дуги. В ленту мастеру пишется предупреждение за 1–3 дня до прибытия.
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
    value: float                       # суммарная стоимость груза
    guard: float                       # сила охраны (0..~3), выше на опасных дорогах
    cargo: tuple = field(default=())   # ((good, amount), ...) снимок груза в пути


def _cargo_snapshot(state: SimState, edge: str) -> tuple:
    totals: dict[str, float] = {}
    for slot in state.pipeline.get(edge, ()):
        for g, amt in slot.items():
            totals[g] = totals.get(g, 0.0) + amt
    return tuple(sorted(totals.items()))


def materialize(world: World, state: SimState) -> list[Caravan]:
    """Шаг 9: породить караваны по дугам, где накоплен поток ≥ θ_car.

    Детерминированно: сила охраны берётся из счётчикового RNG (seed, день, дуга)."""
    theta = world.params.caravan_threshold
    made: list[Caravan] = []
    for e in world.edge_ids():
        accum = state.caravan_accum.get(e, 0.0)
        if accum < theta:
            continue
        edge = world.edges[e]
        # Опасность дороги k_e масштабирует силу охраны; шум из счётчикового RNG.
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
        state.caravan_accum[e] = accum - theta   # остаток переносим
    return made
