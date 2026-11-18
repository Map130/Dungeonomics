"""Counter-based RNG (MODEL.md §5.11, §9).

A value is derived from coordinates (seed, day, target id, template id), NOT from
a call sequence. So undoing and replaying a day gives the same result
regardless of access order, and reproducibility does not depend on how many
times and in what order the generator was called.

We use blake2b: the hash is deterministic and identical on all platforms and in WASM
(unlike pow/exp/ln, §9), so it is fit to be the reference.
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
    """A uniform number in [0, 1) for the coordinates (seed, day, *parts)."""
    (value,) = struct.unpack("<Q", _digest(seed, day, *parts)[:8])
    return value / 2.0**64


def chance(seed: int, day: int, probability: float, *parts: object) -> bool:
    """True with the given probability, deterministically by coordinates."""
    if probability <= 0.0:
        return False
    if probability >= 1.0:
        return True
    return uniform(seed, day, *parts) < probability
