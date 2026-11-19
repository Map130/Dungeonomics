import { describe, expect, it } from "vitest";
import { translateError } from "./errors";

// Echo translator: "key|id" so mappings are visible without the catalogs.
const t = (key: string, p?: Record<string, string | number>) => (p?.id !== undefined ? `${key}|${p.id}` : key);

describe("translateError", () => {
  it("qty", () => expect(translateError("qty must be > 0", t)).toBe("err.qty"));
  it("whole stock", () =>
    expect(translateError("cannot buy the node's whole stock", t)).toBe("err.wholeStock"));
  it("no node", () => expect(translateError("no node stonehaven", t)).toBe("err.noNode|stonehaven"));
  it("no arc", () => expect(translateError("no arc a-b", t)).toBe("err.noArc|a-b"));
  it("no good", () => expect(translateError("no good wine", t)).toBe("err.noGood|wine"));
  it("no category", () => expect(translateError("no category food", t)).toBe("err.noCategory|food"));
  it("Error: prefix from String(e) is tolerated", () =>
    expect(translateError("Error: no node x", t)).toBe("err.noNode|x"));
  it("unknown passes through", () => expect(translateError("boom", t)).toBe("boom"));
  it("empty passes through", () => expect(translateError("", t)).toBe(""));
});
