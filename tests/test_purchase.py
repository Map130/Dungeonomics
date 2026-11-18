"""Effect of a player's purchase/sale on the price (§5.7)."""

import pytest

from ddeco.purchase import apply, quote
from tests.single_node import GOOD, NODE, single_node_world


def _state(stock, m=1.0):
    from ddeco.state import blank_state
    world = single_node_world(1.0, 0.3, 0.6, 10.0)
    st = blank_state(world)
    st.stock[(NODE, GOOD)] = stock
    st.multiplier[(NODE, GOOD)] = m
    return world, st


def test_bulk_buy_costs_more_per_unit_than_small_buy():
    world, st = _state(1000.0)
    small = quote(world, st, NODE, GOOD, 10.0)
    big = quote(world, st, NODE, GOOD, 500.0)
    assert big.avg_price > small.avg_price   # convex curve: the tail is pricier


def test_buy_above_sticker_due_to_spread_and_curve():
    world, st = _state(1000.0, m=1.0)
    g = world.catalog[GOOD]
    sticker = g.base_price * 1.0 * (1 + world.nodes[NODE].spread)
    t = quote(world, st, NODE, GOOD, 100.0)
    assert t.avg_price > sticker


def test_cannot_buy_whole_stock():
    world, st = _state(100.0)
    with pytest.raises(ValueError):
        quote(world, st, NODE, GOOD, 100.0)


def test_apply_updates_stock_and_multiplier():
    world, st = _state(1000.0, m=1.0)
    t = apply(world, st, NODE, GOOD, 200.0)
    assert st.stock[(NODE, GOOD)] == 800.0
    assert st.multiplier[(NODE, GOOD)] > 1.0           # shortage after buying out
    assert abs(st.multiplier[(NODE, GOOD)] - t.new_multiplier) < 1e-12


def test_sell_below_sticker_and_lowers_price():
    world, st = _state(1000.0, m=1.0)
    g = world.catalog[GOOD]
    sticker = g.base_price * 1.0 * (1 - world.nodes[NODE].spread)
    t = apply(world, st, NODE, GOOD, 200.0, sell=True)
    assert t.avg_price < sticker
    assert st.stock[(NODE, GOOD)] == 1200.0
    assert st.multiplier[(NODE, GOOD)] < 1.0
