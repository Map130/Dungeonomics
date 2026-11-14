//! Smoke-тесты фасада на хосте (rlib). Методы, отдающие `JsValue` (snapshot/explain/
//! quote), требуют JS-рантайма и здесь не вызываются — их покрывает сборка под wasm32
//! в CI. Проверяем управление состоянием: тик, прогон, откат, сохранение.

use ddeco_wasm::Sim;

const WORLD: &str = include_str!("../../../worlds/example_region.toml");

/// Шаблон фонового шока: сильный неурожай с высокой интенсивностью, чтобы эффект
/// гарантированно срабатывал в тесте.
const SHOCK: &str = r#"
[[shocks]]
id = "blight"
target_kind = "node"
param = "pcap"
op = "mult"
value = 0.1
duration = 40.0
rate = 5.0
category = "food"
source = "неурожай"
"#;

#[test]
fn constructs_and_ticks() {
    let mut sim = Sim::new(WORLD, 0).expect("мир из example_region.toml");
    assert_eq!(sim.day(), 0);
    sim.step();
    assert_eq!(sim.day(), 1);
    sim.run(10);
    assert_eq!(sim.day(), 11);
}

#[test]
fn undo_restores_previous_state() {
    let mut sim = Sim::new(WORLD, 0).expect("мир");
    sim.run(5);
    let saved = sim.save();
    sim.step();
    assert_eq!(sim.day(), 6);

    assert!(sim.undo(), "откат последнего шага");
    assert_eq!(sim.day(), 5);
    assert_eq!(sim.save(), saved, "состояние совпадает со снимком до шага");

    assert!(sim.undo(), "откат прогона run(5)");
    assert_eq!(sim.day(), 0);
    assert!(!sim.undo(), "история исчерпана");
}

#[test]
fn undo_replays_deterministically() {
    // Откат и повтор того же дня дают бит-в-бит то же состояние (§8.4).
    let mut sim = Sim::new(WORLD, 0).expect("мир");
    sim.run(5);
    sim.step();
    let after_first = sim.save();
    assert!(sim.undo());
    sim.step();
    assert_eq!(sim.save(), after_first, "повтор дня воспроизводим");
}

#[test]
fn warmup_advances_day() {
    let sim = Sim::new(WORLD, 50).expect("прогретый мир");
    assert!(sim.day() > 0, "warmup прокручивает дни до сходимости");
}

#[test]
fn add_effect_changes_trajectory_and_undoes() {
    // База без эффектов.
    let mut base = Sim::new(WORLD, 0).expect("мир");
    base.run(20);
    let base_dump = base.save();

    // Сильный неурожай на производство еды в городе меняет траекторию.
    let mut sim = Sim::new(WORLD, 0).expect("мир");
    sim.add_effect(
        "node",
        "redwater",
        "pcap",
        "mult",
        0.05,
        40.0,
        "step",
        1.0,
        None,
        Some("food".into()),
        "неурожай",
    )
    .expect("эффект принят");
    sim.run(20);
    assert_ne!(sim.save(), base_dump, "эффект сдвинул экономику");

    // Откат снимает и эффект, и прогон → возвращаемся к состоянию до add_effect.
    assert!(sim.undo(), "откат прогона");
    assert!(sim.undo(), "откат add_effect");
    assert_eq!(sim.day(), 0);
}

#[test]
fn shocks_toggle_affects_run_and_is_deterministic() {
    let world = format!("{WORLD}{SHOCK}");

    // Выключено (default) — как будто шоков нет.
    let mut off = Sim::new(&world, 0).expect("мир");
    assert!(!off.shocks_enabled());
    off.run(20);

    // Включено — неурожай срабатывает, траектория иная.
    let mut on = Sim::new(&world, 0).expect("мир");
    on.set_shocks_enabled(true);
    on.run(20);
    assert_ne!(on.save(), off.save(), "включённые шоки меняют прогон");

    // Детерминизм: второй включённый прогон совпадает бит-в-бит (§8.4, §5.11).
    let mut on2 = Sim::new(&world, 0).expect("мир");
    on2.set_shocks_enabled(true);
    on2.run(20);
    assert_eq!(on.save(), on2.save(), "шоки воспроизводимы по seed");
}

#[test]
fn save_load_roundtrip() {
    let mut sim = Sim::new(WORLD, 0).expect("мир");
    sim.run(7);
    let dump = sim.save();
    let mut other = Sim::new(WORLD, 0).expect("мир");
    other.load_state(&dump).expect("загрузка снимка");
    assert_eq!(other.day(), 7);
    assert_eq!(other.save(), dump, "состояние после загрузки идентично");
}
