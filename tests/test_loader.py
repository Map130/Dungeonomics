"""Загрузка world.toml и round-trip сериализации состояния."""

from pathlib import Path

from ddeco.loader import load_world, state_to_toml, load_state, dump_state
from ddeco.state import blank_state
from ddeco.world import NodeKind

WORLD = Path(__file__).resolve().parent.parent / "worlds" / "example_region.toml"


def test_load_example_world():
    world = load_world(WORLD)
    assert world.seed == 42
    # 3 города + 4 деревни
    assert len(world.nodes) == 7
    assert world.nodes["redwater"].kind is NodeKind.CITY
    assert world.nodes["millbrook"].kind is NodeKind.VILLAGE
    assert world.nodes["millbrook"].hub == "redwater"
    # 7 дорог → 14 направленных дуг
    assert len(world.edges) == 14
    assert "redwater__millbrook" in world.edges
    assert "millbrook__redwater" in world.edges


def test_good_overrides_and_defaults():
    world = load_world(WORLD)
    fish = world.catalog["fish"]
    # Переопределение из TOML
    assert fish.spoilage == 0.03
    # Параметр из прайс-класса necessity (§7)
    assert fish.alpha == 0.5
    assert fish.in_food_basket is True
    assert set(world.catalog.food_ids()) == {"fish", "grain"}


def test_edge_delay():
    world = load_world(WORLD)
    e = world.edges["redwater__stonehaven"]  # travel_time 3, difficulty 1.2
    assert e.delay_days() == 4  # ceil(3 * 1.2) = 4


def test_state_roundtrip(tmp_path):
    world = load_world(WORLD)
    st = blank_state(world)
    st.day = 7
    st.stock[("redwater", "grain")] = 123.456789
    st.multiplier[("redwater", "grain")] = 1.25
    st.prosperity["redwater"] = 0.6
    st.pipeline["redwater__millbrook"] = [{"grain": 10.0}, {"grain": 5.5}]
    st.caravan_accum["redwater__millbrook"] = 42.0

    path = tmp_path / "state.toml"
    dump_state(st, path)
    back = load_state(path)

    assert back.day == 7
    assert back.stock[("redwater", "grain")] == 123.456789  # точный round-trip float
    assert back.multiplier[("redwater", "grain")] == 1.25
    assert back.prosperity["redwater"] == 0.6
    assert back.pipeline["redwater__millbrook"] == [{"grain": 10.0}, {"grain": 5.5}]
    assert back.caravan_accum["redwater__millbrook"] == 42.0
