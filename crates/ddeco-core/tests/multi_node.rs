//! R5: сверка многоузлового слоя (торговля §5.6, караваны §5.10, покупки §5.7,
//! разовые события §5.9) с golden-прогонами Python-эталона.
//!
//! Числа сверяются с относительным допуском 1e-9 (разные libm, §9). Загрузка мира —
//! тем же TOML-загрузчиком, старт — калибровочным `seed_state`.

use std::collections::BTreeMap;

use ddeco_core::calibration::seed_state;
use ddeco_core::effects::{Decay, Op, TargetKind};
use ddeco_core::events;
use ddeco_core::loader::world_from_toml;
use ddeco_core::purchase;
use ddeco_core::shocks::ShockTemplate;
use ddeco_core::state::SimState;
use ddeco_core::tick::step_day;
use ddeco_core::world::World;
use serde::Deserialize;

const TOL: f64 = 1e-9;

fn golden(name: &str) -> String {
    std::fs::read_to_string(format!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/{}.json"),
        name
    ))
    .unwrap_or_else(|_| panic!("golden {name}"))
}

fn example_world() -> World {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../worlds/example_region.toml"
    ))
    .expect("example_region.toml");
    world_from_toml(&text).expect("parse")
}

fn assert_close(label: &str, got: f64, want: f64) {
    let scale = want.abs().max(1.0);
    assert!(
        (got - want).abs() <= TOL * scale,
        "{label}: rust={got} python={want} (|Δ|={:.3e})",
        (got - want).abs()
    );
}

#[derive(Deserialize)]
struct Frame {
    day: i64,
    stock: BTreeMap<String, f64>,
    multiplier: BTreeMap<String, f64>,
    prosperity: BTreeMap<String, f64>,
}

fn key(k: &str) -> (String, String) {
    let (n, g) = k.split_once('|').unwrap();
    (n.to_string(), g.to_string())
}

fn check_frame(state: &SimState, frame: &Frame, label: &str) {
    for (k, &want) in &frame.stock {
        assert_close(&format!("{label} stock {k}"), state.stock[&key(k)], want);
    }
    for (k, &want) in &frame.multiplier {
        assert_close(
            &format!("{label} mult {k}"),
            state.multiplier[&key(k)],
            want,
        );
    }
    for (k, &want) in &frame.prosperity {
        assert_close(&format!("{label} pros {k}"), state.prosperity[k], want);
    }
}

/// Прогнать `days` дней, сверяя кадры по их полю `day` (как в golden.rs).
fn run_frames(
    world: &World,
    state: &mut SimState,
    days: u32,
    frames: &[Frame],
    sh: &[ShockTemplate],
) {
    let mut it = frames.iter();
    let mut next = it.next();
    if let Some(fr) = next {
        if fr.day == 0 {
            check_frame(state, fr, "day 0");
            next = it.next();
        }
    }
    for _ in 0..days {
        step_day(world, state, sh);
        if let Some(fr) = next {
            if fr.day == state.day {
                check_frame(state, fr, &format!("day {}", fr.day));
                next = it.next();
            }
        }
    }
    assert!(next.is_none(), "не все кадры сверены");
}

// --- example_run: пример региона, 400 дней, без шоков -------------------------

#[derive(Deserialize)]
struct CaravanJson {
    edge: String,
    day_departed: i64,
    day_arrival: i64,
    value: f64,
    guard: f64,
    cargo: Vec<(String, f64)>,
}

#[derive(Deserialize)]
struct ExampleRun {
    days: u32,
    frames: Vec<Frame>,
    caravans: Vec<CaravanJson>,
}

#[test]
fn example_run() {
    let sc: ExampleRun = serde_json::from_str(&golden("example_run")).unwrap();
    let world = example_world();
    let mut state = seed_state(&world);
    run_frames(&world, &mut state, sc.days, &sc.frames, &[]);

    assert_eq!(
        state.caravans.len(),
        sc.caravans.len(),
        "число караванов: rust={} python={}",
        state.caravans.len(),
        sc.caravans.len()
    );
    for (i, (got, want)) in state.caravans.iter().zip(&sc.caravans).enumerate() {
        assert_eq!(got.edge, want.edge, "караван {i}: дуга");
        assert_eq!(
            got.day_departed, want.day_departed,
            "караван {i}: отправление"
        );
        assert_eq!(got.day_arrival, want.day_arrival, "караван {i}: прибытие");
        assert_close(&format!("караван {i} value"), got.value, want.value);
        assert_close(&format!("караван {i} guard"), got.guard, want.guard);
        assert_eq!(
            got.cargo.len(),
            want.cargo.len(),
            "караван {i}: номенклатура груза"
        );
        for (gc, wc) in got.cargo.iter().zip(&want.cargo) {
            assert_eq!(gc.0, wc.0, "караван {i}: товар");
            assert_close(&format!("караван {i} cargo {}", gc.0), gc.1, wc.1);
        }
    }
}

// --- shocks_run: тот же мир с фоновыми шоками, 200 дней -----------------------

#[derive(Deserialize)]
struct Fired {
    source: String,
    target: String,
    start_day: i64,
}

#[derive(Deserialize)]
struct ShocksRun {
    days: u32,
    frames: Vec<Frame>,
    fired: Vec<Fired>,
}

fn test_shocks() -> Vec<ShockTemplate> {
    vec![
        ShockTemplate {
            id: "blight".into(),
            target_kind: TargetKind::Node,
            param: "pcap".into(),
            op: Op::Mult,
            value: 0.3,
            duration: 15.0,
            rate: 1.0 / 20.0,
            decay: Decay::Linear,
            tau: 1.0,
            good: None,
            category: Some("food".into()),
            source: "неурожай".into(),
        },
        ShockTemplate {
            id: "bandits".into(),
            target_kind: TargetKind::Edge,
            param: "difficulty".into(),
            op: Op::Mult,
            value: 2.5,
            duration: 10.0,
            rate: 1.0 / 15.0,
            decay: Decay::Linear,
            tau: 1.0,
            good: None,
            category: None,
            source: "бандиты".into(),
        },
    ]
}

#[test]
fn shocks_run() {
    let sc: ShocksRun = serde_json::from_str(&golden("shocks_run")).unwrap();
    let world = example_world();
    let mut state = seed_state(&world);
    run_frames(&world, &mut state, sc.days, &sc.frames, &test_shocks());

    // Сработавшие эффекты по (источник, цель, день старта) — проверка RNG-логики.
    let mut fired: Vec<(String, String, i64)> = state
        .effects
        .iter()
        .map(|e| (e.source.clone(), e.target_id.clone(), e.start_day))
        .collect();
    fired.sort();
    fired.dedup();
    assert_eq!(fired.len(), sc.fired.len(), "число сработавших эффектов");
    for (got, want) in fired.iter().zip(&sc.fired) {
        assert_eq!(
            (got.0.as_str(), got.1.as_str(), got.2),
            (want.source.as_str(), want.target.as_str(), want.start_day),
            "сработавший эффект"
        );
    }
}

// --- events: покупка, грабёж, удар по процветанию ------------------------------

#[derive(Deserialize)]
struct TradeJson {
    total: f64,
    avg_price: f64,
    new_multiplier: f64,
}

#[derive(Deserialize)]
struct Step {
    action: String,
    #[serde(default)]
    trade: Option<TradeJson>,
    #[serde(default)]
    edge: Option<String>,
    #[serde(default)]
    looted: Option<f64>,
    state: Frame,
}

#[derive(Deserialize)]
struct Events {
    steps: Vec<Step>,
}

#[test]
fn events() {
    let sc: Events = serde_json::from_str(&golden("events")).unwrap();
    let world = example_world();
    let mut state = seed_state(&world);

    // Шаг 1: 20 дней торговли без шоков.
    let run20 = &sc.steps[0];
    assert_eq!(run20.action, "run20");
    for _ in 0..20 {
        step_day(&world, &mut state, &[]);
    }
    check_frame(&state, &run20.state, "run20");

    // Шаг 2: крупная покупка зерна в redwater.
    let buy = &sc.steps[1];
    assert_eq!(buy.action, "buy");
    let t = purchase::apply(&world, &mut state, "redwater", "grain", 500.0, false, 256)
        .expect("покупка проходит");
    let tj = buy.trade.as_ref().unwrap();
    assert_close("buy total", t.total, tj.total);
    assert_close("buy avg_price", t.avg_price, tj.avg_price);
    assert_close("buy new_multiplier", t.new_multiplier, tj.new_multiplier);
    check_frame(&state, &buy.state, "buy");

    // Шаг 3: грабёж первого каравана в пути (первая дуга с непустым грузом).
    let loot = &sc.steps[2];
    assert_eq!(loot.action, "loot");
    let edge = world
        .edge_ids()
        .find(|e| {
            state
                .pipeline
                .get(*e)
                .is_some_and(|slots| slots.iter().any(|s| !s.is_empty()))
        })
        .cloned()
        .expect("есть дуга с грузом в пути");
    assert_eq!(&edge, loot.edge.as_ref().unwrap(), "дуга грабежа");
    let looted = events::loot_caravan(&mut state, &edge, 0.5, None);
    assert_close("looted", looted, loot.looted.unwrap());
    check_frame(&state, &loot.state, "loot");

    // Шаг 4: удар по процветанию redwater.
    let hit = &sc.steps[3];
    assert_eq!(hit.action, "prosperity_hit");
    events::prosperity_hit(&mut state, "redwater", 0.25);
    check_frame(&state, &hit.state, "prosperity_hit");
}
