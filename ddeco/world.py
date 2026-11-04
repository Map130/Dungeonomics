"""Описание мира: узлы, дороги, граф поставок (REQUIREMENTS §5.1, MODEL.md §2).

Мир — это *параметры* (производство, население, сложность дорог, ёмкость). Они
меняются только эффектами. Состояние (запасы, цены, товары в пути, процветание)
живёт отдельно в state.SimState.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
import math

from .goods import Catalog


class NodeKind(str, Enum):
    CITY = "city"
    VILLAGE = "village"
    CAMP = "camp"


@dataclass(frozen=True, slots=True)
class Node:
    """Узел графа. Деревня привязывается к городу (hub) — иерархия §5.1."""

    id: str
    kind: NodeKind
    population: float                       # N_i, масштаб спроса
    prosperity0: float = 0.5                # стартовое π_i
    spread: float = 0.1                     # σ, спред покупки/продажи
    hub: str | None = None                  # город-хаб для деревни (§5.6)
    hub_share: float = 0.6                  # η_v, обязательная доля производства в хаб
    # Производственная мощность по товарам: good_id -> Pcap_ig. Отсутствие товара
    # в словаре означает Pcap = 0 (узел его не производит).
    capacity: dict[str, float] = field(default_factory=dict)
    # Постоянные добавки к спросу (гарнизон и т.п.): good_id -> Garrison_ig (§5.3).
    garrison: dict[str, float] = field(default_factory=dict)

    def pcap(self, good_id: str) -> float:
        return self.capacity.get(good_id, 0.0)


@dataclass(frozen=True, slots=True)
class Edge:
    """Направленная дуга i→j. Дорога между узлами даёт две дуги (§2)."""

    id: str
    src: str
    dst: str
    travel_time: float      # L_e, базовое время в пути (дней)
    difficulty: float = 1.0  # k_e, множитель сложности (эффективное значение)
    capacity: float = math.inf  # cap_e, пропускная способность (объём/день)

    def delay_days(self) -> int:
        """τ_e = max(1, ceil(L_e · k_e)) — задержка конвейера (§5.6)."""
        return max(1, math.ceil(self.travel_time * self.difficulty))


@dataclass(frozen=True, slots=True)
class GlobalParams:
    """Глобальные настройки симуляции (§7)."""

    lam: float = 0.3          # λ, скорость сглаживания множителя цены
    kappa_trade: float = 0.3  # κ_t, доля доступного запаса к отправке в день
    rho_reserve: float = 0.5  # ρ_res, неснижаемый резерв запаса
    mu: float = 0.03          # μ, скорость сходимости процветания
    pi_ref: float = 0.5       # π_ref, опорное процветание в спросе
    c_gamma: float = 0.03     # c_γ, стоимость пути как доля базовой цены за день·сложность
    w_food: float = 0.6       # w_F, вес еды в процветании
    w_market: float = 0.4     # w_M, вес рынка в процветании
    caravan_threshold: float = 1000.0  # θ_car, порог материализации каравана (§5.10)
    shock_rate: float = 1.0 / 120.0    # базовая интенсивность фоновых шоков (§5.11)
    lo_prod: float = 0.25     # нижний зажим реакции предложения m^β (§5.2)
    hi_prod: float = 2.0      # верхний зажим


@dataclass(frozen=True, slots=True)
class World:
    """Полное описание региона. Неизменяемо: эффекты применяются при расчёте."""

    catalog: Catalog
    nodes: dict[str, Node]
    edges: dict[str, Edge]
    params: GlobalParams = field(default_factory=GlobalParams)
    seed: int = 0

    def node_ids(self) -> list[str]:
        """Детерминированный порядок узлов (§9)."""
        return sorted(self.nodes)

    def edge_ids(self) -> list[str]:
        """Детерминированный порядок дуг (§9)."""
        return sorted(self.edges)

    def outgoing(self, node_id: str) -> list[str]:
        """Исходящие дуги узла, в отсортированном порядке."""
        return [e for e in self.edge_ids() if self.edges[e].src == node_id]

    def incoming(self, node_id: str) -> list[str]:
        return [e for e in self.edge_ids() if self.edges[e].dst == node_id]
