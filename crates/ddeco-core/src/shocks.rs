//! Background shocks (MODEL.md §5.11). Port of `ddeco/shocks.py`.
//!
//! For each template and each matching target, an effect is created on a day with
//! probability 1 − exp(−rate). The RNG is counter-based (bit-exact with Python), so undoing
//! and replaying a day gives the same shocks. Pass an empty template list to turn the generator off.

use crate::effects::{Decay, Effect, Op, TargetKind};
use crate::mathf;
use crate::rng;
use crate::state::SimState;
use crate::world::World;

/// Shock template: which effect, on which targets, at what rate.
#[derive(Debug, Clone)]
pub struct ShockTemplate {
    pub id: String,
    pub target_kind: TargetKind,
    pub param: String,
    pub op: Op,
    pub value: f64,
    pub duration: f64,
    pub rate: f64, // rate per day (§7: of order 1/120)
    pub decay: Decay,
    pub tau: f64,
    pub good: Option<String>,
    pub category: Option<String>,
    pub source: String,
}

impl ShockTemplate {
    /// Probability of firing on a day: 1 − exp(−rate).
    pub fn probability(&self) -> f64 {
        1.0 - mathf::exp(-self.rate)
    }
}

/// Create effects from the templates that fire on the current day (§4, step 0).
///
/// Templates are visited in list order, targets in sorted id order (§9).
pub fn generate(world: &World, state: &SimState, templates: &[ShockTemplate]) -> Vec<Effect> {
    let day = state.day;
    let mut new = Vec::new();
    for tpl in templates {
        let p = tpl.probability();
        let targets: Vec<&String> = match tpl.target_kind {
            TargetKind::Node => world.node_ids().collect(),
            TargetKind::Edge => world.edge_ids().collect(),
        };
        for target_id in targets {
            // Order of the parts (target, template) — as in Python (ddeco/shocks.py).
            if !rng::chance(world.seed, day, p, &[target_id.as_str(), tpl.id.as_str()]) {
                continue;
            }
            new.push(Effect {
                param: tpl.param.clone(),
                target_kind: tpl.target_kind,
                target_id: target_id.clone(),
                op: tpl.op,
                value: tpl.value,
                start_day: day,
                duration: tpl.duration,
                decay: tpl.decay,
                tau: tpl.tau,
                good: tpl.good.clone(),
                category: tpl.category.clone(),
                source: if tpl.source.is_empty() {
                    tpl.id.clone()
                } else {
                    tpl.source.clone()
                },
            });
        }
    }
    new
}
