//! Параметрические эффекты и их композиция (MODEL.md §3). Порт `ddeco/effects.py`.
//!
//! x_eff = clamp( x_base · Π_k (1 + (mult_k − 1)·w_k) + Σ_k add_k·w_k , lo, hi ).

use crate::mathf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Node,
    Edge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Mult,
    Add,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decay {
    Step,   // 1 до конца длительности, затем 0
    Linear, // max(0, 1 − age/duration)
    Exp,    // exp(−age/τ)
}

/// Один параметрический эффект (§3). `duration` — f64, чтобы допускать бесконечность.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    pub param: String,
    pub target_kind: TargetKind,
    pub target_id: String,
    pub op: Op,
    pub value: f64,
    pub start_day: i64,
    pub duration: f64,
    pub decay: Decay,
    pub tau: f64,
    pub good: Option<String>,
    pub category: Option<String>,
    pub source: String,
}

impl Effect {
    /// Вес затухания w_k(t) на заданный день (§3).
    pub fn weight(&self, day: i64) -> f64 {
        let age = (day - self.start_day) as f64;
        if age < 0.0 || age >= self.duration {
            return 0.0;
        }
        match self.decay {
            Decay::Step => 1.0,
            Decay::Linear => (1.0 - age / self.duration).max(0.0),
            Decay::Exp => mathf::exp(-age / self.tau),
        }
    }

    /// Эффект можно удалить: его вес уже всегда 0 на этот день и далее (§4, шаг 1).
    pub fn expired(&self, day: i64) -> bool {
        (day - self.start_day) as f64 >= self.duration
    }

    /// Применим ли эффект к данному товару (фильтры good/category).
    pub fn matches_good(&self, good_id: Option<&str>, category: Option<&str>) -> bool {
        if let Some(g) = &self.good {
            return Some(g.as_str()) == good_id;
        }
        if let Some(c) = &self.category {
            return Some(c.as_str()) == category;
        }
        true
    }
}

/// Эффективное значение параметра после всех применимых эффектов (§3).
///
/// Порядок обхода не влияет на итог: множители перемножаются, добавки суммируются
/// в порядке списка `effects` (его детерминированность обеспечивает вызывающий).
#[allow(clippy::too_many_arguments)]
pub fn effective(
    base: f64,
    effects: &[Effect],
    param: &str,
    target_kind: TargetKind,
    target_id: &str,
    day: i64,
    good_id: Option<&str>,
    category: Option<&str>,
    lo: f64,
    hi: f64,
) -> f64 {
    let mut mult = 1.0;
    let mut add = 0.0;
    for e in effects {
        if e.param != param
            || e.target_kind != target_kind
            || e.target_id != target_id
            || !e.matches_good(good_id, category)
        {
            continue;
        }
        let w = e.weight(day);
        if w == 0.0 {
            continue;
        }
        match e.op {
            Op::Mult => mult *= 1.0 + (e.value - 1.0) * w,
            Op::Add => add += e.value * w,
        }
    }
    mathf::clamp(base * mult + add, lo, hi)
}
