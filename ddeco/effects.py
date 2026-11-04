"""Параметрические эффекты и их композиция (MODEL.md §3).

Эффект — это модификатор *параметра* (производство узла, сложность дороги k_e,
население N_i, ёмкость cap_e, спред σ). Итоговое значение параметра:

    x_eff = clamp( x_base · Π_k (1 + (mult_k − 1)·w_k) + Σ_k add_k·w_k , lo, hi )

где w_k(t) ∈ [0,1] — вес затухания. Множитель интерполируется к 1 по мере
затухания, поэтому конец эффекта не даёт скачка, а два эффекта на одном параметре
складываются независимо. Снятие одного эффекта не ломает остальные.

Разовые события (грабёж каравана, правка запасов, удар по процветанию) меняют
*состояние* один раз и эффектами не являются — см. events.py.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import math


class TargetKind(str, Enum):
    NODE = "node"
    EDGE = "edge"


class Op(str, Enum):
    MULT = "mult"   # множитель
    ADD = "add"     # добавка


class Decay(str, Enum):
    STEP = "step"          # 1 до конца длительности, затем 0
    LINEAR = "linear"      # max(0, 1 − age/duration)
    EXP = "exp"            # exp(−age/τ)


@dataclass(frozen=True, slots=True)
class Effect:
    """Один параметрический эффект (§3, §5.4 атрибуты).

    good / category фильтруют, на какие товары действует эффект на узле (для
    параметров pcap/garrison). None в обоих полях — товаро-независимый параметр
    (population, difficulty, capacity, spread).
    """

    param: str                 # "pcap" | "population" | "spread" | "difficulty" | "capacity" | "garrison"
    target_kind: TargetKind
    target_id: str             # id узла или дуги
    op: Op
    value: float
    start_day: int
    duration: int              # жёсткий конец эффекта (дней); math.inf допустимо
    decay: Decay = Decay.STEP
    tau: float = 1.0           # постоянная для экспоненциального затухания
    good: str | None = None    # фильтр по конкретному товару
    category: str | None = None  # фильтр по категории товара
    source: str = ""           # человекочитаемая причина для лога (§5.12)

    def weight(self, day: int) -> float:
        """Вес затухания w_k(t) на заданный день (§3)."""
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
        """Эффект можно удалить: вес уже всегда 0 на этот день и далее (§4, шаг 1)."""
        return day - self.start_day >= self.duration

    def matches_good(self, good_id: str | None, category: str | None) -> bool:
        """Применим ли эффект к данному товару (по фильтрам good/category)."""
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
    """Эффективное значение параметра после всех применимых эффектов (§3).

    Порядок обхода эффектов не влияет на результат: множители перемножаются,
    добавки суммируются. Для детерминизма складываем добавки в фиксированном
    порядке (как лежат в списке — он управляется вызывающим кодом).
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
    return _clamp(base * mult + add, lo, hi)


def _clamp(x: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, x))
