// Quick event presets (M5): a named bundle of parametric effects (MODEL §3).
import type { EffectSpec } from "./types";
import { t } from "./i18n/index.svelte";

export type PresetId =
  | "craftsman_killed"
  | "blight"
  | "army_billeted"
  | "epidemic"
  | "bandits"
  | "bridge_collapsed";

export interface PresetEffect {
  param: string;
  op: "mult" | "add";
  value: number;
  decay: "step" | "linear" | "exp";
  category?: string;
  perGood?: boolean;
}

export interface Preset {
  id: PresetId;
  target: "node" | "edge";
  needsGood: boolean;
  duration: number;
  effects: PresetEffect[];
}

export const PRESETS: Preset[] = [
  {
    id: "craftsman_killed",
    target: "node",
    needsGood: true,
    duration: 30,
    effects: [{ param: "pcap", op: "mult", value: 0.2, decay: "linear", perGood: true }],
  },
  {
    id: "blight",
    target: "node",
    needsGood: false,
    duration: 60,
    effects: [{ param: "pcap", op: "mult", value: 0.4, decay: "linear", category: "food" }],
  },
  {
    id: "army_billeted",
    target: "node",
    needsGood: false,
    duration: 20,
    effects: [{ param: "population", op: "mult", value: 1.3, decay: "step" }],
  },
  {
    id: "epidemic",
    target: "node",
    needsGood: false,
    duration: 45,
    effects: [
      { param: "population", op: "mult", value: 0.8, decay: "linear" },
      { param: "pcap", op: "mult", value: 0.8, decay: "linear" },
    ],
  },
  {
    id: "bandits",
    target: "edge",
    needsGood: false,
    duration: 14,
    effects: [{ param: "difficulty", op: "mult", value: 2.5, decay: "linear" }],
  },
  {
    id: "bridge_collapsed",
    target: "edge",
    needsGood: false,
    duration: 30,
    effects: [
      { param: "capacity", op: "mult", value: 0.1, decay: "step" },
      { param: "difficulty", op: "mult", value: 3, decay: "step" },
    ],
  },
];

export function presetName(id: PresetId): string {
  return t(`preset.${id}`);
}

export function expandPreset(
  p: Preset,
  targetId: string,
  opts: { good?: string; duration?: number; source: string },
): EffectSpec[] {
  if (p.needsGood && !opts.good) throw new Error("preset needs a good");
  return p.effects.map((e) => ({
    target_kind: p.target,
    target_id: targetId,
    param: e.param,
    op: e.op,
    value: e.value,
    duration: opts.duration ?? p.duration,
    decay: e.decay,
    tau: 1,
    good: e.perGood ? (opts.good ?? null) : null,
    category: e.category ?? null,
    source: opts.source,
  }));
}

/** Expand a preset onto several targets (e.g. both arcs of a road) as one batch. */
export function expandPresetOnTargets(
  p: Preset,
  targetIds: string[],
  opts: { good?: string; duration?: number; source: string },
): EffectSpec[] {
  return targetIds.flatMap((id) => expandPreset(p, id, opts));
}

/** Group directed arcs into roads: one entry per unordered pair of endpoints. */
export function roadsFromEdges(
  edges: { id: string; src: string; dst: string }[],
): { key: string; label: string; arcs: string[] }[] {
  const roads = new Map<string, { key: string; label: string; arcs: string[] }>();
  for (const e of edges) {
    const [a, b] = [e.src, e.dst].sort();
    const key = `${a}__${b}`;
    let r = roads.get(key);
    if (!r) {
      r = { key, label: `${a} ↔ ${b}`, arcs: [] };
      roads.set(key, r);
    }
    r.arcs.push(e.id);
  }
  const out = [...roads.values()];
  for (const r of out) r.arcs.sort();
  return out.sort((x, y) => (x.key < y.key ? -1 : x.key > y.key ? 1 : 0));
}
