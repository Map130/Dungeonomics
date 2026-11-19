// Reactive wrapper over the WASM `Sim` facade (Svelte 5 runes).
//
// One store per app: it holds the Sim instance and re-reads every derived
// snapshot in `refresh()` after each action. Components read the `$state`
// fields and re-render automatically.

import initWasm, { Sim } from "./pkg/ddeco_wasm.js";
import type {
  CaravanView,
  EffectSpec,
  EffectView,
  FlowView,
  PriceReport,
  Snapshot,
  TradeResult,
  WorldInfo,
} from "./types";

/** Which screen is shown: the world overview or one settlement's page (synced with the URL hash). */
export type View = { kind: "world" } | { kind: "node"; id: string };

function viewFromHash(hash: string): View {
  const m = /^#\/node\/(.+)$/.exec(hash);
  return m ? { kind: "node", id: decodeURIComponent(m[1]) } : { kind: "world" };
}

class AppStore {
  view = $state<View>(typeof location === "undefined" ? { kind: "world" } : viewFromHash(location.hash));

  ready = $state(false);
  /** True while a world is being built / warmed up (the UI shows a hint). */
  warming = $state(false);
  error = $state<string | null>(null);

  // $state.raw: the header renders on `app.sim`, so it must be reactive; raw keeps
  // the WASM object unproxied.
  sim = $state.raw<Sim | null>(null);
  world = $state<WorldInfo | null>(null);
  snapshot = $state<Snapshot | null>(null);
  day = $state(0);
  shocksEnabled = $state(false);
  hasShocks = $state(false);

  effects = $state<EffectView[]>([]);
  flows = $state<FlowView[]>([]);
  caravans = $state<CaravanView[]>([]);

  // Transient results of the GM's latest requests.
  report = $state<PriceReport | null>(null);
  lastTrade = $state<TradeResult | null>(null);

  /** Open a settlement's page (pushes a history entry, so Back returns to the overview). */
  openNode(id: string): void {
    location.hash = `#/node/${encodeURIComponent(id)}`;
  }
  openWorld(): void {
    location.hash = "";
  }

  async init() {
    window.addEventListener("hashchange", () => {
      this.view = viewFromHash(location.hash);
      this.report = null;
    });
    await initWasm();
    this.ready = true;
  }

  /** Load a world from TOML and calibrate / warm it up. */
  async loadWorld(worldToml: string, warmupDays = 0): Promise<void> {
    this.warming = warmupDays > 0;
    // Let the "warming up" hint paint before the synchronous construction.
    if (this.warming) await new Promise((r) => setTimeout(r, 0));
    try {
      const sim = new Sim(worldToml, warmupDays);
      this.sim?.free();
      this.sim = sim;
      this.world = sim.world_info() as WorldInfo;
      // Shocks can be enabled only if the file has [[shocks]]: a rough text
      // heuristic (the facade silently ignores the toggle when there are no templates).
      this.hasShocks = /\[\[\s*shocks\s*\]\]/.test(worldToml);
      this.report = null;
      this.lastTrade = null;
      this.error = null;
      this.refresh();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.warming = false;
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

  /** Like `quote` but silent: failures return undefined and leave `error` alone (list pricing). */
  quoteQuiet(node: string, good: string, qty: number, sell: boolean): TradeResult | undefined {
    if (!this.sim) return undefined;
    try {
      return this.sim.quote(node, good, qty, sell) as TradeResult;
    } catch {
      return undefined;
    }
  }

  quote(node: string, good: string, qty: number, sell: boolean): TradeResult | undefined {
    return this.guard((s) => s.quote(node, good, qty, sell) as TradeResult);
  }
  trade(node: string, good: string, qty: number, sell: boolean): TradeResult | undefined {
    const t = this.guard((s) => s.trade(node, good, qty, sell) as TradeResult);
    if (t) this.lastTrade = t;
    this.refresh();
    return t;
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
  /** Apply a batch of effects atomically (one undo step). */
  addEffects(specs: EffectSpec[]): void {
    this.guard((s) => s.add_effects(specs));
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
