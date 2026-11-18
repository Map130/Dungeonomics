#!/usr/bin/env python3
"""Generate golden runs from the Python reference `ddeco` to check the Rust port.

Writes golden/*.json. The Rust tests (crates/ddeco-core/tests/golden.rs) load the same
files and compare: numbers with a relative tolerance (1e-9 at the start), the RNG and caravan
guards exactly.

Number format: Python json writes floats via repr (round-trip), serde_json in Rust
parses to the nearest f64 — the same IEEE754 value. So differences, if
any, come only from different pow/exp/ln implementations (libm vs CPython), which the
tolerance catches.

Usage:
    python scripts/dump_golden.py            # writes to ./golden
"""

from __future__ import annotations

import json
import struct
from pathlib import Path

from ddeco import events
from ddeco.calibration import seed_state
from ddeco.effects import Decay, Effect, Op, TargetKind
from ddeco.goods import Catalog, GoodParams
from ddeco.loader import load_world, state_to_toml
from ddeco.purchase import apply as trade_apply
from ddeco.rng import chance, uniform
from ddeco.shocks import ShockTemplate
from ddeco.state import SimState, blank_state
from ddeco.tick import step_day
from ddeco.world import GlobalParams, Node, NodeKind, World

ROOT = Path(__file__).resolve().parent.parent
GOLDEN = ROOT / "golden"
WORLD = ROOT / "worlds" / "example_region.toml"

# Shock templates — the same as in tests/test_determinism.py.
SHOCKS = [
    ShockTemplate(id="blight", target_kind=TargetKind.NODE, param="pcap",
                  op=Op.MULT, value=0.3, duration=15, rate=1 / 20,
                  category="food", source="blight"),
    ShockTemplate(id="bandits", target_kind=TargetKind.EDGE, param="difficulty",
                  op=Op.MULT, value=2.5, duration=10, rate=1 / 15, source="bandits"),
]


# --- State serialisation -------------------------------------------------------

def snap(state: SimState) -> dict:
    """A JSON-friendly state snapshot. Keys are 'node|good'."""
    return {
        "day": state.day,
        "stock": {f"{n}|{g}": v for (n, g), v in sorted(state.stock.items())},
        "multiplier": {f"{n}|{g}": v for (n, g), v in sorted(state.multiplier.items())},
        "prosperity": dict(sorted(state.prosperity.items())),
        "pipeline": {
            e: [dict(sorted(slot.items())) for slot in slots]
            for e, slots in sorted(state.pipeline.items()) if slots
        },
    }


def sample_days(total: int) -> list[int]:
    """Dense at the start, sparser later — to catch both the transient and the tail."""
    base = [0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233]
    days = sorted({d for d in base if d < total} | {total})
    return days


def run_sampled(world: World, state: SimState, total: int, shocks=()) -> list[dict]:
    """Run total days, saving snapshots on the marked days."""
    want = set(sample_days(total))
    frames = []
    if 0 in want:
        frames.append(snap(state))
    for _ in range(total):
        step_day(world, state, shocks)
        if state.day in want:
            frames.append(snap(state))
    return frames


# --- Worlds for single-node scenarios (a copy of tests/single_node.py logic) ----

def single_node_world(alpha, beta, eps, T, demand=100.0, lam=0.3) -> World:
    good = GoodParams(
        id="x", category="test", base_price=1.0,
        alpha=alpha, epsilon=eps, beta=beta, m_min=0.1, m_max=10.0,
        spoilage=0.0, coverage_days=T, c0=1.0, kappa=0.0, per_capita=1.0,
    )
    node = Node(id="n", kind=NodeKind.CITY, population=demand, prosperity0=0.5,
                capacity={"x": demand})
    return World(catalog=Catalog({"x": good}), nodes={"n": node}, edges={},
                 params=GlobalParams(lam=lam))


def single_node_state(world, ratio=0.8) -> SimState:
    st = blank_state(world)
    good = world.catalog["x"]
    star = good.coverage_days * world.nodes["n"].population
    st.stock[("n", "x")] = ratio * star
    st.multiplier[("n", "x")] = 1.0
    return st


# --- Scenarios -----------------------------------------------------------------

def scenario_single_node() -> list[dict]:
    out = []
    cats = [("necessity", 0.5, 0.3, 0.2, 10.0),
            ("common", 1.0, 0.3, 0.6, 10.0),
            ("luxury", 1.5, 0.3, 1.5, 14.0)]
    for name, a, b, e, T in cats:
        w = single_node_world(a, b, e, T)
        st = single_node_state(w, 0.8)
        out.append({"name": f"single_node_{name}",
                    "params": {"alpha": a, "beta": b, "eps": e, "T": T, "demand": 100.0},
                    "days": 300, "frames": run_sampled(w, st, 300)})
    return out


def scenario_example_run() -> dict:
    w = load_world(WORLD)
    st = seed_state(w)
    frames = run_sampled(w, st, 400)
    caravans = [
        {"edge": c.edge, "day_departed": c.day_departed, "day_arrival": c.day_arrival,
         "value": c.value, "guard": c.guard, "cargo": [list(x) for x in c.cargo]}
        for c in st.caravans
    ]
    return {"days": 400, "frames": frames, "caravans": caravans}


def scenario_shocks() -> dict:
    w = load_world(WORLD)
    st = seed_state(w)
    frames = run_sampled(w, st, 200, shocks=SHOCKS)
    # Fired effects (by source/target/start day) — to check the RNG logic.
    fired = sorted(
        {(e.source, e.target_id, e.start_day) for e in st.effects}
    )
    return {"days": 200, "frames": frames,
            "fired": [{"source": s, "target": t, "start_day": d} for s, t, d in fired]}


def scenario_events() -> dict:
    """One-shot events: purchase, robbery, prosperity hit — the state after each."""
    w = load_world(WORLD)
    st = seed_state(w)
    for _ in range(20):
        step_day(w, st)
    steps = [{"action": "run20", "state": snap(st)}]

    t = trade_apply(w, st, "redwater", "grain", 500.0)
    steps.append({"action": "buy", "trade": {"total": t.total, "avg_price": t.avg_price,
                  "new_multiplier": t.new_multiplier}, "state": snap(st)})

    edge = next(e for e in w.edge_ids() if any(slot for slot in st.pipeline.get(e, ())))
    looted = events.loot_caravan(st, edge, rho=0.5)
    steps.append({"action": "loot", "edge": edge, "looted": looted, "state": snap(st)})

    events.prosperity_hit(st, "redwater", rho=0.25)
    steps.append({"action": "prosperity_hit", "state": snap(st)})
    return {"steps": steps}


def scenario_state() -> dict:
    """State round-trip: the Python emitter writes golden/state.toml and Rust reads it
    and compares with this snapshot (cross-language format compatibility, R6)."""
    w = load_world(WORLD)
    st = seed_state(w)
    for _ in range(30):
        step_day(w, st)
    # Not named "state.toml": that is the session autosave (in .gitignore).
    (GOLDEN / "state_dump.toml").write_text(state_to_toml(st), encoding="utf-8")
    return {
        "day": st.day,
        "stock": {f"{n}|{g}": v for (n, g), v in sorted(st.stock.items())},
        "multiplier": {f"{n}|{g}": v for (n, g), v in sorted(st.multiplier.items())},
        "prosperity": dict(sorted(st.prosperity.items())),
        "caravan_accum": dict(sorted(st.caravan_accum.items())),
        "pipeline": {
            e: [dict(sorted(slot.items())) for slot in slots]
            for e, slots in sorted(st.pipeline.items())
        },
    }


def scenario_rng() -> dict:
    """Pure RNG: a grid of inputs → uniform and chance. Checked bit for bit (R4)."""
    seed = 42
    samples = []
    for day in (0, 1, 7, 100):
        for target in ("redwater", "millbrook", "redwater__millbrook"):
            for tpl in ("blight", "bandits", "caravan_guard"):
                u = uniform(seed, day, target, tpl)
                # uniform_bits — the double's bit mask, so the RNG check does not depend
                # on rounding of a decimal literal when parsing JSON (precision matters).
                bits = struct.unpack("<Q", struct.pack("<d", u))[0]
                samples.append({"day": day, "target": target, "template": tpl,
                                "uniform": u, "uniform_bits": bits,
                                "chance_0_1": chance(seed, day, 0.1, target, tpl)})
    return {"seed": seed, "samples": samples}


def main() -> None:
    GOLDEN.mkdir(exist_ok=True)

    def write(name: str, data) -> None:
        path = GOLDEN / f"{name}.json"
        path.write_text(json.dumps(data, ensure_ascii=False, indent=1), encoding="utf-8")
        print(f"  {path.relative_to(ROOT)}")

    print("golden:")
    for sc in scenario_single_node():
        write(sc["name"], sc)
    write("example_run", scenario_example_run())
    write("shocks_run", scenario_shocks())
    write("events", scenario_events())
    write("state_dump", scenario_state())
    write("rng", scenario_rng())


if __name__ == "__main__":
    main()
