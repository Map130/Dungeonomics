import { describe, expect, it } from "vitest";
import { CATALOG, parseCatalog, type ShopItem } from "./catalog";
import { equivalentUnits, resolveDriver, roleOf } from "./drivers";

const goods = [
  { id: "grain", category: "food" },
  { id: "fish", category: "food" },
  { id: "cloth", category: "cloth" },
  { id: "tools", category: "weapons" },
  { id: "wine", category: "luxury" },
];
const byId = (id: string): ShopItem => {
  const it = CATALOG.find((i) => i.id === id);
  if (!it) throw new Error(id);
  return it;
};

describe("catalog", () => {
  it("parses the real CSV", () => {
    expect(CATALOG).toHaveLength(261);
    const w = byId("wine-fine-bottle");
    expect(w.name).toBe("Wine, fine (bottle)");
    expect(w.priceGp).toBe(10);
  });
  it("skips comments, handles quotes, drops non-positive prices", () => {
    const csv = '# c\nid,name,category,subcategory,price_gp,price_raw,weight_lb,source\na,"B, x",c,s,1.5,r,1,s\nz,Z,c,s,0,r,1,s\n';
    expect(parseCatalog(csv)).toEqual([{ id: "a", name: "B, x", category: "c", subcategory: "s", priceGp: 1.5 }]);
  });
});

describe("roleOf", () => {
  it.each([
    ["longsword", "weapons"],
    ["chain-mail", "weapons"],
    ["wheat-1-lb", "grain"],
    ["rope-hempen-50-feet", "cloth"],
    ["lantern-hooded", "tools"],
    ["rations-1-day", "food"],
    ["wine-common-pitcher", "wine"],
    ["horse-riding", null],
    ["signet-ring", "luxury"],
    ["ball-bearings-bag-of-1000", "tools"],
  ])("%s -> %s", (id, role) => {
    expect(roleOf(byId(id))).toBe(role);
  });
});

describe("roleOf coverage", () => {
  it("every non-mount/ship item has a role", () => {
    const skip = new Set(["Mounts and Other Animals", "Waterborne Vehicles"]);
    const missing = CATALOG.filter((i) => !skip.has(i.subcategory ?? "") && roleOf(i) === null).map((i) => i.id);
    expect(missing).toEqual([]);
  });
});

describe("resolveDriver", () => {
  it("resolves by id, category, stock", () => {
    expect(resolveDriver("weapons", goods, [])).toBe("tools");
    expect(resolveDriver("food", goods, [{ good: "grain", stock: 50 }, { good: "fish", stock: 80 }])).toBe("fish");
    expect(resolveDriver("grain", goods, [])).toBe("grain");
    expect(resolveDriver("luxury", goods, [])).toBe("wine");
    expect(resolveDriver(null, goods, [])).toBeNull();
    expect(resolveDriver("food", [{ id: "cloth", category: "cloth" }], [])).toBeNull();
  });
});

describe("equivalentUnits", () => {
  it("scales by price ratio", () => expect(equivalentUnits(15, 2, 5)).toBe(6));
});
