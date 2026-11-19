// TS mirror of the DTOs in crates/ddeco-wasm/src/lib.rs. Facade methods return `any`
// (serde-wasm-bindgen) — the store casts them to these types.

export interface GoodView {
  good: string;
  stock: number;
  multiplier: number;
  price: number; // base·m (price tag)
  buy_price: number; // price·(1+spread)
  sell_price: number; // price·(1−spread)
  in_transit: number;
  /** r = s/s* (Infinity when there is no demand). */
  provisioning: number;
  verdict: Verdict;
}

export interface NodeView {
  node: string;
  kind: string;
  population: number;
  prosperity: number;
  goods: GoodView[];
}

export interface Snapshot {
  day: number;
  nodes: NodeView[];
}

export interface NodeInfo {
  id: string;
  kind: string;
  population: number;
  spread: number;
  hub: string | null;
  /** Goods the node can produce (base capacity > 0), sorted. */
  produces: string[];
}

export interface GoodInfo {
  id: string;
  category: string;
  base_price: number;
  /** Display names by language; empty when the world file has none. */
  names: Record<string, string>;
}

export interface EdgeInfo {
  id: string;
  src: string;
  dst: string;
  travel_time: number;
}

export interface WorldInfo {
  seed: number;
  nodes: NodeInfo[];
  goods: GoodInfo[];
  edges: EdgeInfo[];
}

/** Structured cause from the core (§5.12, D5); the UI renders it to text. */
export interface Cause {
  kind: "node_effect" | "edge_effect" | "stock";
  magnitude: number;
  /** mult: current factor; add: current addend; stock: provisioning r */
  value: number;
  op: "mult" | "add" | null;
  param: string;
  src: string | null;
  dst: string | null;
  source: string;
}

export type Verdict = "shortage" | "surplus" | "normal";

export interface PriceReport {
  node: string;
  good: string;
  multiplier: number;
  provisioning: number;
  verdict: Verdict;
  causes: Cause[];
}

export interface TradeResult {
  good: string;
  qty: number;
  total: number;
  avg_price: number;
  new_multiplier: number;
}

/** One effect to add (input to `Sim.add_effects`, e.g. from an event preset). */
export interface EffectSpec {
  target_kind: "node" | "edge";
  target_id: string;
  param: string;
  op: "mult" | "add";
  value: number;
  duration: number;
  decay: "step" | "linear" | "exp";
  tau?: number;
  good?: string | null;
  category?: string | null;
  source: string;
}

export interface EffectView {
  index: number;
  target_kind: string;
  target_id: string;
  param: string;
  op: string;
  value: number;
  good: string | null;
  category: string | null;
  source: string;
  weight: number;
  factor: string;
  days_left: number;
}

export interface FlowView {
  edge: string;
  src: string;
  dst: string;
  in_transit: number;
  accum: number;
  threshold: number;
}

export interface CargoView {
  good: string;
  amount: number;
}

export interface CaravanView {
  edge: string;
  day_departed: number;
  day_arrival: number;
  value: number;
  guard: number;
  src: string;
  dst: string;
  cargo: CargoView[];
}
