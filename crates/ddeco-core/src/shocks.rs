//! Фоновые шоки (MODEL.md §5.11). Порт `ddeco/shocks.py`.
//!
//! Для каждого шаблона и каждой подходящей цели в день с вероятностью 1 − exp(−rate)
//! создаётся эффект. RNG счётчиковый (bit-exact с Python), поэтому откат и повтор дня
//! дают те же шоки. Генератор отключается передачей пустого списка шаблонов.

use crate::effects::{Decay, Effect, Op, TargetKind};
use crate::mathf;
use crate::rng;
use crate::state::SimState;
use crate::world::World;

/// Шаблон шока: какой эффект, на какие цели, с какой интенсивностью.
#[derive(Debug, Clone)]
pub struct ShockTemplate {
    pub id: String,
    pub target_kind: TargetKind,
    pub param: String,
    pub op: Op,
    pub value: f64,
    pub duration: f64,
    pub rate: f64, // интенсивность в день (§7: порядок 1/120)
    pub decay: Decay,
    pub tau: f64,
    pub good: Option<String>,
    pub category: Option<String>,
    pub source: String,
}

impl ShockTemplate {
    /// Вероятность срабатывания за день: 1 − exp(−rate).
    pub fn probability(&self) -> f64 {
        1.0 - mathf::exp(-self.rate)
    }
}

/// Создать эффекты от сработавших шаблонов на текущий день (§4, шаг 0).
///
/// Обход шаблонов — в порядке списка, целей — в отсортированном порядке id (§9).
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
            // Порядок частей (target, template) — как в Python (ddeco/shocks.py).
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
