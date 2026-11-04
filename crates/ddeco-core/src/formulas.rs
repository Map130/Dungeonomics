//! Чистые функции модели (MODEL.md §5.1–5.8). Порт `ddeco/formulas.py`.
//!
//! Степени/экспоненты — только через [`crate::mathf`] (детерминизм §9).

use crate::goods::GoodParams;
use crate::mathf::{self, clamp};

// §5.3 Спрос и потребление ----------------------------------------------------

/// c_g(π) = c0 + (1 − c0)·(π/π_ref)^κ.
pub fn wealth_factor(g: &GoodParams, prosperity: f64, pi_ref: f64) -> f64 {
    let base = if prosperity <= 0.0 {
        0.0
    } else {
        mathf::powf(prosperity / pi_ref, g.kappa)
    };
    g.c0 + (1.0 - g.c0) * base
}

/// D_ig = N_i·δc_g·c_g(π) + Garrison_ig.
pub fn base_demand(
    g: &GoodParams,
    population: f64,
    prosperity: f64,
    pi_ref: f64,
    garrison: f64,
) -> f64 {
    population * g.per_capita * wealth_factor(g, prosperity, pi_ref) + garrison
}

/// d_ig = min(D·m^(−ε), s).
pub fn consumption(g: &GoodParams, demand: f64, multiplier: f64, stock: f64) -> f64 {
    let desired = demand * mathf::powf(multiplier, -g.epsilon);
    desired.min(stock)
}

// §5.1 Целевой запас ----------------------------------------------------------

pub fn target_stock(g: &GoodParams, demand: f64) -> f64 {
    g.coverage_days * demand
}

// §5.2 Производство (реакция предложения на цену) -----------------------------

pub fn production(g: &GoodParams, pcap: f64, multiplier: f64, lo: f64, hi: f64) -> f64 {
    pcap * clamp(mathf::powf(multiplier, g.beta), lo, hi)
}

// §5.4 Порча ------------------------------------------------------------------

pub fn after_spoilage(g: &GoodParams, stock: f64) -> f64 {
    stock * (1.0 - g.spoilage)
}

// §5.5 Цена (множитель) -------------------------------------------------------

/// m*_ig = clamp(r^(−α), m_min, m_max), r = s/s*.
pub fn target_multiplier(g: &GoodParams, stock: f64, star: f64) -> f64 {
    if star <= 0.0 {
        return g.m_min;
    }
    let r = stock / star;
    if r <= 0.0 {
        return g.m_max;
    }
    clamp(mathf::powf(r, -g.alpha), g.m_min, g.m_max)
}

pub fn smooth_multiplier(current: f64, target: f64, lam: f64) -> f64 {
    current + lam * (target - current)
}

// §5.8 Процветание ------------------------------------------------------------

/// Σ w_g·min(r_g, 1) / Σ w_g по списку (weight, r).
pub fn basket_fill(terms: &[(f64, f64)]) -> f64 {
    let mut total_w = 0.0;
    let mut acc = 0.0;
    for &(w, r) in terms {
        total_w += w;
        acc += w * r.min(1.0);
    }
    if total_w > 0.0 {
        acc / total_w
    } else {
        0.0
    }
}

/// π*_i = clamp(w_F·F_i + w_M·M_i, 0, 1).
pub fn prosperity_target(food_fill: f64, market_fill: f64, w_food: f64, w_market: f64) -> f64 {
    clamp(w_food * food_fill + w_market * market_fill, 0.0, 1.0)
}

pub fn smooth_prosperity(current: f64, target: f64, mu: f64) -> f64 {
    current + mu * (target - current)
}
