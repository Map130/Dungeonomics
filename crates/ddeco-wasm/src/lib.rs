//! WASM bindings of the living-economy core (`ddeco-core`) for the web UI.
//!
//! A thin facade: the core stays the UI-independent reference (a port of the Python prototype),
//! and this crate only wraps its public API in [`Sim`] and turns the state
//! into plain JSON for JS. All simulation logic lives in the core; this is only glue.
//!
//! Invariants are kept as-is: the tick goes through [`ddeco_core::tick::step_day`],
//! undo uses in-memory `SimState` snapshots (determinism §8.4), I/O (files, clocks)
//! stays on the JS side — the facade works with TOML strings, like the core.

use ddeco_core::calibration::{seed_state, warmup};
use ddeco_core::effects::{Decay, Effect, Op, TargetKind};
use ddeco_core::loader::{shocks_from_toml, state_from_toml, state_to_toml, world_from_toml};
use ddeco_core::purchase::{self, TradeError};
use ddeco_core::shocks::ShockTemplate;
use ddeco_core::state::SimState;
use ddeco_core::world::{NodeKind, World};
use ddeco_core::{events, reasons, tick};

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Depth of the snapshot stack for `undo`. An evening of play is thousands of ticks, but
/// the GM needs only a few steps back; memory is capped by an upper limit.
const HISTORY_CAP: usize = 1024;

/// The simulation facade for the UI: owns the world, the state and the snapshot history.
#[wasm_bindgen]
pub struct Sim {
    world: World,
    state: SimState,
    history: Vec<SimState>,
    shocks: Vec<ShockTemplate>,
    shocks_enabled: bool,
}

#[wasm_bindgen]
impl Sim {
    /// Create a simulation from a TOML world description (schema — `worlds/example_region.toml`).
    ///
    /// `warmup_days > 0` — warm the world up to equilibrium without events (§5.14), otherwise take
    /// the calibrated starting snapshot without a run.
    #[wasm_bindgen(constructor)]
    pub fn new(world_toml: &str, warmup_days: u32) -> Result<Sim, JsValue> {
        console_error_panic_hook::set_once();
        let world = world_from_toml(world_toml).map_err(err)?;
        let shocks = shocks_from_toml(world_toml).map_err(|e| JsValue::from_str(&e))?;
        let state = if warmup_days > 0 {
            warmup(&world, warmup_days, 1e-6)
        } else {
            seed_state(&world)
        };
        Ok(Sim {
            world,
            state,
            history: Vec::new(),
            shocks,
            shocks_enabled: false,
        })
    }

    /// The current simulation day.
    #[wasm_bindgen(getter)]
    pub fn day(&self) -> i64 {
        self.state.day
    }

    /// Advance the simulation by one day (§4). Background shocks follow the toggle (default off).
    pub fn step(&mut self) {
        self.record();
        // Direct field access (disjoint borrows): templates and state do not overlap.
        let shocks: &[ShockTemplate] = if self.shocks_enabled {
            &self.shocks
        } else {
            &[]
        };
        tick::step_day(&self.world, &mut self.state, shocks);
    }

    /// Run `days` days in a row. One undo snapshot is kept for the whole run.
    pub fn run(&mut self, days: u32) {
        self.record();
        for _ in 0..days {
            let shocks: &[ShockTemplate] = if self.shocks_enabled {
                &self.shocks
            } else {
                &[]
            };
            tick::step_day(&self.world, &mut self.state, shocks);
        }
    }

    /// Turn the background shock generator on/off (§5.11). Off by default.
    /// If the world file has no `[[shocks]]` section, the toggle has no effect.
    pub fn set_shocks_enabled(&mut self, on: bool) {
        self.shocks_enabled = on;
    }

    /// Whether the background shock generator is on.
    #[wasm_bindgen(getter)]
    pub fn shocks_enabled(&self) -> bool {
        self.shocks_enabled
    }

    /// Undo the last state change (tick/run/event). `true` if there was
    /// something to undo. Replaying the same day after undo reproduces it bit for bit (§8.4).
    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some(prev) => {
                self.state = prev;
                true
            }
            None => false,
        }
    }

    // --- Snapshots for the UI ---------------------------------------------------

    /// World statics: nodes, goods, roads (to build the interface). Does not change.
    pub fn world_info(&self) -> Result<JsValue, JsValue> {
        let info = WorldInfo {
            seed: self.world.seed,
            nodes: self
                .world
                .nodes
                .values()
                .map(|n| NodeInfo {
                    id: n.id.clone(),
                    kind: kind_str(n.kind),
                    population: n.population,
                    spread: n.spread,
                    hub: n.hub.clone(),
                })
                .collect(),
            goods: self
                .world
                .catalog
                .ids()
                .map(|g| {
                    let good = self.world.catalog.get(g);
                    GoodInfo {
                        id: good.id.clone(),
                        category: good.category.clone(),
                        base_price: good.base_price,
                    }
                })
                .collect(),
            edges: self
                .world
                .edges
                .values()
                .map(|e| EdgeInfo {
                    id: e.id.clone(),
                    src: e.src.clone(),
                    dst: e.dst.clone(),
                    travel_time: e.travel_time,
                })
                .collect(),
        };
        to_js(&info)
    }

    /// Current state: the day, node prosperity and, per good, stock/multiplier/price.
    pub fn snapshot(&self) -> Result<JsValue, JsValue> {
        let nodes = self
            .world
            .nodes
            .values()
            .map(|n| NodeView {
                node: n.id.clone(),
                kind: kind_str(n.kind),
                population: n.population,
                prosperity: self.state.prosperity[&n.id],
                goods: self
                    .world
                    .catalog
                    .ids()
                    .map(|g| {
                        let good = self.world.catalog.get(g);
                        let key = (n.id.clone(), g.clone());
                        let m = self.state.multiplier[&key];
                        let price = good.base_price * m;
                        GoodView {
                            good: g.clone(),
                            stock: self.state.stock[&key],
                            multiplier: m,
                            price,
                            buy_price: price * (1.0 + n.spread),
                            sell_price: price * (1.0 - n.spread),
                            in_transit: self.state.in_transit_to(&self.world, &n.id, g),
                        }
                    })
                    .collect(),
            })
            .collect();
        to_js(&Snapshot {
            day: self.state.day,
            nodes,
        })
    }

    /// Explain a good's current price at a node: provisioning, verdict, cause chain (§5.12).
    pub fn explain(&self, node: &str, good: &str) -> Result<JsValue, JsValue> {
        let r = reasons::explain_price(&self.world, &self.state, node, good);
        to_js(&ReportView {
            node: r.node,
            good: r.good,
            multiplier: r.multiplier,
            provisioning: r.provisioning,
            verdict: r.verdict.code(),
            causes: r
                .causes
                .into_iter()
                .map(|c| {
                    let (src, dst) = match &c.kind {
                        reasons::CauseKind::EdgeEffect { src, dst } => {
                            (Some(src.clone()), Some(dst.clone()))
                        }
                        _ => (None, None),
                    };
                    CauseView {
                        kind: c.kind.code(),
                        magnitude: c.magnitude,
                        value: c.value,
                        op: c.op.map(|op| match op {
                            Op::Mult => "mult",
                            Op::Add => "add",
                        }),
                        param: c.param,
                        src,
                        dst,
                        source: c.source,
                    }
                })
                .collect(),
        })
    }

    // --- Parametric effects (§3, §5.8) ------------------------------------------

    /// Add a parametric effect from the UI (blight on `pcap`, a road ×2.5, etc.).
    ///
    /// `target_kind` — `"node"`/`"edge"`, `op` — `"mult"`/`"add"`, `decay` —
    /// `"step"`/`"linear"`/`"exp"`. `start_day` is the current day. `good`/`category` are
    /// optional good filters. Supports `undo`.
    #[allow(clippy::too_many_arguments)]
    pub fn add_effect(
        &mut self,
        target_kind: &str,
        target_id: &str,
        param: &str,
        op: &str,
        value: f64,
        duration: f64,
        decay: &str,
        tau: f64,
        good: Option<String>,
        category: Option<String>,
        source: &str,
    ) -> Result<(), JsValue> {
        let spec = EffectSpec {
            target_kind: target_kind.to_string(),
            target_id: target_id.to_string(),
            param: param.to_string(),
            op: op.to_string(),
            value,
            duration,
            decay: decay.to_string(),
            tau,
            good,
            category,
            source: source.to_string(),
        };
        let effect = self
            .build_effect(&spec)
            .map_err(|e| JsValue::from_str(&e))?;
        self.record();
        self.state.effects.push(effect);
        Ok(())
    }

    /// Add a batch of effects atomically (a preset): all specs are validated first, then one
    /// undo snapshot is taken. `specs` — an array of objects shaped like [`EffectSpec`].
    pub fn add_effects(&mut self, specs: JsValue) -> Result<(), JsValue> {
        let specs: Vec<EffectSpec> = serde_wasm_bindgen::from_value(specs).map_err(err)?;
        self.add_effect_specs(specs)
            .map_err(|e| JsValue::from_str(&e))
    }

    /// Feed of active effects (non-zero weight on the current day) with source, current
    /// factor and remaining duration — for the UI events panel (§5.8).
    pub fn effects(&self) -> Result<JsValue, JsValue> {
        let day = self.state.day;
        let list: Vec<EffectView> = self
            .state
            .effects
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let w = e.weight(day);
                if w == 0.0 {
                    return None;
                }
                let (factor, op) = match e.op {
                    Op::Mult => (format!("×{:.2}", 1.0 + (e.value - 1.0) * w), "mult"),
                    Op::Add => (format!("+{:.1}", e.value * w), "add"),
                };
                Some(EffectView {
                    index: i,
                    target_kind: kind_tag(e.target_kind),
                    target_id: e.target_id.clone(),
                    param: e.param.clone(),
                    op,
                    value: e.value,
                    good: e.good.clone(),
                    category: e.category.clone(),
                    source: e.source.clone(),
                    weight: w,
                    factor,
                    days_left: e.duration - (day - e.start_day) as f64,
                })
            })
            .collect();
        to_js(&list)
    }

    /// Remove an effect by its index from `effects()`. `true` if the index existed.
    /// Supports `undo`.
    pub fn remove_effect(&mut self, index: usize) -> bool {
        if index >= self.state.effects.len() {
            return false;
        }
        self.record();
        self.state.effects.remove(index);
        true
    }

    // --- Flows and caravans (§5.6, §5.10) ---------------------------------------

    /// Flows along arcs: cargo in transit and progress to the materialisation threshold θ_car (§5.8).
    pub fn flows(&self) -> Result<JsValue, JsValue> {
        let theta = self.world.params.caravan_threshold;
        let list: Vec<FlowView> = self
            .world
            .edges
            .values()
            .map(|e| {
                let in_transit = self
                    .state
                    .pipeline
                    .get(&e.id)
                    .map(|slots| slots.iter().flat_map(|s| s.values()).sum())
                    .unwrap_or(0.0);
                FlowView {
                    edge: e.id.clone(),
                    src: e.src.clone(),
                    dst: e.dst.clone(),
                    in_transit,
                    accum: self.state.caravan_accum.get(&e.id).copied().unwrap_or(0.0),
                    threshold: theta,
                }
            })
            .collect();
        to_js(&list)
    }

    /// Materialised named caravans (the UI feed, §5.10).
    pub fn caravans(&self) -> Result<JsValue, JsValue> {
        let list: Vec<CaravanView> = self
            .state
            .caravans
            .iter()
            .map(|c| CaravanView {
                edge: c.edge.clone(),
                day_departed: c.day_departed,
                day_arrival: c.day_arrival,
                value: c.value,
                guard: c.guard,
                src: self.world.edges[&c.edge].src.clone(),
                dst: self.world.edges[&c.edge].dst.clone(),
                cargo: c
                    .cargo
                    .iter()
                    .map(|(good, amount)| CargoView {
                        good: good.clone(),
                        amount: *amount,
                    })
                    .collect(),
            })
            .collect();
        to_js(&list)
    }

    // --- Player deals (§5.7) ----------------------------------------------------

    /// Compute a deal WITHOUT changing the state (price preview). `sell` — a sale by the player.
    pub fn quote(&self, node: &str, good: &str, qty: f64, sell: bool) -> Result<JsValue, JsValue> {
        let t = purchase::quote(&self.world, &self.state, node, good, qty, sell, 256)
            .map_err(trade_err)?;
        to_js(&TradeView::from(t))
    }

    /// Execute a deal: update the stock and the multiplier, return its details. Supports `undo`.
    pub fn trade(
        &mut self,
        node: &str,
        good: &str,
        qty: f64,
        sell: bool,
    ) -> Result<JsValue, JsValue> {
        // Validate before recording the snapshot so a failed deal does not litter the history.
        purchase::quote(&self.world, &self.state, node, good, qty, sell, 256).map_err(trade_err)?;
        self.record();
        let t = purchase::apply(&self.world, &mut self.state, node, good, qty, sell, 256)
            .map_err(trade_err)?;
        to_js(&TradeView::from(t))
    }

    // --- GM one-shot events (§5.8–5.9) ------------------------------------------

    /// The GM sets the stock of a good at a node (bought out / brought in).
    pub fn set_stock(&mut self, node: &str, good: &str, amount: f64) {
        self.record();
        events::set_stock(&mut self.state, node, good, amount);
    }

    /// Shift a stock by `delta` (may be negative), not below zero.
    pub fn add_stock(&mut self, node: &str, good: &str, delta: f64) {
        self.record();
        events::add_stock(&mut self.state, node, good, delta);
    }

    /// Prosperity hit on a node: π ← π·(1 − rho) (a raid, §5.8).
    pub fn prosperity_hit(&mut self, node: &str, rho: f64) {
        self.record();
        events::prosperity_hit(&mut self.state, node, rho);
    }

    /// Rob a caravan on an arc: remove a share `rho` of the cargo. `slot = -1` — all cargo in
    /// transit, otherwise a specific batch. Returns the total volume looted.
    pub fn loot_caravan(&mut self, edge: &str, rho: f64, slot: i32) -> f64 {
        self.record();
        let idx = if slot < 0 { None } else { Some(slot as usize) };
        events::loot_caravan(&mut self.state, edge, rho, idx)
    }

    // --- Saving/loading the state (§5.9) ----------------------------------------

    /// Serialise the current state to TOML (the GM saves it as a file). Writing is up to JS.
    pub fn save(&self) -> String {
        state_to_toml(&self.state)
    }

    /// Load the state from TOML, replacing the current one. The undo history is cleared.
    pub fn load_state(&mut self, state_toml: &str) -> Result<(), JsValue> {
        self.state = state_from_toml(state_toml).map_err(err)?;
        self.history.clear();
        Ok(())
    }

    // --- Internals --------------------------------------------------------------

    /// Record a snapshot before changing the state (for `undo`), respecting the history cap.
    fn record(&mut self) {
        if self.history.len() == HISTORY_CAP {
            self.history.remove(0);
        }
        self.history.push(self.state.clone());
    }
}

impl Sim {
    /// Validate a spec against the world and turn it into a core effect (starts today).
    fn build_effect(&self, spec: &EffectSpec) -> Result<Effect, String> {
        let target_kind = parse_target_kind(&spec.target_kind)?;
        let op = parse_op(&spec.op)?;
        let decay = parse_decay(&spec.decay)?;
        // The target must exist, otherwise the effect silently has no effect.
        match target_kind {
            TargetKind::Node if !self.world.nodes.contains_key(&spec.target_id) => {
                return Err(format!("no node {}", spec.target_id));
            }
            TargetKind::Edge if !self.world.edges.contains_key(&spec.target_id) => {
                return Err(format!("no arc {}", spec.target_id));
            }
            _ => {}
        }
        if let Some(g) = &spec.good {
            if !self.world.catalog.goods.contains_key(g) {
                return Err(format!("no good {g}"));
            }
        }
        if let Some(c) = &spec.category {
            if !self.world.catalog.goods.values().any(|g| &g.category == c) {
                return Err(format!("no category {c}"));
            }
        }
        Ok(Effect {
            param: spec.param.clone(),
            target_kind,
            target_id: spec.target_id.clone(),
            op,
            value: spec.value,
            start_day: self.state.day,
            duration: spec.duration,
            decay,
            tau: spec.tau,
            good: spec.good.clone(),
            category: spec.category.clone(),
            source: spec.source.clone(),
        })
    }

    /// All-or-nothing batch: every spec is validated before anything changes, then a single
    /// undo snapshot covers the whole batch. Host-callable (returns `String` errors).
    pub fn add_effect_specs(&mut self, specs: Vec<EffectSpec>) -> Result<(), String> {
        if specs.is_empty() {
            return Err("no effects".to_string());
        }
        let effects = specs
            .iter()
            .map(|s| self.build_effect(s))
            .collect::<Result<Vec<_>, _>>()?;
        self.record();
        self.state.effects.extend(effects);
        Ok(())
    }
}

/// One effect to add, as a plain JSON object (see `add_effect` for the field meanings).
#[derive(Deserialize, Clone)]
pub struct EffectSpec {
    pub target_kind: String,
    pub target_id: String,
    pub param: String,
    pub op: String,
    pub value: f64,
    pub duration: f64,
    pub decay: String,
    #[serde(default = "one")]
    pub tau: f64,
    #[serde(default)]
    pub good: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub source: String,
}

fn one() -> f64 {
    1.0
}

// --- DTOs for JS (plain JSON, decoupled from the core's internal types) --------

#[derive(Serialize)]
struct Snapshot {
    day: i64,
    nodes: Vec<NodeView>,
}

#[derive(Serialize)]
struct NodeView {
    node: String,
    kind: &'static str,
    population: f64,
    prosperity: f64,
    goods: Vec<GoodView>,
}

#[derive(Serialize)]
struct GoodView {
    good: String,
    stock: f64,
    multiplier: f64,
    price: f64,      // base·m (reference price tag)
    buy_price: f64,  // price for a player buying: price·(1+spread)
    sell_price: f64, // price for a player selling: price·(1−spread)
    in_transit: f64,
}

#[derive(Serialize)]
struct WorldInfo {
    seed: u64,
    nodes: Vec<NodeInfo>,
    goods: Vec<GoodInfo>,
    edges: Vec<EdgeInfo>,
}

#[derive(Serialize)]
struct NodeInfo {
    id: String,
    kind: &'static str,
    population: f64,
    spread: f64,
    hub: Option<String>,
}

#[derive(Serialize)]
struct GoodInfo {
    id: String,
    category: String,
    base_price: f64,
}

#[derive(Serialize)]
struct EdgeInfo {
    id: String,
    src: String,
    dst: String,
    travel_time: f64,
}

#[derive(Serialize)]
struct ReportView {
    node: String,
    good: String,
    multiplier: f64,
    provisioning: f64,
    verdict: &'static str, // "shortage" | "surplus" | "normal"
    causes: Vec<CauseView>,
}

/// Structured cause (§5.12, D5): the UI renders and translates it.
#[derive(Serialize)]
struct CauseView {
    kind: &'static str, // "node_effect" | "edge_effect" | "stock"
    magnitude: f64,
    value: f64, // mult: factor; add: addend; stock: provisioning r
    op: Option<&'static str>,
    param: String,
    src: Option<String>,
    dst: Option<String>,
    source: String,
}

#[derive(Serialize)]
struct TradeView {
    good: String,
    qty: f64,
    total: f64,
    avg_price: f64,
    new_multiplier: f64,
}

impl From<purchase::Trade> for TradeView {
    fn from(t: purchase::Trade) -> Self {
        TradeView {
            good: t.good,
            qty: t.qty,
            total: t.total,
            avg_price: t.avg_price,
            new_multiplier: t.new_multiplier,
        }
    }
}

#[derive(Serialize)]
struct EffectView {
    index: usize,
    target_kind: &'static str,
    target_id: String,
    param: String,
    op: &'static str,
    value: f64,
    good: Option<String>,
    category: Option<String>,
    source: String,
    weight: f64,
    factor: String, // current contribution, e.g. "×0.42" / "+3.0"
    days_left: f64,
}

#[derive(Serialize)]
struct FlowView {
    edge: String,
    src: String,
    dst: String,
    in_transit: f64, // total cargo volume in transit on the arc
    accum: f64,      // accumulated flow value
    threshold: f64,  // θ_car
}

#[derive(Serialize)]
struct CaravanView {
    edge: String,
    day_departed: i64,
    day_arrival: i64,
    value: f64,
    guard: f64,
    src: String,
    dst: String,
    cargo: Vec<CargoView>,
}

#[derive(Serialize)]
struct CargoView {
    good: String,
    amount: f64,
}

// --- Helpers -------------------------------------------------------------------

fn kind_str(k: NodeKind) -> &'static str {
    match k {
        NodeKind::City => "city",
        NodeKind::Village => "village",
        NodeKind::Camp => "camp",
    }
}

fn kind_tag(k: TargetKind) -> &'static str {
    match k {
        TargetKind::Node => "node",
        TargetKind::Edge => "edge",
    }
}

fn parse_target_kind(s: &str) -> Result<TargetKind, String> {
    match s {
        "node" => Ok(TargetKind::Node),
        "edge" => Ok(TargetKind::Edge),
        other => Err(format!("unknown target_kind: {other}")),
    }
}

fn parse_op(s: &str) -> Result<Op, String> {
    match s {
        "mult" => Ok(Op::Mult),
        "add" => Ok(Op::Add),
        other => Err(format!("unknown op: {other}")),
    }
}

fn parse_decay(s: &str) -> Result<Decay, String> {
    match s {
        "step" => Ok(Decay::Step),
        "linear" => Ok(Decay::Linear),
        "exp" => Ok(Decay::Exp),
        other => Err(format!("unknown decay: {other}")),
    }
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn err<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn trade_err(e: TradeError) -> JsValue {
    JsValue::from_str(match e {
        TradeError::NonPositiveQty => "qty must be > 0",
        TradeError::BuyExceedsStock => "cannot buy the node's whole stock",
    })
}
