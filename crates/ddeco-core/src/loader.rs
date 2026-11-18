//! Loading the world and state from TOML (REQUIREMENTS §5.9). Port of `ddeco/loader.py`.
//!
//! The schema is described in `ddeco/loader.py` and `worlds/example_region.toml`. Unknown
//! good fields are ignored by serde (Python treated them as §7 overrides;
//! the known override fields are parsed explicitly).
//!
//! I/O (reading the file, atomic writes) is left to the caller — the core targets WASM
//! and works with strings: `world_from_toml`/`state_from_toml` take `&str`,
//! `state_to_toml` returns a `String`.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::effects::{Decay, Op, TargetKind};
use crate::goods::{defaults_for, Catalog, GoodParams, PriceClass};
use crate::shocks::ShockTemplate;
use crate::state::SimState;
use crate::world::{Edge, GlobalParams, Node, NodeKind, World};

#[derive(Debug, Deserialize)]
struct RawWorld {
    #[serde(default)]
    seed: u64,
    #[serde(default)]
    params: RawParams,
    #[serde(default)]
    goods: BTreeMap<String, RawGood>,
    #[serde(default)]
    nodes: BTreeMap<String, RawNode>,
    #[serde(default)]
    roads: Vec<RawRoad>,
}

#[derive(Debug, Deserialize)]
struct RawGood {
    category: String,
    base_price: f64,
    per_capita: f64,
    #[serde(default = "default_common")]
    price_class: PriceClass,
    #[serde(default)]
    in_food_basket: bool,
    #[serde(default)]
    basket_weight: f64,
    #[serde(default = "one")]
    volume: f64,
    #[serde(default = "one")]
    weight: f64,
    // §7 overrides (if present).
    alpha: Option<f64>,
    epsilon: Option<f64>,
    beta: Option<f64>,
    m_min: Option<f64>,
    m_max: Option<f64>,
    spoilage: Option<f64>,
    coverage_days: Option<f64>,
    c0: Option<f64>,
    kappa: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawNode {
    #[serde(default = "default_city")]
    kind: NodeKind,
    population: f64,
    #[serde(default = "half")]
    prosperity0: f64,
    #[serde(default = "spread_default")]
    spread: f64,
    hub: Option<String>,
    #[serde(default = "hub_share_default")]
    hub_share: f64,
    #[serde(default)]
    capacity: BTreeMap<String, f64>,
    #[serde(default)]
    garrison: BTreeMap<String, f64>,
}

#[derive(Debug, Deserialize)]
struct RawRoad {
    a: String,
    b: String,
    travel_time: f64,
    #[serde(default = "one")]
    difficulty: f64,
    #[serde(default = "f64_inf")]
    capacity: f64,
}

#[derive(Debug, Default, Deserialize)]
struct RawParams(BTreeMap<String, f64>);

fn default_common() -> PriceClass {
    PriceClass::Common
}
fn default_city() -> NodeKind {
    NodeKind::City
}
fn one() -> f64 {
    1.0
}
fn half() -> f64 {
    0.5
}
fn spread_default() -> f64 {
    0.1
}
fn hub_share_default() -> f64 {
    0.6
}
fn f64_inf() -> f64 {
    f64::INFINITY
}

/// Load a world from a TOML string.
pub fn world_from_toml(text: &str) -> Result<World, toml::de::Error> {
    let raw: RawWorld = toml::from_str(text)?;
    Ok(build_world(raw))
}

fn build_world(raw: RawWorld) -> World {
    let mut goods = BTreeMap::new();
    for (id, g) in raw.goods {
        goods.insert(id.clone(), build_good(id, g));
    }
    let catalog = Catalog { goods };

    let nodes = raw
        .nodes
        .into_iter()
        .map(|(id, n)| {
            (
                id.clone(),
                Node {
                    id,
                    kind: n.kind,
                    population: n.population,
                    prosperity0: n.prosperity0,
                    spread: n.spread,
                    hub: n.hub,
                    hub_share: n.hub_share,
                    capacity: n.capacity,
                    garrison: n.garrison,
                },
            )
        })
        .collect();

    // Each undirected road → two directed arcs a→b and b→a.
    let mut edges = BTreeMap::new();
    for road in raw.roads {
        for (src, dst) in [(&road.a, &road.b), (&road.b, &road.a)] {
            let id = format!("{src}__{dst}");
            edges.insert(
                id.clone(),
                Edge {
                    id,
                    src: src.clone(),
                    dst: dst.clone(),
                    travel_time: road.travel_time,
                    difficulty: road.difficulty,
                    capacity: road.capacity,
                },
            );
        }
    }

    World {
        catalog,
        nodes,
        edges,
        params: build_params(raw.params),
        seed: raw.seed,
    }
}

fn build_good(id: String, g: RawGood) -> GoodParams {
    let d = defaults_for(g.price_class);
    GoodParams {
        id,
        category: g.category,
        base_price: g.base_price,
        alpha: g.alpha.unwrap_or(d.alpha),
        epsilon: g.epsilon.unwrap_or(d.epsilon),
        beta: g.beta.unwrap_or(d.beta),
        m_min: g.m_min.unwrap_or(d.m_min),
        m_max: g.m_max.unwrap_or(d.m_max),
        spoilage: g.spoilage.unwrap_or(d.spoilage),
        coverage_days: g.coverage_days.unwrap_or(d.coverage_days),
        c0: g.c0.unwrap_or(d.c0),
        kappa: g.kappa.unwrap_or(d.kappa),
        per_capita: g.per_capita,
        volume: g.volume,
        weight: g.weight,
        in_food_basket: g.in_food_basket,
        basket_weight: g.basket_weight,
    }
}

fn build_params(raw: RawParams) -> GlobalParams {
    let mut p = GlobalParams::default();
    for (k, v) in raw.0 {
        match k.as_str() {
            "lam" => p.lam = v,
            "kappa_trade" => p.kappa_trade = v,
            "rho_reserve" => p.rho_reserve = v,
            "mu" => p.mu = v,
            "pi_ref" => p.pi_ref = v,
            "c_gamma" => p.c_gamma = v,
            "w_food" => p.w_food = v,
            "w_market" => p.w_market = v,
            "caravan_threshold" => p.caravan_threshold = v,
            "shock_rate" => p.shock_rate = v,
            "lo_prod" => p.lo_prod = v,
            "hi_prod" => p.hi_prod = v,
            _ => {} // unknown parameter is ignored, like in Python
        }
    }
    p
}

// --- Saving/loading the state (§5.9) -------------------------------------------
//
// Effects and materialised caravans are NOT serialised (the GM / scenario log
// restores them, v1). Undo/replay within a session uses in-memory snapshots
// (`SimState::clone`), so the file does not affect the determinism of a run.

#[derive(Debug, Default, Deserialize)]
struct RawState {
    #[serde(default)]
    day: i64,
    #[serde(default)]
    prosperity: BTreeMap<String, f64>,
    #[serde(default)]
    stock: BTreeMap<String, BTreeMap<String, f64>>,
    #[serde(default)]
    multiplier: BTreeMap<String, BTreeMap<String, f64>>,
    #[serde(default)]
    pipeline: BTreeMap<String, Vec<BTreeMap<String, f64>>>,
    #[serde(default)]
    caravan_accum: BTreeMap<String, f64>,
}

/// Parse a state snapshot from TOML.
pub fn state_from_toml(text: &str) -> Result<SimState, toml::de::Error> {
    let raw: RawState = toml::from_str(text)?;
    let mut st = SimState {
        day: raw.day,
        ..Default::default()
    };
    for (node, goods) in raw.stock {
        for (good, v) in goods {
            st.stock.insert((node.clone(), good), v);
        }
    }
    for (node, goods) in raw.multiplier {
        for (good, v) in goods {
            st.multiplier.insert((node.clone(), good), v);
        }
    }
    st.prosperity = raw.prosperity;
    st.caravan_accum = raw.caravan_accum;
    st.pipeline = raw.pipeline;
    Ok(st)
}

/// Compact TOML for the `SimState` schema. Keys are sorted (stable diffs);
/// the format matches the Python emitter (`ddeco/loader.py:state_to_toml`).
pub fn state_to_toml(st: &SimState) -> String {
    let mut lines: Vec<String> = vec![format!("day = {}", st.day), String::new()];

    lines.push("[prosperity]".into());
    for (node, v) in &st.prosperity {
        lines.push(format!("{node} = {}", num(*v)));
    }
    lines.push(String::new());

    for (table, data) in [("stock", &st.stock), ("multiplier", &st.multiplier)] {
        let mut by_node: BTreeMap<&str, BTreeMap<&str, f64>> = BTreeMap::new();
        for ((node, good), v) in data {
            by_node.entry(node).or_default().insert(good, *v);
        }
        for (node, goods) in by_node {
            lines.push(format!("[{table}.{node}]"));
            for (good, v) in goods {
                lines.push(format!("{good} = {}", num(v)));
            }
            lines.push(String::new());
        }
    }

    if st.pipeline.values().any(|slots| !slots.is_empty()) {
        lines.push("[pipeline]".into());
        for (e, slots) in &st.pipeline {
            let inner: Vec<String> = slots
                .iter()
                .map(|slot| {
                    let goods: Vec<String> = slot
                        .iter()
                        .map(|(g, v)| format!("{g} = {}", num(*v)))
                        .collect();
                    format!("{{{}}}", goods.join(", "))
                })
                .collect();
            lines.push(format!("{e} = [{}]", inner.join(", ")));
        }
        lines.push(String::new());
    }

    lines.push("[caravan_accum]".into());
    for (e, v) in &st.caravan_accum {
        lines.push(format!("{e} = {}", num(*v)));
    }
    lines.push(String::new());

    let mut s = lines.join("\n");
    s.push('\n');
    s
}

// --- Background shock templates (§5.11) ---------------------------------------
//
// Optional `[[shocks]]` section in the world file. The generator is off by default
// (an empty list = no shocks), so the section is additive and does not change golden runs.
// The Op/Decay/TargetKind enums do not implement Deserialize — we parse strings and
// map them explicitly, like `build_params`.

#[derive(Debug, Deserialize)]
struct RawShockFile {
    #[serde(default)]
    shocks: Vec<RawShock>,
}

#[derive(Debug, Deserialize)]
struct RawShock {
    id: String,
    target_kind: String,
    param: String,
    op: String,
    value: f64,
    duration: f64,
    rate: f64,
    #[serde(default = "default_decay")]
    decay: String,
    #[serde(default = "one")]
    tau: f64,
    good: Option<String>,
    category: Option<String>,
    #[serde(default)]
    source: String,
}

fn default_decay() -> String {
    "step".into()
}

fn parse_target_kind(s: &str) -> Result<TargetKind, String> {
    match s {
        "node" => Ok(TargetKind::Node),
        "edge" => Ok(TargetKind::Edge),
        other => Err(format!("unknown shock target_kind: {other}")),
    }
}

fn parse_op(s: &str) -> Result<Op, String> {
    match s {
        "mult" => Ok(Op::Mult),
        "add" => Ok(Op::Add),
        other => Err(format!("unknown shock op: {other}")),
    }
}

fn parse_decay(s: &str) -> Result<Decay, String> {
    match s {
        "step" => Ok(Decay::Step),
        "linear" => Ok(Decay::Linear),
        "exp" => Ok(Decay::Exp),
        other => Err(format!("unknown shock decay: {other}")),
    }
}

/// Parse background shock templates from the world TOML. serde ignores other sections;
/// a missing `[[shocks]]` gives an empty list (generator off).
pub fn shocks_from_toml(text: &str) -> Result<Vec<ShockTemplate>, String> {
    let raw: RawShockFile = toml::from_str(text).map_err(|e| e.to_string())?;
    raw.shocks
        .into_iter()
        .map(|s| {
            Ok(ShockTemplate {
                target_kind: parse_target_kind(&s.target_kind)?,
                op: parse_op(&s.op)?,
                decay: parse_decay(&s.decay)?,
                id: s.id,
                param: s.param,
                value: s.value,
                duration: s.duration,
                rate: s.rate,
                tau: s.tau,
                good: s.good,
                category: s.category,
                source: s.source,
            })
        })
        .collect()
}

/// A float as a valid TOML float with full precision (shortest round-trip).
///
/// Rust `{}` for f64 gives the shortest representation that parses back to the same
/// f64 (like `repr` in Python). We ensure a dot/exponent is present — otherwise TOML would
/// read "1" as an integer and deserialising into f64 would fail.
fn num(x: f64) -> String {
    if x == f64::INFINITY {
        return "inf".into();
    }
    if x == f64::NEG_INFINITY {
        return "-inf".into();
    }
    let s = format!("{x}");
    if s.contains(['.', 'e', 'E']) || s.contains("inf") || s.contains("nan") {
        s
    } else {
        format!("{s}.0")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_shocks_section_is_empty() {
        // A world file without [[shocks]] — the generator is off.
        assert!(shocks_from_toml("seed = 1\n").unwrap().is_empty());
    }

    #[test]
    fn parses_shock_template() {
        let toml = r#"
            seed = 1
            [[shocks]]
            id = "blight"
            target_kind = "node"
            param = "pcap"
            op = "mult"
            value = 0.3
            duration = 30.0
            rate = 0.008
            decay = "linear"
            category = "food"
            source = "blight"
        "#;
        let s = shocks_from_toml(toml).unwrap();
        assert_eq!(s.len(), 1);
        let t = &s[0];
        assert_eq!(t.id, "blight");
        assert_eq!(t.target_kind, TargetKind::Node);
        assert_eq!(t.op, Op::Mult);
        assert_eq!(t.decay, Decay::Linear);
        assert_eq!(t.category.as_deref(), Some("food"));
        assert_eq!(t.good, None);
        assert_eq!(t.tau, 1.0, "tau defaults to 1.0");
    }

    #[test]
    fn rejects_unknown_enum() {
        let toml = r#"
            [[shocks]]
            id = "x"
            target_kind = "planet"
            param = "pcap"
            op = "mult"
            value = 1.0
            duration = 1.0
            rate = 0.1
        "#;
        assert!(shocks_from_toml(toml).is_err());
    }
}
