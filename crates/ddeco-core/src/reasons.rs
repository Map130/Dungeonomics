//! Лог причин (MODEL.md §5.12). Порт `ddeco/reasons.py`.
//!
//! Для каждого изменения цены система называет причину. Цепочка:
//!     цена ← обеспеченность r ← {производство, спрос, потоки, порча}
//!           ← {активные эффекты, сложность дорог}.
//!
//! Лента показывает обеспеченность узла и главные причины по величине вклада, с
//! источником эффекта. Вклад оцениваем по относительному отклонению параметра,
//! наведённому эффектом (в v1 — без точного контрфактуала; граф мал, можно усилить
//! позже).

use crate::effects::{Effect, Op, TargetKind};
use crate::formulas as f;
use crate::state::SimState;
use crate::world::World;

/// Один вклад в объяснение цены.
#[derive(Debug, Clone, PartialEq)]
pub struct Cause {
    pub magnitude: f64, // величина вклада (для ранжирования)
    pub text: String,   // человекочитаемое описание с источником
}

/// Отчёт по множителю цены товара в узле.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceReport {
    pub node: String,
    pub good: String,
    pub multiplier: f64,
    pub provisioning: f64, // r = s/s*
    pub verdict: String,   // "дефицит" | "избыток" | "норма"
    pub causes: Vec<Cause>,
}

impl PriceReport {
    /// Тексты n главных причин.
    pub fn top(&self, n: usize) -> Vec<String> {
        self.causes.iter().take(n).map(|c| c.text.clone()).collect()
    }
}

/// Мгновенный вклад эффекта и его текстовое представление.
fn effect_factor(e: &Effect, day: i64) -> (f64, String) {
    let w = e.weight(day);
    match e.op {
        Op::Mult => {
            let factor = 1.0 + (e.value - 1.0) * w;
            ((factor - 1.0).abs(), format!("×{factor:.2}"))
        }
        Op::Add => ((e.value * w).abs(), format!("+{:.1}", e.value * w)),
    }
}

/// Текстовая метка для параметра узла (повторяет словарь Python-эталона).
fn node_label(param: &str) -> String {
    match param {
        "pcap" => "производство",
        "garrison" => "гарнизон",
        "population" => "население",
        other => other,
    }
    .to_string()
}

/// Объяснить текущий множитель цены товара в узле (§5.12).
pub fn explain_price(world: &World, state: &SimState, node: &str, good: &str) -> PriceReport {
    let g = world.catalog.get(good);
    let day = state.day;
    let key = (node.to_string(), good.to_string());
    let s = state.stock[&key];

    // Обеспеченность r считаем по базовому спросу (без эффектов — для вердикта).
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
        "дефицит"
    } else if r > 1.1 {
        "избыток"
    } else {
        "норма"
    };

    let inbound: Vec<String> = world.incoming(node).cloned().collect();
    let mut causes: Vec<Cause> = Vec::new();
    for e in &state.effects {
        if e.weight(day) == 0.0 {
            continue;
        }
        let label = if e.target_kind == TargetKind::Node && e.target_id == node {
            if (e.param == "pcap" || e.param == "garrison")
                && !e.matches_good(Some(good), Some(&g.category))
            {
                continue;
            }
            node_label(&e.param)
        } else if e.target_kind == TargetKind::Edge
            && inbound.contains(&e.target_id)
            && (e.param == "difficulty" || e.param == "capacity")
        {
            let edge = &world.edges[&e.target_id];
            format!("дорога {}→{}", edge.src, edge.dst)
        } else {
            continue;
        };
        let (mag, factor) = effect_factor(e, day);
        let src = if e.source.is_empty() {
            String::new()
        } else {
            format!(" ({})", e.source)
        };
        causes.push(Cause {
            magnitude: mag,
            text: format!("{label} {factor}{src}"),
        });
    }

    // Если эффектов нет, причина — сам баланс запаса.
    if causes.is_empty() {
        causes.push(Cause {
            magnitude: (r - 1.0).abs(),
            text: format!("запас {:.0}% от целевого", r * 100.0),
        });
    }

    // Сортировка по вкладу, стабильная (как у Python) — порядок равных сохраняется.
    causes.sort_by(|a, b| b.magnitude.partial_cmp(&a.magnitude).unwrap());

    PriceReport {
        node: node.to_string(),
        good: good.to_string(),
        multiplier: state.multiplier[&key],
        provisioning: r,
        verdict: verdict.to_string(),
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
            source: "неурожай".into(),
        }
    }

    #[test]
    fn verdict_follows_provisioning() {
        let w = world();
        let mut st = blank_state(&w);
        // star = coverage_days·demand = 10·100 = 1000.
        st.stock.insert(("n".into(), "grain".into()), 500.0); // r = 0.5
        let rep = explain_price(&w, &st, "n", "grain");
        assert_eq!(rep.verdict, "дефицит");
        assert!((rep.provisioning - 0.5).abs() < 1e-12);
    }

    #[test]
    fn active_effect_named_as_cause() {
        let w = world();
        let mut st = blank_state(&w);
        st.stock.insert(("n".into(), "grain".into()), 1000.0);
        st.effects.push(blight(0.3));
        let rep = explain_price(&w, &st, "n", "grain");
        assert_eq!(
            rep.top(1),
            vec!["производство ×0.30 (неурожай)".to_string()]
        );
    }

    #[test]
    fn stock_balance_is_fallback_when_no_effects() {
        let w = world();
        let mut st = blank_state(&w);
        st.stock.insert(("n".into(), "grain".into()), 1000.0); // r = 1.0
        let rep = explain_price(&w, &st, "n", "grain");
        assert_eq!(rep.verdict, "норма");
        assert_eq!(rep.top(3), vec!["запас 100% от целевого".to_string()]);
    }
}
