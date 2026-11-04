"""Счётчиковый RNG (MODEL.md §5.11, §9).

Значение выводится из координат (seed, день, id цели, id шаблона), а НЕ из
последовательности вызовов. Тогда откат и повтор дня дают тот же результат
независимо от порядка обращений, и воспроизводимость не зависит от того, сколько
раз и в каком порядке дёргали генератор.

Используем blake2b: хеш детерминирован и одинаков на всех платформах и в WASM
(в отличие от pow/exp/ln, §9), поэтому годится как эталон.
"""

from __future__ import annotations

import hashlib
import struct

_SEP = b"\x1f"


def _digest(seed: int, day: int, *parts: object) -> bytes:
    h = hashlib.blake2b(digest_size=16)
    h.update(str(seed).encode())
    h.update(_SEP)
    h.update(str(day).encode())
    for p in parts:
        h.update(_SEP)
        h.update(str(p).encode())
    return h.digest()


def uniform(seed: int, day: int, *parts: object) -> float:
    """Равномерное число в [0, 1) для координат (seed, day, *parts)."""
    (value,) = struct.unpack("<Q", _digest(seed, day, *parts)[:8])
    return value / 2.0**64


def chance(seed: int, day: int, probability: float, *parts: object) -> bool:
    """True с вероятностью probability, детерминированно по координатам."""
    if probability <= 0.0:
        return False
    if probability >= 1.0:
        return True
    return uniform(seed, day, *parts) < probability
