"""Караваны (§5.10), разовые события (§3) и лог причин (§5.12)."""

from pathlib import Path

from ddeco import events
from ddeco.caravans import materialize
from ddeco.calibration import seed_state
from ddeco.effects import Decay, Effect, Op, TargetKind
from ddeco.loader import load_world
from ddeco import reasons
from ddeco.reasons import explain_price
from ddeco.tick import run, step_day

WORLD = Path(__file__).resolve().parent.parent / "worlds" / "example_region.toml"


def _traded_world():
    """Пример мира, прогретый так, что по дорогам идут потоки."""
    world = load_world(WORLD)
    st = seed_state(world)
    run(world, st, 200)
    return world, st


# --- Караваны -----------------------------------------------------------------

def test_caravan_materializes_with_cargo_and_guard():
    world, st = _traded_world()
    # Крутим, пока не накопится хоть один караван.
    for _ in range(400):
        step_day(world, st)
        if st.caravans:
            break
    assert st.caravans, "караваны не материализовались"
    c = st.caravans[0]
    assert c.value >= world.params.caravan_threshold - 1e-6
    assert c.day_arrival > c.day_departed
    assert c.guard > 0.0
    assert c.cargo, "у каравана пустой груз"


def test_caravan_guard_is_deterministic():
    w1, s1 = _traded_world()
    w2, s2 = _traded_world()
    for _ in range(400):
        step_day(w1, s1)
        step_day(w2, s2)
    guards1 = [c.guard for c in s1.caravans]
    guards2 = [c.guard for c in s2.caravans]
    assert guards1 == guards2


# --- Разовые события ----------------------------------------------------------

def test_loot_removes_cargo_from_pipeline():
    world, st = _traded_world()
    # Найти дугу с грузом в пути.
    edge = next(e for e in world.edge_ids()
                if any(slot for slot in st.pipeline.get(e, ())))
    before = sum(v for slot in st.pipeline[edge] for v in slot.values())
    looted = events.loot_caravan(st, edge, rho=0.5)
    after = sum(v for slot in st.pipeline[edge] for v in slot.values())
    assert looted > 0.0
    assert abs(before - after - looted) < 1e-9
    assert after < before


def test_prosperity_hit_and_stock_edit():
    world, st = _traded_world()
    n = "redwater"
    pi0 = st.prosperity[n]
    events.prosperity_hit(st, n, rho=0.25)
    assert abs(st.prosperity[n] - pi0 * 0.75) < 1e-12
    events.set_stock(st, n, "grain", 42.0)
    assert st.stock[(n, "grain")] == 42.0


# --- Лог причин ---------------------------------------------------------------

def test_reasons_name_the_blocked_road():
    world = load_world(WORLD)
    st = seed_state(world)
    # Монстр на дороге eastford→redwater: ×2.5 сложности на 10 дней (§5.4 пример).
    st.effects.append(Effect(
        param="difficulty", target_kind=TargetKind.EDGE, target_id="eastford__redwater",
        op=Op.MULT, value=2.5, start_day=st.day, duration=10, decay=Decay.LINEAR,
        source="монстр на дороге",
    ))
    run(world, st, 5)
    report = explain_price(world, st, "redwater", "grain")
    road = [c for c in report.top() if c.kind == reasons.EDGE_EFFECT]
    assert road, report.causes
    c = road[0]
    assert (c.src, c.dst, c.param, c.op) == ("eastford", "redwater", "difficulty", "mult")
    assert c.source == "монстр на дороге"
    assert c.value > 1.0


def test_reasons_verdict_scarcity():
    world = load_world(WORLD)
    st = seed_state(world)
    events.set_stock(st, "redwater", "grain", 1.0)  # искусственный дефицит
    report = explain_price(world, st, "redwater", "grain")
    assert report.verdict == reasons.SHORTAGE
