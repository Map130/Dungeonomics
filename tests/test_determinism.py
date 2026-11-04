"""Автотест §8.4: определённость.

Два прогона с одним seed совпадают бит-в-бит; откат на N дней и повтор дают тот же
результат. Проверяется и с включёнными фоновыми шоками — за счёт счётчикового RNG
(§5.11) результат не зависит от числа и порядка обращений к генератору.
"""

from ddeco.calibration import seed_state
from ddeco.effects import Op, TargetKind
from ddeco.loader import load_world
from ddeco.shocks import ShockTemplate
from ddeco.tick import step_day
from pathlib import Path

WORLD = Path(__file__).resolve().parent.parent / "worlds" / "example_region.toml"

SHOCKS = [
    ShockTemplate(id="blight", target_kind=TargetKind.NODE, param="pcap",
                  op=Op.MULT, value=0.3, duration=15, rate=1 / 20,
                  category="food", source="неурожай"),
    ShockTemplate(id="bandits", target_kind=TargetKind.EDGE, param="difficulty",
                  op=Op.MULT, value=2.5, duration=10, rate=1 / 15, source="бандиты"),
]


def _snapshot(state):
    """Сравнимое представление состояния (dict'ы и конвейеры)."""
    return (
        state.day,
        dict(state.stock),
        dict(state.multiplier),
        dict(state.prosperity),
        {e: [dict(s) for s in slots] for e, slots in state.pipeline.items()},
    )


def test_two_runs_identical_with_shocks():
    world = load_world(WORLD)
    a = seed_state(world)
    b = seed_state(world)
    for _ in range(200):
        step_day(world, a, SHOCKS)
        step_day(world, b, SHOCKS)
    assert _snapshot(a) == _snapshot(b)


def test_rollback_and_replay():
    world = load_world(WORLD)
    st = seed_state(world)
    for _ in range(100):
        step_day(world, st, SHOCKS)

    checkpoint = st.clone()                       # in-memory снимок (§7)
    for _ in range(50):
        step_day(world, st, SHOCKS)
    after = _snapshot(st)

    # Откатиться к снимку и повторить те же 50 дней.
    st = checkpoint
    for _ in range(50):
        step_day(world, st, SHOCKS)
    assert _snapshot(st) == after


def test_shocks_actually_fire():
    """Санити: за 200 дней шоки хоть раз срабатывают (иначе тест детерминизма пуст)."""
    world = load_world(WORLD)
    st = seed_state(world)
    fired = False
    for _ in range(200):
        step_day(world, st, SHOCKS)
        if st.effects:
            fired = True
            break
    assert fired
