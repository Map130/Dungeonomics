//! Effect of a player's purchase/sale on the price (MODEL.md §5.7). Port of `ddeco/purchase.py`.
//!
//! A large deal moves the price on every unit; the player pays the average along the curve, not
//! the price tag — protection against buying out the market. The marginal multiplier after u units:
//! m·(s/(s−u))^α for a purchase. The [m_min, m_max] corridor applies at every point,
//! so we sum numerically over steps rather than use a closed form. After
//! the deal m is set to the marginal multiplier at the last unit,
//! then the usual smoothing takes over.

use crate::mathf::{self, clamp};
use crate::state::SimState;
use crate::world::World;

/// Details of an executed / computed deal.
#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    pub good: String,
    pub qty: f64,
    pub total: f64,          // total price of the deal (spread included)
    pub avg_price: f64,      // average price per unit
    pub new_multiplier: f64, // multiplier after the deal
}

/// Deal errors (instead of Python's `ValueError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeError {
    NonPositiveQty,  // qty must be > 0
    BuyExceedsStock, // cannot buy the whole stock (q < s)
}

/// Marginal multiplier after u units taken out (u>0 purchase, u<0 sale).
fn price_multiplier(m0: f64, s0: f64, u: f64, alpha: f64, m_min: f64, m_max: f64) -> f64 {
    let denom = s0 - u;
    if denom <= 0.0 {
        return m_max;
    }
    clamp(m0 * mathf::powf(s0 / denom, alpha), m_min, m_max)
}

/// Compute a deal WITHOUT changing the state (price preview for the GM).
///
/// `sell = false` — a purchase (takes stock out, u>0), `true` — a sale (adds, u<0).
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
    let sign = if sell { -1.0 } else { 1.0 }; // a purchase takes out (u>0), a sale adds (u<0)
    if !sell && qty >= s0 {
        return Err(TradeError::BuyExceedsStock);
    }

    let du = qty / steps as f64;
    let spread_factor = if sell { 1.0 - spread } else { 1.0 + spread };
    let mut total = 0.0;
    for i in 0..steps {
        let u = sign * (i as f64 + 0.5) * du; // middle of the step
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

/// Execute a deal: update the stock and the multiplier. Returns the deal details.
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
        assert!(t.new_multiplier > 1.0, "a purchase raises the multiplier");
        // The average price is above the price tag (base·spread) because the price rises along the curve.
        assert!(t.avg_price > 2.0 * 1.1);
        assert_eq!(st.stock[&("n".into(), "x".into())], 900.0);
        assert_eq!(st.multiplier[&("n".into(), "x".into())], t.new_multiplier);
    }

    #[test]
    fn sell_lowers_price_and_adds_stock() {
        let (w, mut st) = setup(1000.0);
        let t = apply(&w, &mut st, "n", "x", 100.0, true, 256).unwrap();
        assert!(t.new_multiplier < 1.0, "a sale lowers the multiplier");
        assert!(
            t.avg_price < 2.0,
            "the sale price is below the price tag (spread down)"
        );
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
            Err(TradeError::BuyExceedsStock) // cannot buy the whole stock
        );
    }
}
