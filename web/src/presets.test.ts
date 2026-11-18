import { describe, expect, it } from "vitest";
import { en } from "./i18n/en";
import { ru } from "./i18n/ru";
import { PRESETS, expandPreset, expandPresetOnTargets, roadsFromEdges, type PresetId } from "./presets";

const get = (id: PresetId) => PRESETS.find((p) => p.id === id)!;

describe("presets", () => {
  it("every preset has EN and RU names", () => {
    for (const p of PRESETS) {
      const key = `preset.${p.id}` as keyof typeof en;
      expect(en[key]).toBeTruthy();
      expect(ru[key]).toBeTruthy();
    }
  });

  it("craftsman_killed expands with the chosen good", () => {
    expect(
      expandPreset(get("craftsman_killed"), "redwater", { good: "grain", source: "x" }),
    ).toEqual([
      {
        target_kind: "node",
        target_id: "redwater",
        param: "pcap",
        op: "mult",
        value: 0.2,
        duration: 30,
        decay: "linear",
        tau: 1,
        good: "grain",
        category: null,
        source: "x",
      },
    ]);
  });

  it("craftsman_killed without a good throws", () => {
    expect(() => expandPreset(get("craftsman_killed"), "redwater", { source: "x" })).toThrow(
      /needs a good/,
    );
  });

  it("duration override applies to every effect", () => {
    const specs = expandPreset(get("epidemic"), "n", { duration: 7, source: "x" });
    expect(specs).toHaveLength(2);
    expect(specs.every((s) => s.duration === 7)).toBe(true);
  });

  it("bridge_collapsed targets the edge with two effects", () => {
    const specs = expandPreset(get("bridge_collapsed"), "e1", { source: "x" });
    expect(specs.every((s) => s.target_kind === "edge")).toBe(true);
    expect(specs.map((s) => s.param)).toEqual(["capacity", "difficulty"]);
  });

  it("roadsFromEdges dedupes opposite arcs into one road", () => {
    const roads = roadsFromEdges([
      { id: "redwater__eastford", src: "redwater", dst: "eastford" },
      { id: "eastford__redwater", src: "eastford", dst: "redwater" },
      { id: "a__b", src: "a", dst: "b" },
    ]);
    expect(roads).toEqual([
      { key: "a__b", label: "a ↔ b", arcs: ["a__b"] },
      {
        key: "eastford__redwater",
        label: "eastford ↔ redwater",
        arcs: ["eastford__redwater", "redwater__eastford"],
      },
    ]);
  });

  it("bandits on a road expands to both arcs", () => {
    const specs = expandPresetOnTargets(get("bandits"), ["eastford__redwater", "redwater__eastford"], {
      source: "x",
    });
    expect(specs).toHaveLength(2);
    expect(specs.map((s) => s.target_id)).toEqual(["eastford__redwater", "redwater__eastford"]);
    expect(specs.every((s) => s.target_kind === "edge" && s.param === "difficulty")).toBe(true);
  });
});
