"""World description: nodes, roads, the supply graph (REQUIREMENTS §5.1, MODEL.md §2).

The world is *parameters* (production, population, road difficulty, capacity). They
change only through effects. The state (stocks, prices, goods in transit, prosperity)
lives separately in state.SimState.
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
    """A graph node. A village is bound to a city (hub) — the §5.1 hierarchy."""

    id: str
    kind: NodeKind
    population: float                       # N_i, demand scale
    prosperity0: float = 0.5                # starting π_i
    spread: float = 0.1                     # σ, buy/sell spread
    hub: str | None = None                  # hub city for a village (§5.6)
    hub_share: float = 0.6                  # η_v, mandatory share of production to the hub
    # Production capacity per good: good_id -> Pcap_ig. A good missing
    # from the dict means Pcap = 0 (the node does not produce it).
    capacity: dict[str, float] = field(default_factory=dict)
    # Constant additions to demand (garrison etc.): good_id -> Garrison_ig (§5.3).
    garrison: dict[str, float] = field(default_factory=dict)

    def pcap(self, good_id: str) -> float:
        return self.capacity.get(good_id, 0.0)


@dataclass(frozen=True, slots=True)
class Edge:
    """Directed arc i→j. A road between nodes gives two arcs (§2)."""

    id: str
    src: str
    dst: str
    travel_time: float      # L_e, base travel time (days)
    difficulty: float = 1.0  # k_e, difficulty multiplier (effective value)
    capacity: float = math.inf  # cap_e, throughput (volume/day)

    def delay_days(self) -> int:
        """τ_e = max(1, ceil(L_e · k_e)) — pipeline delay (§5.6)."""
        return max(1, math.ceil(self.travel_time * self.difficulty))


@dataclass(frozen=True, slots=True)
class GlobalParams:
    """Global simulation settings (§7)."""

    lam: float = 0.3          # λ, smoothing rate of the price multiplier
    kappa_trade: float = 0.3  # κ_t, share of the available stock shipped per day
    rho_reserve: float = 0.5  # ρ_res, untouchable stock reserve
    mu: float = 0.03          # μ, convergence rate of prosperity
    pi_ref: float = 0.5       # π_ref, reference prosperity in demand
    c_gamma: float = 0.03     # c_γ, travel cost as a share of base price per day·difficulty
    w_food: float = 0.6       # w_F, weight of food in prosperity
    w_market: float = 0.4     # w_M, weight of the market in prosperity
    caravan_threshold: float = 1000.0  # θ_car, caravan materialisation threshold (§5.10)
    shock_rate: float = 1.0 / 120.0    # base rate of background shocks (§5.11)
    lo_prod: float = 0.25     # lower clamp of the supply response m^β (§5.2)
    hi_prod: float = 2.0      # upper clamp


@dataclass(frozen=True, slots=True)
class World:
    """Full region description. Immutable: effects are applied at evaluation time."""

    catalog: Catalog
    nodes: dict[str, Node]
    edges: dict[str, Edge]
    params: GlobalParams = field(default_factory=GlobalParams)
    seed: int = 0

    def node_ids(self) -> list[str]:
        """Deterministic order of nodes (§9)."""
        return sorted(self.nodes)

    def edge_ids(self) -> list[str]:
        """Deterministic order of arcs (§9)."""
        return sorted(self.edges)

    def outgoing(self, node_id: str) -> list[str]:
        """Outgoing arcs of a node, in sorted order."""
        return [e for e in self.edge_ids() if self.edges[e].src == node_id]

    def incoming(self, node_id: str) -> list[str]:
        return [e for e in self.edge_ids() if self.edges[e].dst == node_id]
