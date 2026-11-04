"""Автосохранение состояния сессии (REQUIREMENTS §5.9).

Состояние пишется в state.toml периодически (по умолчанию раз в минуту wall-clock),
а НЕ на каждый тик: тиков за вечер игры тысячи, а на диск достаточно сбрасываться
изредка для защиты от падения. Запись атомарна (tmp + rename) — см. loader.dump_state.

Откат/повтор внутри сессии идёт по in-memory снимкам (SimState.clone), файл в этот
путь не вовлечён, поэтому потеря нескольких бит float при сериализации не влияет на
детерминизм прогона.
"""

from __future__ import annotations

import time
from pathlib import Path

from .loader import dump_state
from .state import SimState


class Autosaver:
    """Сбрасывает состояние на диск не чаще, чем раз в interval секунд."""

    def __init__(self, path: str | Path, interval: float = 60.0, clock=time.monotonic):
        self.path = Path(path)
        self.interval = interval
        self._clock = clock
        self._last = clock()

    def maybe_save(self, state: SimState, *, force: bool = False) -> bool:
        """Сохранить, если прошёл интервал (или force). Вернуть True, если записали."""
        now = self._clock()
        if not force and now - self._last < self.interval:
            return False
        dump_state(state, self.path)
        self._last = now
        return True
