//! Goods catalogue and their parameters (MODEL.md §2, §7).
//!
//! Port of `ddeco/goods.py`. Numeric defaults are the §7 starting guesses.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PriceClass {
    Necessity, // necessities (food etc.)
    Common,    // ordinary goods
    Luxury,    // luxury
}

/// Parameters of one good (notation — §2).
#[derive(Debug, Clone, PartialEq)]
pub struct GoodParams {
    pub id: String,
    pub category: String,
    pub base_price: f64, // P0_g

    pub alpha: f64,         // α: provisioning r → price multiplier
    pub epsilon: f64,       // ε: price elasticity of demand
    pub beta: f64,          // β: price elasticity of supply
    pub m_min: f64,         // lower bound of the corridor
    pub m_max: f64,         // upper bound of the corridor
    pub spoilage: f64,      // δ: spoilage per day
    pub coverage_days: f64, // T: s* = T·D
    pub c0: f64,            // share of demand independent of wealth
    pub kappa: f64,         // κ: growth of per-capita demand with π
    pub per_capita: f64,    // δc_g: per-capita consumption at π_ref
    pub volume: f64,        // vol_g
    pub weight: f64,        // ω_g

    pub in_food_basket: bool, // counts towards F_i (else M_i), §5.8
    pub basket_weight: f64,   // w_g / v_g
}

/// §7 starting parameters for a price class (α, ε, β, corridor, δ, T, c0, κ).
pub fn defaults_for(pc: PriceClass) -> ClassDefaults {
    match pc {
        PriceClass::Necessity => ClassDefaults {
            alpha: 0.5,
            epsilon: 0.2,
            beta: 0.3,
            m_min: 0.7,
            m_max: 1.8,
            spoilage: 0.01,
            coverage_days: 10.0,
            c0: 0.8,
            kappa: 0.3,
        },
        PriceClass::Common => ClassDefaults {
            alpha: 1.0,
            epsilon: 0.6,
            beta: 0.3,
            m_min: 0.5,
            m_max: 3.0,
            spoilage: 0.002,
            coverage_days: 10.0,
            c0: 0.4,
            kappa: 1.0,
        },
        PriceClass::Luxury => ClassDefaults {
            alpha: 1.5,
            epsilon: 1.5,
            beta: 0.3,
            m_min: 0.4,
            m_max: 3.0,
            spoilage: 0.0,
            coverage_days: 14.0,
            c0: 0.0,
            kappa: 2.0,
        },
    }
}

/// §7 parameters set by the price class that may be overridden in TOML.
#[derive(Debug, Clone, Copy)]
pub struct ClassDefaults {
    pub alpha: f64,
    pub epsilon: f64,
    pub beta: f64,
    pub m_min: f64,
    pub m_max: f64,
    pub spoilage: f64,
    pub coverage_days: f64,
    pub c0: f64,
    pub kappa: f64,
}

/// The world's goods catalogue. Iteration in id-sorted order (§9), which
/// `BTreeMap` guarantees.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub goods: BTreeMap<String, GoodParams>,
}

impl Catalog {
    pub fn get(&self, id: &str) -> &GoodParams {
        &self.goods[id]
    }

    /// Deterministic order of goods.
    pub fn ids(&self) -> impl Iterator<Item = &String> {
        self.goods.keys()
    }

    pub fn food_ids(&self) -> impl Iterator<Item = &String> {
        self.goods
            .iter()
            .filter(|(_, g)| g.in_food_basket)
            .map(|(id, _)| id)
    }

    pub fn market_ids(&self) -> impl Iterator<Item = &String> {
        self.goods
            .iter()
            .filter(|(_, g)| !g.in_food_basket)
            .map(|(id, _)| id)
    }
}
