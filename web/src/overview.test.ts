import { describe, expect, it } from "vitest";
import { mostExpensive } from "./overview";

describe("mostExpensive", () => {
  it("empty -> null", () => expect(mostExpensive([])).toBeNull());
  it("all below threshold -> null", () =>
    expect(mostExpensive([{ good: "a", multiplier: 1.49 }, { good: "b", multiplier: 0.8 }])).toBeNull());
  it("exactly the threshold counts", () =>
    expect(mostExpensive([{ good: "a", multiplier: 1.5 }])?.good).toBe("a"));
  it("ties go to the lower id", () =>
    expect(mostExpensive([{ good: "b", multiplier: 2 }, { good: "a", multiplier: 2 }])?.good).toBe("a"));
  it("the max wins", () =>
    expect(mostExpensive([{ good: "a", multiplier: 1.6 }, { good: "b", multiplier: 3 }, { good: "c", multiplier: 2 }])?.good).toBe("b"));
});
