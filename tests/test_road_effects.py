"""Road effects reach trade and caravans (§2, §5.6, §5.10): k_e and cap_e are effective values."""

from pathlib import Path

from ddeco import trade
from ddeco.calibration import seed_state
from ddeco.effects import Decay, Effect, Op, TargetKind
from ddeco.loader import load_world
from ddeco.tick import run, step_day

WORLD = Path(__file__).resolve().parent.parent / "worlds" / "example_region.toml"


def _traded_world():
    world = load_world(WORLD)
    st = seed_state(world)
    run(world, st, 200)
    return world, st


def _road_effect(st, edge, param, value, duration, decay=Decay.STEP):
    st.effects.append(Effect(
        param=param, target_kind=TargetKind.EDGE, target_id=edge, op=Op.MULT,
        value=value, start_day=st.day, duration=duration, decay=decay, source="test",
    ))


def test_difficulty_effect_lengthens_delay():
    edge = "eastford__redwater"
    world, st = _traded_world()
    base_world, base_st = _traded_world()
    _road_effect(st, edge, "difficulty", 2.5, 10)
    step_day(world, st)
    step_day(base_world, base_st)
    slots = st.pipeline[edge]
    assert len(slots) >= 5 and slots[4]
    assert len(base_st.pipeline[edge]) <= 2


def test_capacity_effect_limits_flow(monkeypatch):
    edge = "redwater__eastford"
    world, st = _traded_world()
    base_world, base_st = _traded_world()
    _road_effect(st, edge, "capacity", 0.1, 30)

    added: list[float] = []
    real = trade.add_to_pipeline

    def spy(state, edge_id, tau, good, amount):
        if edge_id == edge:
            added.append(amount * world.catalog[good].volume)
        real(state, edge_id, tau, good, amount)

    monkeypatch.setattr(trade, "add_to_pipeline", spy)
    limited_days = []
    for _ in range(10):
        added.clear()
        step_day(world, st)
        limited_days.append(sum(added))
    limited = sum(limited_days)
    assert max(limited_days) <= 600 * 0.1 + 1e-9

    added.clear()
    base_total = 0.0
    for _ in range(10):
        added.clear()
        step_day(base_world, base_st)
        base_total += sum(added)
    assert limited < base_total


def test_road_effect_expires():
    edge = "eastford__redwater"
    world, st = _traded_world()
    _road_effect(st, edge, "difficulty", 2.5, 5)
    run(world, st, 6)
    st.pipeline[edge] = []   # drop old cargo; look only at what is dispatched on the next day
    step_day(world, st)
    slots = st.pipeline[edge]
    assert len(slots) == 2 and slots[1]


def test_zero_multiplier_on_infinite_base_is_zero():
    import math

    from ddeco.effects import effective

    e = Effect("capacity", TargetKind.EDGE, "a__b", Op.MULT, 0.0, 0, 10)
    assert effective(math.inf, [e], "capacity", TargetKind.EDGE, "a__b", 0) == 0.0
