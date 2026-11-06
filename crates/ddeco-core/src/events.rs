//! Разовые события (MODEL.md §3, §5.9). Порт `ddeco/events.py`.
//!
//! В отличие от параметрических эффектов, разовые события меняют *состояние* один раз
//! и в параметры не входят: правка запасов мастером, удар по процветанию, грабёж
//! каравана. Все они детерминированы и не зависят от RNG.

use crate::state::SimState;

/// Мастер вручную задаёт запас (игрок скупил/подвёз товар). REQUIREMENTS §5.8.
pub fn set_stock(state: &mut SimState, node: &str, good: &str, amount: f64) {
    state
        .stock
        .insert((node.to_string(), good.to_string()), amount.max(0.0));
}

/// Сдвинуть запас на delta (может быть отрицательным), не ниже нуля.
pub fn add_stock(state: &mut SimState, node: &str, good: &str, delta: f64) {
    let key = (node.to_string(), good.to_string());
    let cur = state.stock.get(&key).copied().unwrap_or(0.0);
    state.stock.insert(key, (cur + delta).max(0.0));
}

/// Удар по процветанию: π ← π·(1 − ρ) (набег, §5.8).
pub fn prosperity_hit(state: &mut SimState, node: &str, rho: f64) {
    if let Some(p) = state.prosperity.get_mut(node) {
        *p *= 1.0 - rho;
    }
}

/// Ограбление: убрать долю ρ груза из конвейера дуги (§5.9).
///
/// `slot_index = None` — весь груз в пути по дуге; иначе конкретная партия.
/// Возвращает суммарный объём награбленного (мастер решает, передать ли игрокам).
pub fn loot_caravan(state: &mut SimState, edge: &str, rho: f64, slot_index: Option<usize>) -> f64 {
    let slots = match state.pipeline.get_mut(edge) {
        Some(s) if !s.is_empty() => s,
        _ => return 0.0,
    };
    let targets: Vec<usize> = match slot_index {
        Some(i) => {
            if i >= slots.len() {
                return 0.0;
            }
            vec![i]
        }
        None => (0..slots.len()).collect(),
    };
    let mut looted = 0.0;
    for i in targets {
        let goods: Vec<String> = slots[i].keys().cloned().collect();
        for g in goods {
            let take = slots[i][&g] * rho;
            *slots[i].get_mut(&g).unwrap() -= take;
            looted += take;
        }
    }
    looted
}
