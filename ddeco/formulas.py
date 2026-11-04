"""Чистые функции модели (MODEL.md §5.1–5.8).

Каждая функция детерминирована и не трогает состояние — только считает. Состояние
двигает tick.py в фиксированном порядке §4. Обозначения — §2.
"""

from __future__ import annotations

from .goods import GoodParams


def clamp(x: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, x))


# §5.3 Спрос и потребление ------------------------------------------------------

def wealth_factor(good: GoodParams, prosperity: float, pi_ref: float) -> float:
    """c_g(π) = c0 + (1 − c0)·(π/π_ref)^κ — рост душевого спроса с богатством."""
    if prosperity <= 0.0:
        base = 0.0
    else:
        base = (prosperity / pi_ref) ** good.kappa
    return good.c0 + (1.0 - good.c0) * base


def base_demand(
    good: GoodParams,
    population: float,
    prosperity: float,
    pi_ref: float,
    garrison: float = 0.0,
) -> float:
    """D_ig = N_i·δc_g·c_g(π) + Garrison_ig — базовый спрос до реакции на цену."""
    return population * good.per_capita * wealth_factor(good, prosperity, pi_ref) + garrison


def consumption(good: GoodParams, demand: float, multiplier: float, stock: float) -> float:
    """d_ig = min(D·m^(−ε), s) — фактическое потребление, ограничено запасом."""
    desired = demand * multiplier ** (-good.epsilon)
    return min(desired, stock)


# §5.1 Целевой запас ------------------------------------------------------------

def target_stock(good: GoodParams, demand: float) -> float:
    """s*_ig = T_g · D_ig."""
    return good.coverage_days * demand


# §5.2 Производство (реакция предложения на цену) -------------------------------

def production(good: GoodParams, pcap: float, multiplier: float, lo: float, hi: float) -> float:
    """p_ig = Pcap · clamp(m^β, lo, hi). Главный замыкающий контур против разгона."""
    return pcap * clamp(multiplier ** good.beta, lo, hi)


# §5.4 Порча --------------------------------------------------------------------

def after_spoilage(good: GoodParams, stock: float) -> float:
    """s ← s·(1 − δ)."""
    return stock * (1.0 - good.spoilage)


# §5.5 Цена (множитель) ---------------------------------------------------------

def target_multiplier(good: GoodParams, stock: float, star: float) -> float:
    """m*_ig = clamp(r^(−α), m_min, m_max), r = s/s*.

    Если s* = 0 (нет спроса), обеспеченность бесконечна → цена на нижней границе."""
    if star <= 0.0:
        return good.m_min
    r = stock / star
    if r <= 0.0:
        return good.m_max
    return clamp(r ** (-good.alpha), good.m_min, good.m_max)


def smooth_multiplier(current: float, target: float, lam: float) -> float:
    """m ← m + λ·(m* − m) — сглаживание, множитель плывёт, а не прыгает."""
    return current + lam * (target - current)
