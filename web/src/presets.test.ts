import { describe, expect, it } from "vitest";
import { en } from "./i18n/en";
import { ru } from "./i18n/ru";
import { PRESETS, expandPreset, type PresetId } from "./presets";

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
});
