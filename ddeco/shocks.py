"""Фоновые шоки (MODEL.md §5.11).

Генератор случайных шоков (неурожай, эпидемия, налёт, бандиты на дороге). Для
каждого шаблона и каждой подходящей цели в день с вероятностью 1 − exp(−rate)
создаётся эффект из шаблона. RNG счётчиковый (rng.py), поэтому откат и повтор дня
дают те же шоки. Генератор отключается мастером (не передавать шаблоны или пустой
список).
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
    """Шаблон шока: какой эффект, на какие цели, с какой интенсивностью."""

    id: str
    target_kind: TargetKind
    param: str
    op: Op
    value: float
    duration: int
    rate: float = 1.0 / 120.0      # интенсивность в день (§7: порядок 1/120)
    decay: Decay = Decay.LINEAR
    tau: float = 1.0
    good: str | None = None
    category: str | None = None
    source: str = ""               # человекочитаемое описание для лога

    def probability(self) -> float:
        """Вероятность срабатывания за день: 1 − exp(−rate)."""
        return 1.0 - math.exp(-self.rate)


def generate(world: World, state: SimState, templates) -> list[Effect]:
    """Создать эффекты от сработавших шаблонов на текущий день (§4, шаг 0).

    Возвращает список новых эффектов; обходим шаблоны и цели в фиксированном
    порядке (§9). Детерминизм обеспечивает счётчиковый RNG по (seed, день, цель,
    шаблон)."""
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
