//! WASM-биндинги ядра «живой экономики» (`ddeco-core`) для веб-UI.
//!
//! Тонкий фасад: ядро остаётся UI-независимым эталоном (порт Python-прототипа),
//! а этот крейт лишь оборачивает его публичный API в [`Sim`] и переводит состояние
//! в чистый JSON для JS. Вся логика симуляции — в ядре; здесь только склейка.
//!
//! Инварианты сохраняются as-is: тик идёт через [`ddeco_core::tick::step_day`],
//! откат — по in-memory снимкам `SimState` (детерминизм §8.4), I/O (файлы, часы)
//! остаётся на стороне JS — фасад работает со строками TOML, как и ядро.

use ddeco_core::calibration::{seed_state, warmup};
use ddeco_core::effects::{Decay, Effect, Op, TargetKind};
use ddeco_core::loader::{shocks_from_toml, state_from_toml, state_to_toml, world_from_toml};
use ddeco_core::purchase::{self, TradeError};
use ddeco_core::shocks::ShockTemplate;
use ddeco_core::state::SimState;
use ddeco_core::world::{NodeKind, World};
use ddeco_core::{events, reasons, tick};

use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Глубина стека снимков для отката (`undo`). Вечер игры — тысячи тиков, но назад
/// мастеру нужны единицы; ограничиваем память верхним пределом.
const HISTORY_CAP: usize = 1024;

/// Фасад симуляции для UI: владеет миром, состоянием и историей снимков.
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
    /// Создать симуляцию из TOML-описания мира (схема — `worlds/example_region.toml`).
    ///
    /// `warmup_days > 0` — прогреть мир до равновесия без событий (§5.14), иначе взять
    /// откалиброванный стартовый снимок без прогона.
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

    /// Текущий день симуляции.
    #[wasm_bindgen(getter)]
    pub fn day(&self) -> i64 {
        self.state.day
    }

    /// Продвинуть симуляцию на один день (§4). Фоновые шоки — по тумблеру (default off).
    pub fn step(&mut self) {
        self.record();
        // Прямое обращение к полям (disjoint borrows): шаблоны и state не пересекаются.
        let shocks: &[ShockTemplate] = if self.shocks_enabled {
            &self.shocks
        } else {
            &[]
        };
        tick::step_day(&self.world, &mut self.state, shocks);
    }

    /// Прокрутить `days` дней подряд. Для отката сохраняется один снимок на весь прогон.
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

    /// Включить/выключить генератор фоновых шоков (§5.11). По умолчанию выключен.
    /// Если в файле мира нет секции `[[shocks]]`, тумблер ни на что не влияет.
    pub fn set_shocks_enabled(&mut self, on: bool) {
        self.shocks_enabled = on;
    }

    /// Включён ли генератор фоновых шоков.
    #[wasm_bindgen(getter)]
    pub fn shocks_enabled(&self) -> bool {
        self.shocks_enabled
    }

    /// Откатить последнее изменение состояния (тик/прогон/событие). `true` — если было
    /// что откатывать. Повтор того же дня после отката воспроизводится бит-в-бит (§8.4).
    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some(prev) => {
                self.state = prev;
                true
            }
            None => false,
        }
    }

    // --- Снимки для UI --------------------------------------------------------

    /// Статика мира: узлы, товары, дороги (для построения интерфейса). Не меняется.
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

    /// Текущее состояние: день, процветание узлов и по каждому товару запас/множитель/цена.
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

    /// Объяснить текущую цену товара в узле: обеспеченность, вердикт, цепочка причин (§5.12).
    pub fn explain(&self, node: &str, good: &str) -> Result<JsValue, JsValue> {
        let r = reasons::explain_price(&self.world, &self.state, node, good);
        to_js(&ReportView {
            node: r.node,
            good: r.good,
            multiplier: r.multiplier,
            provisioning: r.provisioning,
            verdict: r.verdict,
            causes: r
                .causes
                .into_iter()
                .map(|c| CauseView {
                    magnitude: c.magnitude,
                    text: c.text,
                })
                .collect(),
        })
    }

    // --- Параметрические эффекты (§3, §5.8) -----------------------------------

    /// Добавить параметрический эффект из UI (неурожай на `pcap`, дорога ×2.5 и т.п.).
    ///
    /// `target_kind` — `"node"`/`"edge"`, `op` — `"mult"`/`"add"`, `decay` —
    /// `"step"`/`"linear"`/`"exp"`. `start_day` берётся текущим. `good`/`category` —
    /// необязательные фильтры товара. Поддерживает `undo`.
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
        let target_kind = parse_target_kind(target_kind)?;
        let op = parse_op(op)?;
        let decay = parse_decay(decay)?;
        // Цель должна существовать, иначе эффект молча ни на что не влияет.
        match target_kind {
            TargetKind::Node if !self.world.nodes.contains_key(target_id) => {
                return Err(JsValue::from_str(&format!("нет узла {target_id}")));
            }
            TargetKind::Edge if !self.world.edges.contains_key(target_id) => {
                return Err(JsValue::from_str(&format!("нет дуги {target_id}")));
            }
            _ => {}
        }
        if let Some(g) = &good {
            if !self.world.catalog.goods.contains_key(g) {
                return Err(JsValue::from_str(&format!("нет товара {g}")));
            }
        }
        self.record();
        self.state.effects.push(Effect {
            param: param.to_string(),
            target_kind,
            target_id: target_id.to_string(),
            op,
            value,
            start_day: self.state.day,
            duration,
            decay,
            tau,
            good,
            category,
            source: source.to_string(),
        });
        Ok(())
    }

    /// Лента активных эффектов (ненулевой вес на текущий день) с источником, текущим
    /// фактором и остатком длительности — для панели событий UI (§5.8).
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

    /// Снять эффект по индексу из `effects()`. `true` — если индекс существовал.
    /// Поддерживает `undo`.
    pub fn remove_effect(&mut self, index: usize) -> bool {
        if index >= self.state.effects.len() {
            return false;
        }
        self.record();
        self.state.effects.remove(index);
        true
    }

    // --- Потоки и караваны (§5.6, §5.10) --------------------------------------

    /// Потоки по дугам: груз в пути и прогресс к порогу материализации θ_car (§5.8).
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

    /// Материализованные именные караваны (лента для UI, §5.10).
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
                source: c.source.clone(),
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

    // --- Сделки игрока (§5.7) -------------------------------------------------

    /// Посчитать сделку, НЕ меняя состояние (предпросмотр цены). `sell` — продажа игроком.
    pub fn quote(&self, node: &str, good: &str, qty: f64, sell: bool) -> Result<JsValue, JsValue> {
        let t = purchase::quote(&self.world, &self.state, node, good, qty, sell, 256)
            .map_err(trade_err)?;
        to_js(&TradeView::from(t))
    }

    /// Провести сделку: обновить запас и множитель, вернуть её детали. Поддерживает `undo`.
    pub fn trade(
        &mut self,
        node: &str,
        good: &str,
        qty: f64,
        sell: bool,
    ) -> Result<JsValue, JsValue> {
        // Валидируем до записи снимка, чтобы ошибочная сделка не засоряла историю.
        purchase::quote(&self.world, &self.state, node, good, qty, sell, 256).map_err(trade_err)?;
        self.record();
        let t = purchase::apply(&self.world, &mut self.state, node, good, qty, sell, 256)
            .map_err(trade_err)?;
        to_js(&TradeView::from(t))
    }

    // --- Разовые события мастера (§5.8–5.9) -----------------------------------

    /// Мастер задаёт запас товара в узле (скупили/подвезли).
    pub fn set_stock(&mut self, node: &str, good: &str, amount: f64) {
        self.record();
        events::set_stock(&mut self.state, node, good, amount);
    }

    /// Сдвинуть запас на `delta` (может быть отрицательным), не ниже нуля.
    pub fn add_stock(&mut self, node: &str, good: &str, delta: f64) {
        self.record();
        events::add_stock(&mut self.state, node, good, delta);
    }

    /// Удар по процветанию узла: π ← π·(1 − rho) (набег, §5.8).
    pub fn prosperity_hit(&mut self, node: &str, rho: f64) {
        self.record();
        events::prosperity_hit(&mut self.state, node, rho);
    }

    /// Ограбление каравана на дуге: убрать долю `rho` груза. `slot = -1` — весь груз в
    /// пути, иначе конкретная партия. Возвращает суммарный объём награбленного.
    pub fn loot_caravan(&mut self, edge: &str, rho: f64, slot: i32) -> f64 {
        self.record();
        let idx = if slot < 0 { None } else { Some(slot as usize) };
        events::loot_caravan(&mut self.state, edge, rho, idx)
    }

    // --- Сохранение/загрузка состояния (§5.9) ---------------------------------

    /// Сериализовать текущее состояние в TOML (мастер сохраняет файлом). Запись — на JS.
    pub fn save(&self) -> String {
        state_to_toml(&self.state)
    }

    /// Загрузить состояние из TOML, заменив текущее. История отката очищается.
    pub fn load_state(&mut self, state_toml: &str) -> Result<(), JsValue> {
        self.state = state_from_toml(state_toml).map_err(err)?;
        self.history.clear();
        Ok(())
    }

    // --- Внутреннее -----------------------------------------------------------

    /// Записать снимок перед изменением состояния (для `undo`), соблюдая потолок истории.
    fn record(&mut self) {
        if self.history.len() == HISTORY_CAP {
            self.history.remove(0);
        }
        self.history.push(self.state.clone());
    }
}

// --- DTO для JS (чистый JSON, развязан с внутренними типами ядра) -------------

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
    price: f64,      // base·m (справочная цена-ценник)
    buy_price: f64,  // цена покупки игроком: price·(1+spread)
    sell_price: f64, // цена продажи игроком: price·(1−spread)
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
    verdict: String,
    causes: Vec<CauseView>,
}

#[derive(Serialize)]
struct CauseView {
    magnitude: f64,
    text: String,
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
    factor: String, // текущий вклад, напр. "×0.42" / "+3.0"
    days_left: f64,
}

#[derive(Serialize)]
struct FlowView {
    edge: String,
    src: String,
    dst: String,
    in_transit: f64, // суммарный объём груза в пути по дуге
    accum: f64,      // накопленная стоимость потока
    threshold: f64,  // θ_car
}

#[derive(Serialize)]
struct CaravanView {
    edge: String,
    day_departed: i64,
    day_arrival: i64,
    value: f64,
    guard: f64,
    source: String,
    cargo: Vec<CargoView>,
}

#[derive(Serialize)]
struct CargoView {
    good: String,
    amount: f64,
}

// --- Хелперы ------------------------------------------------------------------

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

fn parse_target_kind(s: &str) -> Result<TargetKind, JsValue> {
    match s {
        "node" => Ok(TargetKind::Node),
        "edge" => Ok(TargetKind::Edge),
        other => Err(JsValue::from_str(&format!(
            "неизвестный target_kind: {other}"
        ))),
    }
}

fn parse_op(s: &str) -> Result<Op, JsValue> {
    match s {
        "mult" => Ok(Op::Mult),
        "add" => Ok(Op::Add),
        other => Err(JsValue::from_str(&format!("неизвестный op: {other}"))),
    }
}

fn parse_decay(s: &str) -> Result<Decay, JsValue> {
    match s {
        "step" => Ok(Decay::Step),
        "linear" => Ok(Decay::Linear),
        "exp" => Ok(Decay::Exp),
        other => Err(JsValue::from_str(&format!("неизвестный decay: {other}"))),
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
        TradeError::NonPositiveQty => "qty должно быть > 0",
        TradeError::BuyExceedsStock => "нельзя купить весь запас узла",
    })
}
