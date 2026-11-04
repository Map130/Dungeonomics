"""Автотест устойчивости §8.1: одноузловой контур цена–запас–производство.

Для каждой категории из таблицы §7/§6.3 прогон с возмущением стартового запаса
±20%. Критерий приёмки: к дню 300 |s/s* − 1| < 0.01 и множитель не на границе
коридора. Числа взяты из проверенной численно таблицы §6.3.
"""

import pytest

from ddeco.tick import step_day
from tests.single_node import GOOD, NODE, single_node_world, perturbed_state

# (имя, α, β, ε, T) — таблица §7 / §6.3
CATEGORIES = [
    ("necessity", 0.5, 0.3, 0.2, 10.0),
    ("common", 1.0, 0.3, 0.6, 10.0),
    ("luxury", 1.5, 0.3, 1.5, 14.0),
]


@pytest.mark.parametrize("name,alpha,beta,eps,T", CATEGORIES)
@pytest.mark.parametrize("ratio", [0.8, 1.2])
def test_converges_to_equilibrium(name, alpha, beta, eps, T, ratio):
    world = single_node_world(alpha, beta, eps, T)
    state, star = perturbed_state(world, ratio)

    for _ in range(300):
        step_day(world, state)

    s = state.stock[(NODE, GOOD)]
    m = state.multiplier[(NODE, GOOD)]
    good = world.catalog[GOOD]

    assert abs(s / star - 1.0) < 0.01, f"{name}: s/s*={s/star:.4f} не сошлось"
    # Множитель не прилип к границе коридора.
    assert good.m_min + 1e-3 < m < good.m_max - 1e-3, f"{name}: m={m:.4f} на границе"
    # В равновесии при Pcap=D множитель ≈ 1.
    assert abs(m - 1.0) < 0.02, f"{name}: m={m:.4f} далёк от 1"


@pytest.mark.parametrize("name,alpha,beta,eps,T", CATEGORIES)
def test_no_overshoot_past_corridor(name, alpha, beta, eps, T):
    """За весь прогон множитель остаётся внутри коридора (клипа быть не должно)."""
    world = single_node_world(alpha, beta, eps, T)
    state, _ = perturbed_state(world, 0.8)
    good = world.catalog[GOOD]
    for _ in range(300):
        step_day(world, state)
        m = state.multiplier[(NODE, GOOD)]
        assert good.m_min <= m <= good.m_max


def test_equilibrium_is_fixed_point():
    """Старт ровно в равновесии (s=s*, m=1) — состояние не должно дрейфовать."""
    world = single_node_world(1.0, 0.3, 0.6, 10.0)
    state, star = perturbed_state(world, 1.0)
    for _ in range(100):
        step_day(world, state)
    assert abs(state.stock[(NODE, GOOD)] / star - 1.0) < 1e-9
    assert abs(state.multiplier[(NODE, GOOD)] - 1.0) < 1e-9
