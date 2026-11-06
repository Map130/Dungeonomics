//! Влияние покупки/продажи игрока на цену (MODEL.md §5.7). Порт `ddeco/purchase.py`.
//!
//! Крупная сделка сдвигает цену на каждой единице, игрок платит среднюю по кривой, а
//! не цену с ценника — защита от скупки. Предельный множитель после u единиц:
//! m·(s/(s−u))^α при покупке. Коридор [m_min, m_max] применяется в каждой точке,
//! поэтому считаем численным суммированием по шагам, а не замкнутой формулой. После
//! сделки m устанавливается равным предельному множителю на последней единице,
//! дальше работает обычное сглаживание.

use crate::mathf::{self, clamp};
use crate::state::SimState;
use crate::world::World;

/// Детали проведённой/посчитанной сделки.
#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    pub good: String,
    pub qty: f64,
    pub total: f64,          // суммарная цена сделки (с учётом спреда)
    pub avg_price: f64,      // средняя цена за единицу
    pub new_multiplier: f64, // множитель после сделки
}

/// Ошибки сделки (вместо Python `ValueError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeError {
    NonPositiveQty,  // qty должно быть > 0
    BuyExceedsStock, // нельзя купить весь запас (q < s)
}

/// Предельный множитель после u изъятых единиц (u>0 покупка, u<0 продажа).
fn price_multiplier(m0: f64, s0: f64, u: f64, alpha: f64, m_min: f64, m_max: f64) -> f64 {
    let denom = s0 - u;
    if denom <= 0.0 {
        return m_max;
    }
    clamp(m0 * mathf::powf(s0 / denom, alpha), m_min, m_max)
}

/// Посчитать сделку, НЕ меняя состояние (для предпросмотра цены мастеру).
///
/// `sell = false` — покупка (изымает запас, u>0), `true` — продажа (добавляет, u<0).
pub fn quote(
    world: &World,
    state: &SimState,
    node: &str,
    good: &str,
    qty: f64,
    sell: bool,
    steps: u32,
) -> Result<Trade, TradeError> {
    if qty <= 0.0 {
        return Err(TradeError::NonPositiveQty);
    }
    let g = world.catalog.get(good);
    let key = (node.to_string(), good.to_string());
    let s0 = state.stock[&key];
    let m0 = state.multiplier[&key];
    let spread = world.nodes[node].spread;
    let sign = if sell { -1.0 } else { 1.0 }; // покупка изымает (u>0), продажа добавляет (u<0)
    if !sell && qty >= s0 {
        return Err(TradeError::BuyExceedsStock);
    }

    let du = qty / steps as f64;
    let spread_factor = if sell { 1.0 - spread } else { 1.0 + spread };
    let mut total = 0.0;
    for i in 0..steps {
        let u = sign * (i as f64 + 0.5) * du; // середина шага
        let m = price_multiplier(m0, s0, u, g.alpha, g.m_min, g.m_max);
        let unit = g.base_price * m * spread_factor;
        total += unit * du;
    }
    let new_m = price_multiplier(m0, s0, sign * qty, g.alpha, g.m_min, g.m_max);
    Ok(Trade {
        good: good.to_string(),
        qty,
        total,
        avg_price: total / qty,
        new_multiplier: new_m,
    })
}

/// Провести сделку: обновить запас и множитель. Возвращает детали сделки.
pub fn apply(
    world: &World,
    state: &mut SimState,
    node: &str,
    good: &str,
    qty: f64,
    sell: bool,
    steps: u32,
) -> Result<Trade, TradeError> {
    let trade = quote(world, state, node, good, qty, sell, steps)?;
    let key = (node.to_string(), good.to_string());
    let delta = if sell { qty } else { -qty };
    let s = (state.stock[&key] + delta).max(0.0);
    state.stock.insert(key.clone(), s);
    state.multiplier.insert(key, trade.new_multiplier);
    Ok(trade)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goods::{Catalog, GoodParams};
    use crate::state::blank_state;
    use crate::world::{GlobalParams, Node, NodeKind};
    use std::collections::BTreeMap;

    fn setup(stock: f64) -> (World, SimState) {
        let good = GoodParams {
            id: "x".into(),
            category: "test".into(),
            base_price: 2.0,
            alpha: 1.0,
            epsilon: 0.6,
            beta: 0.3,
            m_min: 0.5,
            m_max: 3.0,
            spoilage: 0.0,
            coverage_days: 10.0,
            c0: 1.0,
            kappa: 0.0,
            per_capita: 1.0,
            volume: 1.0,
            weight: 1.0,
            in_food_basket: false,
            basket_weight: 0.0,
        };
        let node = Node {
            id: "n".into(),
            kind: NodeKind::City,
            population: 100.0,
            prosperity0: 0.5,
            spread: 0.1,
            hub: None,
            hub_share: 0.6,
            capacity: BTreeMap::new(),
            garrison: BTreeMap::new(),
        };
        let world = World {
            catalog: Catalog {
                goods: BTreeMap::from([("x".to_string(), good)]),
            },
            nodes: BTreeMap::from([("n".to_string(), node)]),
            edges: BTreeMap::new(),
            params: GlobalParams::default(),
            seed: 0,
        };
        let mut st = blank_state(&world);
        st.stock.insert(("n".into(), "x".into()), stock);
        (world, st)
    }

    #[test]
    fn buy_raises_price_and_drains_stock() {
        let (w, mut st) = setup(1000.0);
        let t = apply(&w, &mut st, "n", "x", 100.0, false, 256).unwrap();
        assert!(t.new_multiplier > 1.0, "покупка поднимает множитель");
        // Средняя цена выше ценника (база·спред), т.к. цена растёт по кривой.
        assert!(t.avg_price > 2.0 * 1.1);
        assert_eq!(st.stock[&("n".into(), "x".into())], 900.0);
        assert_eq!(st.multiplier[&("n".into(), "x".into())], t.new_multiplier);
    }

    #[test]
    fn sell_lowers_price_and_adds_stock() {
        let (w, mut st) = setup(1000.0);
        let t = apply(&w, &mut st, "n", "x", 100.0, true, 256).unwrap();
        assert!(t.new_multiplier < 1.0, "продажа опускает множитель");
        assert!(t.avg_price < 2.0, "цена продажи ниже ценника (спред вниз)");
        assert_eq!(st.stock[&("n".into(), "x".into())], 1100.0);
    }

    #[test]
    fn invalid_trades_error() {
        let (w, st) = setup(50.0);
        assert_eq!(
            quote(&w, &st, "n", "x", 0.0, false, 256),
            Err(TradeError::NonPositiveQty)
        );
        assert_eq!(
            quote(&w, &st, "n", "x", 50.0, false, 256),
            Err(TradeError::BuyExceedsStock) // нельзя купить весь запас
        );
    }
}
