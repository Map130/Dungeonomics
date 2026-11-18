"""Loading and saving the world/state as TOML (REQUIREMENTS §5.9).

Two separate files:
- world.toml  — the world description (edited by the GM / community, versioned).
- state.toml  — a snapshot of the session state (autosaved, not on every tick).

tomllib (stdlib) can only read, so writing state.toml uses a compact
emitter for our schema. The program never rewrites the world description, it only reads it.

world.toml schema:

    seed = 42

    [params]              # optional; overrides GlobalParams
    lam = 0.3

    [goods.grain]
    category = "food"
    base_price = 1.0
    price_class = "necessity"   # necessity | common | luxury
    per_capita = 1.0
    in_food_basket = true
    basket_weight = 0.5
    # any GoodParams field can be overridden: alpha = 0.6, coverage_days = 12, ...

    [nodes.redwater]
    kind = "city"               # city | village | camp
    population = 1000
    prosperity0 = 0.5
    capacity = { grain = 800.0, cloth = 120.0 }
    garrison = { grain = 50.0 }     # optional

    [nodes.millbrook]
    kind = "village"
    hub = "redwater"
    hub_share = 0.6

    [[roads]]                   # undirected road → two arcs
    a = "redwater"
    b = "millbrook"
    travel_time = 2
    difficulty = 1.0
    capacity = 500.0
"""

from __future__ import annotations

import math
import tomllib
from pathlib import Path

from .goods import Catalog, GoodParams, PriceClass, make_good
from .state import SimState
from .world import Edge, GlobalParams, Node, NodeKind, World


# --- Loading the world ---------------------------------------------------------

def load_world(path: str | Path) -> World:
    data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
    return world_from_dict(data)


def world_from_dict(data: dict) -> World:
    catalog = _parse_catalog(data.get("goods", {}))
    nodes = {nid: _parse_node(nid, raw) for nid, raw in data.get("nodes", {}).items()}
    edges = _parse_roads(data.get("roads", []))
    params = _parse_params(data.get("params", {}))
    return World(
        catalog=catalog,
        nodes=nodes,
        edges=edges,
        params=params,
        seed=int(data.get("seed", 0)),
    )


def _parse_catalog(raw: dict) -> Catalog:
    goods: dict[str, GoodParams] = {}
    for gid, g in raw.items():
        price_class = PriceClass(g.pop("price_class", "common"))
        goods[gid] = make_good(
            id=gid,
            category=g.pop("category"),
            base_price=float(g.pop("base_price")),
            price_class=price_class,
            per_capita=float(g.pop("per_capita")),
            in_food_basket=bool(g.pop("in_food_basket", False)),
            basket_weight=float(g.pop("basket_weight", 0.0)),
            volume=float(g.pop("volume", 1.0)),
            weight=float(g.pop("weight", 1.0)),
            **{k: float(v) for k, v in g.items()},  # the rest are §7 overrides
        )
    return Catalog(goods=goods)


def _parse_node(nid: str, raw: dict) -> Node:
    return Node(
        id=nid,
        kind=NodeKind(raw.get("kind", "city")),
        population=float(raw["population"]),
        prosperity0=float(raw.get("prosperity0", 0.5)),
        spread=float(raw.get("spread", 0.1)),
        hub=raw.get("hub"),
        hub_share=float(raw.get("hub_share", 0.6)),
        capacity={k: float(v) for k, v in raw.get("capacity", {}).items()},
        garrison={k: float(v) for k, v in raw.get("garrison", {}).items()},
    )


def _parse_roads(raw: list) -> dict[str, Edge]:
    """Each undirected road → two directed arcs a→b and b→a."""
    edges: dict[str, Edge] = {}
    for road in raw:
        a, b = road["a"], road["b"]
        common = dict(
            travel_time=float(road["travel_time"]),
            difficulty=float(road.get("difficulty", 1.0)),
            capacity=float(road.get("capacity", math.inf)),
        )
        for src, dst in ((a, b), (b, a)):
            eid = f"{src}__{dst}"
            edges[eid] = Edge(id=eid, src=src, dst=dst, **common)
    return edges


def _parse_params(raw: dict) -> GlobalParams:
    fields = GlobalParams.__dataclass_fields__
    return GlobalParams(**{k: float(v) for k, v in raw.items() if k in fields})


# --- Saving/loading the state --------------------------------------------------

def load_state(path: str | Path) -> SimState:
    data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
    st = SimState(day=int(data.get("day", 0)))
    for node, goods in data.get("stock", {}).items():
        for g, v in goods.items():
            st.stock[(node, g)] = float(v)
    for node, goods in data.get("multiplier", {}).items():
        for g, v in goods.items():
            st.multiplier[(node, g)] = float(v)
    st.prosperity = {k: float(v) for k, v in data.get("prosperity", {}).items()}
    st.caravan_accum = {k: float(v) for k, v in data.get("caravan_accum", {}).items()}
    for e, slots in data.get("pipeline", {}).items():
        st.pipeline[e] = [{g: float(v) for g, v in slot.items()} for slot in slots]
    return st


def dump_state(st: SimState, path: str | Path) -> None:
    """Write state.toml atomically (tmp + rename)."""
    text = state_to_toml(st)
    p = Path(path)
    tmp = p.with_suffix(p.suffix + ".tmp")
    tmp.write_text(text, encoding="utf-8")
    tmp.replace(p)


def state_to_toml(st: SimState) -> str:
    """A compact TOML emitter for the SimState schema. Effects are not serialised:
    the GM / the scenario log restores them (v1). Keys are sorted for
    stable diffs."""
    lines = [f"day = {st.day}", ""]

    lines.append("[prosperity]")
    for node in sorted(st.prosperity):
        lines.append(f"{node} = {_num(st.prosperity[node])}")
    lines.append("")

    for table, data in (("stock", st.stock), ("multiplier", st.multiplier)):
        by_node: dict[str, dict[str, float]] = {}
        for (node, good), v in data.items():
            by_node.setdefault(node, {})[good] = v
        for node in sorted(by_node):
            lines.append(f"[{table}.{node}]")
            for good in sorted(by_node[node]):
                lines.append(f"{good} = {_num(by_node[node][good])}")
            lines.append("")

    if any(st.pipeline.values()):
        lines.append("[pipeline]")
        for e in sorted(st.pipeline):
            slots = st.pipeline[e]
            inner = ", ".join(
                "{" + ", ".join(f"{g} = {_num(slot[g])}" for g in sorted(slot)) + "}"
                for slot in slots
            )
            lines.append(f"{e} = [{inner}]")
        lines.append("")

    lines.append("[caravan_accum]")
    for e in sorted(st.caravan_accum):
        lines.append(f"{e} = {_num(st.caravan_accum[e])}")
    lines.append("")

    return "\n".join(lines) + "\n"


def _num(x: float) -> str:
    """A float with full precision (repr) for the most accurate round-trip."""
    if x == math.inf:
        return "inf"
    return repr(float(x))
