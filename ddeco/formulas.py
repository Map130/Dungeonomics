"""Pure functions of the model (MODEL.md §5.1–5.8).

Every function is deterministic and does not touch the state — it only computes. The state
is moved by tick.py in the fixed §4 order. Notation — §2.
"""

from __future__ import annotations

from .goods import GoodParams


def clamp(x: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, x))


# §5.3 Demand and consumption --------------------------------------------------

def wealth_factor(good: GoodParams, prosperity: float, pi_ref: float) -> float:
    """c_g(π) = c0 + (1 − c0)·(π/π_ref)^κ — per-capita demand growing with wealth."""
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
    """D_ig = N_i·δc_g·c_g(π) + Garrison_ig — base demand before the price response."""
    return population * good.per_capita * wealth_factor(good, prosperity, pi_ref) + garrison


def consumption(good: GoodParams, demand: float, multiplier: float, stock: float) -> float:
    """d_ig = min(D·m^(−ε), s) — actual consumption, capped by the stock."""
    desired = demand * multiplier ** (-good.epsilon)
    return min(desired, stock)


# §5.1 Target stock ------------------------------------------------------------

def target_stock(good: GoodParams, demand: float) -> float:
    """s*_ig = T_g · D_ig."""
    return good.coverage_days * demand


# §5.2 Production (supply response to price) -----------------------------------

def production(good: GoodParams, pcap: float, multiplier: float, lo: float, hi: float) -> float:
    """p_ig = Pcap · clamp(m^β, lo, hi). The main closing loop against runaway growth."""
    return pcap * clamp(multiplier ** good.beta, lo, hi)


# §5.4 Spoilage ----------------------------------------------------------------

def after_spoilage(good: GoodParams, stock: float) -> float:
    """s ← s·(1 − δ)."""
    return stock * (1.0 - good.spoilage)


# §5.5 Price (multiplier) ------------------------------------------------------

def target_multiplier(good: GoodParams, stock: float, star: float) -> float:
    """m*_ig = clamp(r^(−α), m_min, m_max), r = s/s*.

    If s* = 0 (no demand), provisioning is infinite → the price sits at the lower bound."""
    if star <= 0.0:
        return good.m_min
    r = stock / star
    if r <= 0.0:
        return good.m_max
    return clamp(r ** (-good.alpha), good.m_min, good.m_max)


def smooth_multiplier(current: float, target: float, lam: float) -> float:
    """m ← m + λ·(m* − m) — smoothing, the multiplier drifts rather than jumps."""
    return current + lam * (target - current)


# §5.8 Prosperity --------------------------------------------------------------

def basket_fill(terms: list[tuple[float, float]]) -> float:
    """Basket fill Σ w_g·min(r_g, 1) / Σ w_g.

    terms — a list of (weight, r). A shortage of one good is not offset by a surplus
    of another (cap min(r,1)), so diversity counts. Normalised by
    the sum of the weights of the goods present."""
    total_w = 0.0
    acc = 0.0
    for w, r in terms:
        total_w += w
        acc += w * min(r, 1.0)
    return acc / total_w if total_w > 0.0 else 0.0


def prosperity_target(food_fill: float, market_fill: float, w_food: float, w_market: float) -> float:
    """π*_i = clamp(w_F·F_i + w_M·M_i, 0, 1)."""
    return clamp(w_food * food_fill + w_market * market_fill, 0.0, 1.0)


def smooth_prosperity(current: float, target: float, mu: float) -> float:
    """π ← π + μ·(π* − π) — a slow variable."""
    return current + mu * (target - current)
