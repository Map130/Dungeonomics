import { describe, expect, it } from "vitest";
import { formatCoin, formatCoinsMixed, type CoinLabels } from "./coins";

const L: CoinLabels = { gp: "gp", sp: "sp", cp: "cp" };

describe("formatCoin (one coin by magnitude)", () => {
  it.each([
    [0.01, "1 cp"],
    [0.007, "0.7 cp"],
    [0.0004, "0.04 cp"],
    [0.05, "5 cp"],
    [0.5, "5 sp"],
    [0.47, "4.7 sp"],
    [1.5, "1.5 gp"],
    [1.234, "1.23 gp"],
    [10, "10 gp"],
    [0, "0 cp"],
  ])("%s gp → %s", (gp, want) => {
    expect(formatCoin(gp, L)).toBe(want);
  });

  it("uses the given labels", () => {
    expect(formatCoin(0.5, { gp: "зм", sp: "см", cp: "мм" })).toBe("5 см");
  });
});

describe("formatCoinsMixed (trade totals)", () => {
  it.each([
    [1.47, "1 gp 4 sp 7 cp"],
    [2, "2 gp"],
    [0.3, "3 sp"],
    [12.05, "12 gp 5 cp"],
    [0.007, "0.7 cp"],
    [0, "0 cp"],
  ])("%s gp → %s", (gp, want) => {
    expect(formatCoinsMixed(gp, L)).toBe(want);
  });

  it("rounds to whole copper", () => {
    expect(formatCoinsMixed(0.4699999, L)).toBe("4 sp 7 cp");
  });
});
