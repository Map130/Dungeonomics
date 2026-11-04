"""Состояние симуляции (MODEL.md §2). Эволюционирует само; эффекты меняют только
параметры мира, не это.

Содержит запасы s_ig, множители цен m_ig, процветание π_i, конвейеры товаров в
пути q[e][d] и список активных эффектов. Всё, что нужно для отката/повтора дня
(§7, §8.4), лежит здесь — снимок делается через clone().

Конвейер дуги: pipeline[edge_id] — список словарей {good_id: amount}, где индекс p
означает «прибудет через p шагов прибытия». На шаге прибытия (§4, шаг 2) берётся
индекс 0 и список сдвигается. Отправка с задержкой τ кладёт груз в индекс τ−1
(см. trade.py).
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .effects import Effect
from .world import World


Key = tuple[str, str]  # (node_id, good_id)


@dataclass(slots=True)
class SimState:
    day: int = 0
    stock: dict[Key, float] = field(default_factory=dict)        # s_ig
    multiplier: dict[Key, float] = field(default_factory=dict)   # m_ig
    prosperity: dict[str, float] = field(default_factory=dict)   # π_i
    pipeline: dict[str, list[dict[str, float]]] = field(default_factory=dict)  # q[e][d]
    effects: list[Effect] = field(default_factory=list)
    # Накопленная стоимость потока по дуге с последней материализации (§5.10).
    caravan_accum: dict[str, float] = field(default_factory=dict)
    # Материализованные караваны (лента мастеру, §5.10). Элементы — frozen dataclass.
    caravans: list = field(default_factory=list)

    def clone(self) -> "SimState":
        """Глубокий снимок для отката/повтора. Effect/Caravan неизменяемы → списки
        копируем поверхностно; остальное — вручную, чтобы не тянуть copy.deepcopy и
        держать поведение явным и детерминированным."""
        return SimState(
            day=self.day,
            stock=dict(self.stock),
            multiplier=dict(self.multiplier),
            prosperity=dict(self.prosperity),
            pipeline={e: [dict(slot) for slot in slots] for e, slots in self.pipeline.items()},
            effects=list(self.effects),
            caravan_accum=dict(self.caravan_accum),
            caravans=list(self.caravans),
        )

    def in_transit_to(self, world: World, node_id: str, good_id: str) -> float:
        """Q_jg: суммарный товар good_id в пути в узел node_id (§5.6).

        Суммируем в фиксированном порядке дуг и слотов (§9)."""
        total = 0.0
        for e in world.edge_ids():
            if world.edges[e].dst != node_id:
                continue
            for slot in self.pipeline.get(e, ()):
                total += slot.get(good_id, 0.0)
        return total


def blank_state(world: World) -> SimState:
    """Пустое состояние: нулевые запасы, m = 1, π = π0, пустые конвейеры."""
    st = SimState(day=0)
    for n in world.node_ids():
        node = world.nodes[n]
        st.prosperity[n] = node.prosperity0
        for g in world.catalog.ids():
            st.stock[(n, g)] = 0.0
            st.multiplier[(n, g)] = 1.0
    for e in world.edge_ids():
        st.pipeline[e] = []
        st.caravan_accum[e] = 0.0
    return st
