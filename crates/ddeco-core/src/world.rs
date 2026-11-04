//! Описание мира: узлы, дороги, граф поставок (REQUIREMENTS §5.1, MODEL.md §2).
//!
//! Порт `ddeco/world.py`. Мир — это *параметры*; состояние живёт отдельно в [`crate::state`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::goods::Catalog;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    City,
    Village,
    Camp,
}

/// Узел графа. Деревня привязывается к городу (`hub`) — иерархия §5.1.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub population: f64,  // N_i
    pub prosperity0: f64, // стартовое π_i
    pub spread: f64,      // σ
    pub hub: Option<String>,
    pub hub_share: f64,                  // η_v
    pub capacity: BTreeMap<String, f64>, // good -> Pcap_ig
    pub garrison: BTreeMap<String, f64>, // good -> Garrison_ig
}

impl Node {
    pub fn pcap(&self, good: &str) -> f64 {
        self.capacity.get(good).copied().unwrap_or(0.0)
    }

    pub fn garrison_of(&self, good: &str) -> f64 {
        self.garrison.get(good).copied().unwrap_or(0.0)
    }
}

/// Направленная дуга i→j. Дорога между узлами даёт две дуги (§2).
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub id: String,
    pub src: String,
    pub dst: String,
    pub travel_time: f64, // L_e
    pub difficulty: f64,  // k_e
    pub capacity: f64,    // cap_e
}

impl Edge {
    /// τ_e = max(1, ceil(L_e · k_e)) — задержка конвейера (§5.6).
    pub fn delay_days(&self) -> usize {
        let raw = (self.travel_time * self.difficulty).ceil() as i64;
        raw.max(1) as usize
    }
}

/// Глобальные настройки симуляции (§7). `Default` повторяет Python-значения.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalParams {
    pub lam: f64,
    pub kappa_trade: f64,
    pub rho_reserve: f64,
    pub mu: f64,
    pub pi_ref: f64,
    pub c_gamma: f64,
    pub w_food: f64,
    pub w_market: f64,
    pub caravan_threshold: f64,
    pub shock_rate: f64,
    pub lo_prod: f64,
    pub hi_prod: f64,
}

impl Default for GlobalParams {
    fn default() -> Self {
        GlobalParams {
            lam: 0.3,
            kappa_trade: 0.3,
            rho_reserve: 0.5,
            mu: 0.03,
            pi_ref: 0.5,
            c_gamma: 0.03,
            w_food: 0.6,
            w_market: 0.4,
            caravan_threshold: 1000.0,
            shock_rate: 1.0 / 120.0,
            lo_prod: 0.25,
            hi_prod: 2.0,
        }
    }
}

/// Полное описание региона. Обход узлов/дуг — в отсортированном порядке (§9).
#[derive(Debug, Clone)]
pub struct World {
    pub catalog: Catalog,
    pub nodes: BTreeMap<String, Node>,
    pub edges: BTreeMap<String, Edge>,
    pub params: GlobalParams,
    pub seed: u64,
}

impl World {
    pub fn node_ids(&self) -> impl Iterator<Item = &String> {
        self.nodes.keys()
    }

    pub fn edge_ids(&self) -> impl Iterator<Item = &String> {
        self.edges.keys()
    }

    pub fn outgoing<'a>(&'a self, node: &'a str) -> impl Iterator<Item = &'a String> {
        self.edges
            .iter()
            .filter(move |(_, e)| e.src == node)
            .map(|(id, _)| id)
    }

    pub fn incoming<'a>(&'a self, node: &'a str) -> impl Iterator<Item = &'a String> {
        self.edges
            .iter()
            .filter(move |(_, e)| e.dst == node)
            .map(|(id, _)| id)
    }
}
