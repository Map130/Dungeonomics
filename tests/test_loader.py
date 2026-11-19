"""Loading world.toml and the state serialisation round-trip."""

from pathlib import Path

from ddeco.loader import load_world, state_to_toml, load_state, dump_state
from ddeco.state import blank_state
from ddeco.world import NodeKind

WORLD = Path(__file__).resolve().parent.parent / "worlds" / "example_region.toml"


def test_load_example_world():
    world = load_world(WORLD)
    assert world.seed == 42
    # 3 cities + 6 villages + 1 trading post
    assert len(world.nodes) == 10
    assert world.nodes["redwater"].kind is NodeKind.CITY
    assert world.nodes["millbrook"].kind is NodeKind.VILLAGE
    assert world.nodes["millbrook"].hub == "redwater"
    assert world.nodes["eastport"].kind is NodeKind.CAMP
    assert world.nodes["eastport"].hub is None
    # 11 roads → 22 directed arcs
    assert len(world.edges) == 22
    assert "redwater__millbrook" in world.edges
    assert "millbrook__redwater" in world.edges


def test_good_overrides_and_defaults():
    world = load_world(WORLD)
    fish = world.catalog["fish"]
    # Override from TOML
    assert fish.spoilage == 0.03
    # Parameter from the necessity price class (§7)
    assert fish.alpha == 0.5
    assert fish.in_food_basket is True
    assert set(world.catalog.food_ids()) == {"fish", "grain", "meat", "salt"}
    assert len(world.catalog.ids()) == 11


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
    assert back.stock[("redwater", "grain")] == 123.456789  # exact float round-trip
    assert back.multiplier[("redwater", "grain")] == 1.25
    assert back.prosperity["redwater"] == 0.6
    assert back.pipeline["redwater__millbrook"] == [{"grain": 10.0}, {"grain": 5.5}]
    assert back.caravan_accum["redwater__millbrook"] == 42.0


def test_good_name_table_is_ignored(tmp_path):
    """A `name = { en, ru }` table on a good is display data, not a §7 override."""
    p = tmp_path / "w.toml"
    p.write_text(
        'seed = 1\n[goods.grain]\ncategory = "food"\nbase_price = 1.0\nper_capita = 0.5\n'
        'name = { en = "Grain", ru = "Зерно" }\n'
    )
    world = load_world(p)
    assert world.catalog["grain"].base_price == 1.0
