"""Autosaving the session state (REQUIREMENTS §5.9).

The state is written to state.toml periodically (by default once a minute of wall-clock time),
NOT on every tick: an evening of play has thousands of ticks, and flushing to disk
occasionally is enough to survive a crash. The write is atomic (tmp + rename) — see loader.dump_state.

Undo/replay within a session uses in-memory snapshots (SimState.clone); the file is not
involved in that path, so losing a few float bits during serialisation does not affect
the determinism of a run.
"""

from __future__ import annotations

import time
from pathlib import Path

from .loader import dump_state
from .state import SimState


class Autosaver:
    """Flushes the state to disk no more often than once every interval seconds."""

    def __init__(self, path: str | Path, interval: float = 60.0, clock=time.monotonic):
        self.path = Path(path)
        self.interval = interval
        self._clock = clock
        self._last = clock()

    def maybe_save(self, state: SimState, *, force: bool = False) -> bool:
        """Save if the interval has passed (or force). Return True if written."""
        now = self._clock()
        if not force and now - self._last < self.interval:
            return False
        dump_state(state, self.path)
        self._last = now
        return True
