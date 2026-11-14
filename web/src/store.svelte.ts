// Реактивная обёртка над WASM-фасадом `Sim` (Svelte 5 runes).
//
// Одно состояние на приложение: держит экземпляр Sim и перечитывает все
// производные снимки в `refresh()` после каждого действия. Компоненты читают
// поля `$state` и перерисовываются автоматически.

import initWasm, { Sim } from "./pkg/ddeco_wasm.js";
import type {
  CaravanView,
  EffectView,
  FlowView,
  PriceReport,
  Snapshot,
  TradeResult,
  WorldInfo,
} from "./types";

class AppStore {
  ready = $state(false);
  error = $state<string | null>(null);

  sim: Sim | null = null;
  world = $state<WorldInfo | null>(null);
  snapshot = $state<Snapshot | null>(null);
  day = $state(0);
  shocksEnabled = $state(false);
  hasShocks = $state(false);

  effects = $state<EffectView[]>([]);
  flows = $state<FlowView[]>([]);
  caravans = $state<CaravanView[]>([]);

  // Транзиентные результаты последних запросов мастера.
  report = $state<PriceReport | null>(null);
  lastTrade = $state<TradeResult | null>(null);

  async init() {
    await initWasm();
    this.ready = true;
  }

  /** Загрузить мир из TOML и откалибровать/прогреть. */
  loadWorld(worldToml: string, warmupDays = 0): void {
    try {
      const sim = new Sim(worldToml, warmupDays);
      this.sim?.free();
      this.sim = sim;
      this.world = sim.world_info() as WorldInfo;
      // Шоки доступны к включению, только если в файле есть [[shocks]]: грубая
      // эвристика по тексту (фасад молча игнорирует тумблер, если шаблонов нет).
      this.hasShocks = /\[\[\s*shocks\s*\]\]/.test(worldToml);
      this.report = null;
      this.lastTrade = null;
      this.error = null;
      this.refresh();
    } catch (e) {
      this.error = String(e);
    }
  }

  private refresh(): void {
    if (!this.sim) return;
    this.snapshot = this.sim.snapshot() as Snapshot;
    this.day = Number(this.sim.day);
    this.shocksEnabled = this.sim.shocks_enabled;
    this.effects = this.sim.effects() as EffectView[];
    this.flows = this.sim.flows() as FlowView[];
    this.caravans = this.sim.caravans() as CaravanView[];
  }

  private guard<T>(fn: (sim: Sim) => T): T | undefined {
    if (!this.sim) return undefined;
    try {
      this.error = null;
      return fn(this.sim);
    } catch (e) {
      this.error = String(e);
      return undefined;
    }
  }

  step(): void {
    this.guard((s) => s.step());
    this.refresh();
  }
  run(days: number): void {
    this.guard((s) => s.run(days));
    this.refresh();
  }
  undo(): void {
    this.guard((s) => s.undo());
    this.refresh();
  }
  setShocks(on: boolean): void {
    this.guard((s) => s.set_shocks_enabled(on));
    this.refresh();
  }

  explain(node: string, good: string): void {
    const r = this.guard((s) => s.explain(node, good) as PriceReport);
    if (r) this.report = r;
  }

  quote(node: string, good: string, qty: number, sell: boolean): TradeResult | undefined {
    return this.guard((s) => s.quote(node, good, qty, sell) as TradeResult);
  }
  trade(node: string, good: string, qty: number, sell: boolean): void {
    const t = this.guard((s) => s.trade(node, good, qty, sell) as TradeResult);
    if (t) this.lastTrade = t;
    this.refresh();
  }

  addEffect(
    targetKind: string,
    targetId: string,
    param: string,
    op: string,
    value: number,
    duration: number,
    decay: string,
    tau: number,
    good: string | null,
    category: string | null,
    source: string,
  ): void {
    this.guard((s) =>
      s.add_effect(targetKind, targetId, param, op, value, duration, decay, tau, good, category, source),
    );
    this.refresh();
  }
  removeEffect(index: number): void {
    this.guard((s) => s.remove_effect(index));
    this.refresh();
  }

  setStock(node: string, good: string, amount: number): void {
    this.guard((s) => s.set_stock(node, good, amount));
    this.refresh();
  }
  addStock(node: string, good: string, delta: number): void {
    this.guard((s) => s.add_stock(node, good, delta));
    this.refresh();
  }
  prosperityHit(node: string, rho: number): void {
    this.guard((s) => s.prosperity_hit(node, rho));
    this.refresh();
  }
  lootCaravan(edge: string, rho: number, slot = -1): void {
    this.guard((s) => s.loot_caravan(edge, rho, slot));
    this.refresh();
  }

  save(): string {
    return this.guard((s) => s.save()) ?? "";
  }
  loadState(toml: string): void {
    this.guard((s) => s.load_state(toml));
    this.refresh();
  }
}

export const app = new AppStore();
