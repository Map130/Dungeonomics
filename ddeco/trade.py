"""Торговые потоки: арбитраж и поставка в хаб (MODEL.md §5.6).

Фоновая торговля без агентов: если ожидаемая разница цен между узлами превышает
стоимость пути, часть запаса уходит в конвейер дуги с задержкой. Решение о
перевозке принимается по *ожидаемому* множителю с учётом уже едущего товара Q
(иначе торговцы слали бы груз каждый день, пока едет первая партия → перелив).

Всё считается по синхронному снимку запасов на начало шага 6: сначала обязательные
поставки деревень в хаб, затем арбитраж по остатку. Порядок обхода дуг/товаров
фиксирован (§9), поэтому результат воспроизводим.
"""

from __future__ import annotations

from . import formulas as F
from .state import SimState
from .world import World


def add_to_pipeline(state: SimState, edge_id: str, tau: int, good: str, amount: float) -> None:
    """Положить груз в конвейер дуги с задержкой τ (прибудет через τ дней).

    Индекс τ−1: шаг прибытия (§4.2) снимает индекс 0 в начале каждого дня, поэтому
    груз на индексе τ−1 будет снят ровно через τ дней."""
    slots = state.pipeline.setdefault(edge_id, [])
    idx = tau - 1
    while len(slots) <= idx:
        slots.append({})
    slots[idx][good] = slots[idx].get(good, 0.0) + amount


def hub_deliveries(world: World, state: SimState, produced: dict) -> None:
    """Обязательная поставка деревни в город-хаб: доля η_v производства независимо
    от разницы цен (§5.6). Арбитраж дальше работает с остатком."""
    for n in world.node_ids():
        node = world.nodes[n]
        if node.hub is None:
            continue
        edge_id = f"{n}__{node.hub}"
        edge = world.edges.get(edge_id)
        if edge is None:
            continue  # нет дороги деревня→хаб — поставку пропускаем
        tau = edge.delay_days()
        for g in world.catalog.ids():
            p = produced.get((n, g), 0.0)
            if p <= 0.0:
                continue
            amount = min(node.hub_share * p, state.stock[(n, g)])
            if amount <= 0.0:
                continue
            state.stock[(n, g)] -= amount
            add_to_pipeline(state, edge_id, tau, g, amount)


def _expected_multiplier(world: World, state: SimState, node: str, good: str, star: float) -> float:
    """m̂ = clamp(((s + Q)/s*)^(−α), m_min, m_max) — с учётом товара в пути в узел."""
    good_p = world.catalog[good]
    if star <= 0.0:
        return good_p.m_min
    q = state.in_transit_to(world, node, good)
    r = (state.stock[(node, good)] + q) / star
    if r <= 0.0:
        return good_p.m_max
    return F.clamp(r ** (-good_p.alpha), good_p.m_min, good_p.m_max)


def arbitrage(world: World, state: SimState, star: dict) -> None:
    """Перелив запасов по дугам, где выгода превышает стоимость пути (§5.6)."""
    p = world.params
    nodes = world.node_ids()
    goods = world.catalog.ids()

    # Ожидаемые множители по снимку начала шага.
    mhat = {
        (n, g): _expected_multiplier(world, state, n, g, star[(n, g)])
        for n in nodes for g in goods
    }

    # Доступно к вывозу (неснижаемый резерв ρ_res·s*).
    avail = {
        (n, g): max(0.0, state.stock[(n, g)] - p.rho_reserve * star[(n, g)])
        for n in nodes for g in goods
    }

    # Желаемый поток по каждой дуге и товару (u_eg).
    u: dict[tuple[str, str], float] = {}
    for e in world.edge_ids():
        edge = world.edges[e]
        gamma = p.c_gamma * edge.travel_time * edge.difficulty
        for g in goods:
            good_p = world.catalog[g]
            delta_over_p0 = mhat[(edge.dst, g)] - mhat[(edge.src, g)] - gamma * good_p.weight
            if delta_over_p0 <= 0.0:
                continue
            flow = p.kappa_trade * F.clamp(delta_over_p0, 0.0, 1.0) * avail[(edge.src, g)]
            if flow > 0.0:
                u[(e, g)] = flow

    # ψ_ig: не вывезти больше, чем доступно, суммируя по исходящим дугам (на товар).
    psi: dict[tuple[str, str], float] = {}
    for n in nodes:
        out_edges = world.outgoing(n)
        for g in goods:
            demand_sum = sum(u.get((e, g), 0.0) for e in out_edges)
            a = avail[(n, g)]
            psi[(n, g)] = min(1.0, a / demand_sum) if demand_sum > 0.0 else 1.0

    # φ_e: ограничение по ёмкости дороги (суммируя объём по товарам).
    for e in world.edge_ids():
        edge = world.edges[e]
        vol_sum = sum(
            world.catalog[g].volume * psi[(edge.src, g)] * u.get((e, g), 0.0)
            for g in goods
        )
        phi = min(1.0, edge.capacity / vol_sum) if vol_sum > 0.0 else 1.0
        tau = edge.delay_days()
        for g in goods:
            ueg = u.get((e, g), 0.0)
            if ueg <= 0.0:
                continue
            f = phi * psi[(edge.src, g)] * ueg
            if f <= 0.0:
                continue
            state.stock[(edge.src, g)] -= f
            add_to_pipeline(state, e, tau, g, f)


def dispatch(world: World, state: SimState, star: dict, produced: dict) -> None:
    """Шаг 6 целиком: сначала обязательные поставки в хаб, затем арбитраж."""
    hub_deliveries(world, state, produced)
    arbitrage(world, state, star)
