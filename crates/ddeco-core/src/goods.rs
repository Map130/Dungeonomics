//! Справочник товаров и их параметры (MODEL.md §2, §7).
//!
//! Порт `ddeco/goods.py`. Числовые значения по умолчанию — стартовые догадки §7.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PriceClass {
    Necessity, // первая необходимость (еда и т.п.)
    Common,    // обычные товары
    Luxury,    // роскошь
}

/// Параметры одного товара (обозначения — §2).
#[derive(Debug, Clone, PartialEq)]
pub struct GoodParams {
    pub id: String,
    pub category: String,
    pub base_price: f64, // P0_g

    pub alpha: f64,         // α: обеспеченность r → множитель цены
    pub epsilon: f64,       // ε: эластичность спроса по цене
    pub beta: f64,          // β: эластичность предложения по цене
    pub m_min: f64,         // нижняя граница коридора
    pub m_max: f64,         // верхняя граница коридора
    pub spoilage: f64,      // δ: порча в день
    pub coverage_days: f64, // T: s* = T·D
    pub c0: f64,            // доля спроса вне богатства
    pub kappa: f64,         // κ: рост душевого спроса с π
    pub per_capita: f64,    // δc_g: душевое потребление при π_ref
    pub volume: f64,        // vol_g
    pub weight: f64,        // ω_g

    pub in_food_basket: bool, // вклад в F_i (иначе в M_i), §5.8
    pub basket_weight: f64,   // w_g / v_g
}

/// Стартовые параметры §7 для прайс-класса (α, ε, β, коридор, δ, T, c0, κ).
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

/// Параметры §7, которые задаются прайс-классом и могут переопределяться в TOML.
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

/// Справочник товаров мира. Обход — в отсортированном по id порядке (§9), что
/// обеспечивает `BTreeMap`.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub goods: BTreeMap<String, GoodParams>,
}

impl Catalog {
    pub fn get(&self, id: &str) -> &GoodParams {
        &self.goods[id]
    }

    /// Детерминированный порядок товаров.
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
