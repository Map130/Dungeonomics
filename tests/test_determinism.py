"""Autotest §8.4: determinism.

Two runs with one seed match bit for bit; undoing N days and replaying gives the same
result. Also checked with background shocks on — thanks to the counter-based RNG
(§5.11) the result does not depend on how many times and in what order the generator is called.
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
                  category="food", source="blight"),
    ShockTemplate(id="bandits", target_kind=TargetKind.EDGE, param="difficulty",
                  op=Op.MULT, value=2.5, duration=10, rate=1 / 15, source="bandits"),
]


def _snapshot(state):
    """A comparable representation of the state (dicts and pipelines)."""
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

    checkpoint = st.clone()                       # in-memory snapshot (§7)
    for _ in range(50):
        step_day(world, st, SHOCKS)
    after = _snapshot(st)

    # Roll back to the snapshot and replay the same 50 days.
    st = checkpoint
    for _ in range(50):
        step_day(world, st, SHOCKS)
    assert _snapshot(st) == after


def test_shocks_actually_fire():
    """Sanity: in 200 days the shocks fire at least once (otherwise the determinism test is empty)."""
    world = load_world(WORLD)
    st = seed_state(world)
    fired = False
    for _ in range(200):
        step_day(world, st, SHOCKS)
        if st.effects:
            fired = True
            break
    assert fired
