//! Parametric effects and their composition (MODEL.md §3). Port of `ddeco/effects.py`.
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
    Step,   // 1 until the end of the duration, then 0
    Linear, // max(0, 1 − age/duration)
    Exp,    // exp(−age/τ)
}

/// A single parametric effect (§3). `duration` is f64 to allow infinity.
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
    /// Decay weight w_k(t) on the given day (§3).
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

    /// The effect can be removed: its weight is 0 from this day on (§4, step 1).
    pub fn expired(&self, day: i64) -> bool {
        (day - self.start_day) as f64 >= self.duration
    }

    /// Whether the effect applies to this good (good/category filters).
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

/// Effective parameter value after all applicable effects (§3).
///
/// The order does not affect the result: multipliers multiply, addends are summed
/// in the order of the `effects` list (the caller keeps it deterministic).
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
    // 0 · inf = 0: a zero factor closes an unlimited base instead of giving NaN.
    let scaled = if mult == 0.0 { 0.0 } else { base * mult };
    mathf::clamp(scaled + add, lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_multiplier_on_infinite_base_is_zero() {
        let e = Effect {
            param: "capacity".into(),
            target_kind: TargetKind::Edge,
            target_id: "a__b".into(),
            op: Op::Mult,
            value: 0.0,
            start_day: 0,
            duration: 10.0,
            decay: Decay::Step,
            tau: 1.0,
            good: None,
            category: None,
            source: "t".into(),
        };
        let v = effective(
            f64::INFINITY,
            &[e],
            "capacity",
            TargetKind::Edge,
            "a__b",
            0,
            None,
            None,
            0.0,
            f64::INFINITY,
        );
        assert_eq!(v, 0.0);
    }
}
