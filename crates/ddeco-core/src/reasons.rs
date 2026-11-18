//! Reasons log (MODEL.md §5.12). Port of `ddeco/reasons.py`.
//!
//! For every price change the system names a cause. The chain is:
//!     price ← provisioning r ← {production, demand, flows, spoilage}
//!           ← {active effects, road difficulty}.
//!
//! The report gives the node's provisioning and the main causes ranked by contribution,
//! with the effect's source. A contribution is estimated by the relative deviation the
//! effect induces in its parameter (no exact counterfactual in v1).
//!
//! The core emits **structured** causes only (D5): a kind plus parameters. Rendering to
//! text (and translation) is the UI's job, so the core contains no natural-language strings.

use crate::effects::{Effect, Op, TargetKind};
use crate::formulas as f;
use crate::state::SimState;
use crate::world::World;

/// Verdict by provisioning r = s/s*: below 0.9 — shortage, above 1.1 — surplus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Shortage,
    Surplus,
    Normal,
}

impl Verdict {
    /// Stable code shared with the Python oracle and the UI.
    pub fn code(self) -> &'static str {
        match self {
            Verdict::Shortage => "shortage",
            Verdict::Surplus => "surplus",
            Verdict::Normal => "normal",
        }
    }
}

/// What a cause refers to.
#[derive(Debug, Clone, PartialEq)]
pub enum CauseKind {
    /// An effect on a node parameter (pcap, garrison, population, ...).
    NodeEffect,
    /// An effect on an inbound road (difficulty, capacity) `src→dst`.
    EdgeEffect { src: String, dst: String },
    /// No active effects: the stock balance itself (`value` = r).
    Stock,
}

impl CauseKind {
    pub fn code(&self) -> &'static str {
        match self {
            CauseKind::NodeEffect => "node_effect",
            CauseKind::EdgeEffect { .. } => "edge_effect",
            CauseKind::Stock => "stock",
        }
    }
}

/// One contribution to a price explanation.
#[derive(Debug, Clone, PartialEq)]
pub struct Cause {
    pub kind: CauseKind,
    pub magnitude: f64, // size of the contribution (for ranking)
    pub value: f64,     // Mult: current factor; Add: current addend; Stock: r
    pub op: Option<Op>, // None for Stock
    pub param: String,  // effect parameter, empty for Stock
    pub source: String, // free-text origin of the effect (GM / template)
}

/// Report on a good's price multiplier at a node.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceReport {
    pub node: String,
    pub good: String,
    pub multiplier: f64,
    pub provisioning: f64, // r = s/s*
    pub verdict: Verdict,
    pub causes: Vec<Cause>,
}

impl PriceReport {
    /// The n main causes.
    pub fn top(&self, n: usize) -> &[Cause] {
        &self.causes[..n.min(self.causes.len())]
    }
}

/// Current contribution of an effect: (magnitude, value).
fn effect_factor(e: &Effect, day: i64) -> (f64, f64) {
    let w = e.weight(day);
    match e.op {
        Op::Mult => {
            let factor = 1.0 + (e.value - 1.0) * w;
            ((factor - 1.0).abs(), factor)
        }
        Op::Add => ((e.value * w).abs(), e.value * w),
    }
}

/// Explain the current price multiplier of a good at a node (§5.12).
pub fn explain_price(world: &World, state: &SimState, node: &str, good: &str) -> PriceReport {
    let g = world.catalog.get(good);
    let day = state.day;
    let key = (node.to_string(), good.to_string());
    let s = state.stock[&key];

    // Provisioning r uses the base demand (without effects — for the verdict).
    let nd = &world.nodes[node];
    let demand = f::base_demand(
        g,
        nd.population,
        state.prosperity[node],
        world.params.pi_ref,
        nd.garrison_of(good),
    );
    let star = f::target_stock(g, demand);
    let r = if star > 0.0 { s / star } else { f64::INFINITY };
    let verdict = if r < 0.9 {
        Verdict::Shortage
    } else if r > 1.1 {
        Verdict::Surplus
    } else {
        Verdict::Normal
    };

    let inbound: Vec<String> = world.incoming(node).cloned().collect();
    let mut causes: Vec<Cause> = Vec::new();
    for e in &state.effects {
        if e.weight(day) == 0.0 {
            continue;
        }
        let kind = if e.target_kind == TargetKind::Node && e.target_id == node {
            if (e.param == "pcap" || e.param == "garrison")
                && !e.matches_good(Some(good), Some(&g.category))
            {
                continue;
            }
            CauseKind::NodeEffect
        } else if e.target_kind == TargetKind::Edge
            && inbound.contains(&e.target_id)
            && (e.param == "difficulty" || e.param == "capacity")
        {
            let edge = &world.edges[&e.target_id];
            CauseKind::EdgeEffect {
                src: edge.src.clone(),
                dst: edge.dst.clone(),
            }
        } else {
            continue;
        };
        let (magnitude, value) = effect_factor(e, day);
        causes.push(Cause {
            kind,
            magnitude,
            value,
            op: Some(e.op),
            param: e.param.clone(),
            source: e.source.clone(),
        });
    }

    // With no effects, the cause is the stock balance itself.
    if causes.is_empty() {
        causes.push(Cause {
            kind: CauseKind::Stock,
            magnitude: (r - 1.0).abs(),
            value: r,
            op: None,
            param: String::new(),
            source: String::new(),
        });
    }

    // Stable sort by contribution (like Python) — equal items keep their order.
    causes.sort_by(|a, b| b.magnitude.partial_cmp(&a.magnitude).unwrap());

    PriceReport {
        node: node.to_string(),
        good: good.to_string(),
        multiplier: state.multiplier[&key],
        provisioning: r,
        verdict,
        causes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::Decay;
    use crate::goods::{Catalog, GoodParams};
    use crate::state::blank_state;
    use crate::world::{GlobalParams, Node, NodeKind};
    use std::collections::BTreeMap;

    fn world() -> World {
        let good = GoodParams {
            id: "grain".into(),
            category: "food".into(),
            base_price: 1.0,
            alpha: 0.5,
            epsilon: 0.2,
            beta: 0.3,
            m_min: 0.7,
            m_max: 1.8,
            spoilage: 0.0,
            coverage_days: 10.0,
            c0: 1.0,
            kappa: 0.0,
            per_capita: 1.0,
            volume: 1.0,
            weight: 1.0,
            in_food_basket: true,
            basket_weight: 1.0,
        };
        let node = Node {
            id: "n".into(),
            kind: NodeKind::City,
            population: 100.0,
            prosperity0: 0.5,
            spread: 0.1,
            hub: None,
            hub_share: 0.6,
            capacity: BTreeMap::from([("grain".to_string(), 100.0)]),
            garrison: BTreeMap::new(),
        };
        World {
            catalog: Catalog {
                goods: BTreeMap::from([("grain".to_string(), good)]),
            },
            nodes: BTreeMap::from([("n".to_string(), node)]),
            edges: BTreeMap::new(),
            params: GlobalParams::default(),
            seed: 0,
        }
    }

    fn blight(value: f64) -> Effect {
        Effect {
            param: "pcap".into(),
            target_kind: TargetKind::Node,
            target_id: "n".into(),
            op: Op::Mult,
            value,
            start_day: 0,
            duration: 100.0,
            decay: Decay::Step,
            tau: 1.0,
            good: None,
            category: Some("food".into()),
            source: "blight".into(),
        }
    }

    #[test]
    fn verdict_follows_provisioning() {
        let w = world();
        let mut st = blank_state(&w);
        // star = coverage_days·demand = 10·100 = 1000.
        st.stock.insert(("n".into(), "grain".into()), 500.0); // r = 0.5
        let rep = explain_price(&w, &st, "n", "grain");
        assert_eq!(rep.verdict, Verdict::Shortage);
        assert!((rep.provisioning - 0.5).abs() < 1e-12);
    }

    #[test]
    fn active_effect_named_as_cause() {
        let w = world();
        let mut st = blank_state(&w);
        st.stock.insert(("n".into(), "grain".into()), 1000.0);
        st.effects.push(blight(0.3));
        let rep = explain_price(&w, &st, "n", "grain");
        let c = &rep.top(1)[0];
        assert_eq!(c.kind, CauseKind::NodeEffect);
        assert_eq!((c.param.as_str(), c.op), ("pcap", Some(Op::Mult)));
        assert!((c.value - 0.3).abs() < 1e-12);
        assert_eq!(c.source, "blight");
    }

    #[test]
    fn stock_balance_is_fallback_when_no_effects() {
        let w = world();
        let mut st = blank_state(&w);
        st.stock.insert(("n".into(), "grain".into()), 1000.0); // r = 1.0
        let rep = explain_price(&w, &st, "n", "grain");
        assert_eq!(rep.verdict, Verdict::Normal);
        assert_eq!(rep.causes.len(), 1);
        assert_eq!(rep.causes[0].kind, CauseKind::Stock);
        assert!((rep.causes[0].value - 1.0).abs() < 1e-12);
    }
}
