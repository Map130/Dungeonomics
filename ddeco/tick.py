"""Оркестрация одного дня симуляции (MODEL.md §4).

Порядок шагов фиксирован и менять его нельзя. Все узлы и дуги обрабатываются по
синхронному снимку состояния на начало соответствующего шага, поэтому порядок
обхода не влияет на результат.

    0. События (эффекты добавляются извне до вызова step_day)
    1. Пересчёт эффективных параметров; истёкшие эффекты удаляются
    2. Прибытие (конвейеры сдвигаются, груз с d=0 попадает в запас)
    3. Производство
    4. Потребление
    5. Порча
    6. Торговая отправка
    7. Процветание
    8. Цена
    9. Материализация караванов
    10. Лог причин

Торговля (6), процветание (7), караваны (9) и лог (10) подключаются в своих
задачах; сейчас это заглушки, не влияющие на результат.
"""

from __future__ import annotations

from . import formulas as F
from .effects import Effect, Op, TargetKind, effective
from .state import SimState
from .world import World


# --- Эффективные параметры (база из мира + активные эффекты, §3) ---------------

def effective_pcap(world: World, state: SimState, node: str, good: str) -> float:
    base = world.nodes[node].pcap(good)
    return effective(
        base, state.effects, "pcap", TargetKind.NODE, node, state.day,
        good_id=good, category=world.catalog[good].category, lo=0.0,
    )


def effective_population(world: World, state: SimState, node: str) -> float:
    base = world.nodes[node].population
    return effective(
        base, state.effects, "population", TargetKind.NODE, node, state.day, lo=0.0,
    )


def effective_garrison(world: World, state: SimState, node: str, good: str) -> float:
    base = world.nodes[node].garrison.get(good, 0.0)
    return effective(
        base, state.effects, "garrison", TargetKind.NODE, node, state.day,
        good_id=good, category=world.catalog[good].category, lo=0.0,
    )


def effective_spread(world: World, state: SimState, node: str) -> float:
    base = world.nodes[node].spread
    return effective(
        base, state.effects, "spread", TargetKind.NODE, node, state.day, lo=0.0,
    )


def effective_difficulty(world: World, state: SimState, edge: str) -> float:
    base = world.edges[edge].difficulty
    return effective(
        base, state.effects, "difficulty", TargetKind.EDGE, edge, state.day, lo=0.0,
    )


def effective_capacity(world: World, state: SimState, edge: str) -> float:
    base = world.edges[edge].capacity
    return effective(
        base, state.effects, "capacity", TargetKind.EDGE, edge, state.day, lo=0.0,
    )


# --- Шаги дня ------------------------------------------------------------------

def _drop_expired(state: SimState) -> None:
    """Шаг 1: удалить эффекты, у которых вес уже всегда 0."""
    day = state.day
    state.effects = [e for e in state.effects if not e.expired(day)]


def _arrivals(world: World, state: SimState) -> None:
    """Шаг 2: груз с d=0 прибывает в узел назначения, конвейер сдвигается."""
    for e in world.edge_ids():
        slots = state.pipeline.get(e)
        if not slots:
            continue
        arrived = slots.pop(0)  # индекс 0 = прибывает сегодня; остальное сдвигается
        dst = world.edges[e].dst
        for g, amt in arrived.items():
            state.stock[(dst, g)] = state.stock.get((dst, g), 0.0) + amt


def _production_consumption_spoilage(
    world: World, state: SimState, m0: dict, demand: dict, star: dict
) -> None:
    """Шаги 3–5: производство, потребление, порча по снимку начала дня (m0)."""
    p = world.params
    for n in world.node_ids():
        pros = state.prosperity[n]
        pop = effective_population(world, state, n)
        for g in world.catalog.ids():
            good = world.catalog[g]
            D = F.base_demand(good, pop, pros, p.pi_ref, effective_garrison(world, state, n, g))
            demand[(n, g)] = D
            star[(n, g)] = F.target_stock(good, D)

            s = state.stock[(n, g)]
            s += F.production(good, effective_pcap(world, state, n, g), m0[(n, g)], p.lo_prod, p.hi_prod)
            s -= F.consumption(good, D, m0[(n, g)], s)
            s = F.after_spoilage(good, s)
            state.stock[(n, g)] = s


def _prices(world: World, state: SimState, star: dict) -> None:
    """Шаг 8: целевой множитель от обеспеченности и сглаживание к нему."""
    lam = world.params.lam
    for n in world.node_ids():
        for g in world.catalog.ids():
            good = world.catalog[g]
            mstar = F.target_multiplier(good, state.stock[(n, g)], star[(n, g)])
            state.multiplier[(n, g)] = F.smooth_multiplier(state.multiplier[(n, g)], mstar, lam)


# Заглушки, подключаются в следующих задачах.
def _dispatch(world: World, state: SimState, star: dict) -> None:
    """Шаг 6: торговая отправка (§5.6) — задача 4."""


def _basket_terms(world: World, state: SimState, node: str, good_ids: list[str], star: dict):
    """Список (basket_weight, r) по товарам корзины с ненулевым спросом."""
    terms = []
    for g in good_ids:
        good = world.catalog[g]
        if good.basket_weight <= 0.0:
            continue
        s_star = star[(node, g)]
        if s_star <= 0.0:
            continue  # нет спроса — товар не входит в корзину этого узла
        terms.append((good.basket_weight, state.stock[(node, g)] / s_star))
    return terms


def _prosperity(world: World, state: SimState, star: dict) -> None:
    """Шаг 7: процветание π как медленная переменная (§5.8).

    Естественный потолок: выше π → выше спрос (§5.3) → ниже r → ниже π*, поэтому
    π упирается в уровень, где потребление съедает излишек производства."""
    p = world.params
    catalog = world.catalog
    food_ids = catalog.food_ids()
    market_ids = catalog.market_ids()
    for n in world.node_ids():
        food_fill = F.basket_fill(_basket_terms(world, state, n, food_ids, star))
        market_fill = F.basket_fill(_basket_terms(world, state, n, market_ids, star))
        target = F.prosperity_target(food_fill, market_fill, p.w_food, p.w_market)
        state.prosperity[n] = F.smooth_prosperity(state.prosperity[n], target, p.mu)


def _caravans(world: World, state: SimState) -> None:
    """Шаг 9: материализация караванов (§5.10) — задача 6."""


def step_day(world: World, state: SimState) -> SimState:
    """Продвинуть состояние на один день. Мутирует state и возвращает его.

    Снимок для отката делается вызывающим кодом через state.clone() до вызова."""
    _drop_expired(state)                              # 1
    _arrivals(world, state)                           # 2
    m0 = dict(state.multiplier)                       # снимок цен на начало дня
    demand: dict = {}
    star: dict = {}
    _production_consumption_spoilage(world, state, m0, demand, star)  # 3–5
    _dispatch(world, state, star)                     # 6
    _prosperity(world, state, star)                   # 7
    _prices(world, state, star)                       # 8
    _caravans(world, state)                           # 9
    state.day += 1
    return state


def run(world: World, state: SimState, days: int) -> SimState:
    """Прокрутить N дней (REQUIREMENTS §5.7)."""
    for _ in range(days):
        step_day(world, state)
    return state
