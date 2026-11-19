import { describe, expect, it } from "vitest";
import { CATALOG, parseCatalog, type ShopItem } from "./catalog";
import { equivalentUnits, resolveDriver, roleOf } from "./drivers";

const goods = [
  { id: "grain", category: "food", total_capacity: 30 },
  { id: "fish", category: "food", total_capacity: 50 },
  { id: "meat", category: "food", total_capacity: 10 },
  { id: "salt", category: "food", total_capacity: 5 },
  { id: "iron", category: "materials", total_capacity: 8 },
  { id: "cloth", category: "cloth", total_capacity: 4 },
  { id: "tools", category: "tools", total_capacity: 4 },
  { id: "weapons", category: "weapons", total_capacity: 3 },
  { id: "wine", category: "luxury", total_capacity: 9 },
  { id: "spices", category: "luxury", total_capacity: 9 },
  { id: "silk", category: "luxury", total_capacity: 2 },
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
    ["robes", "cloth"],
    ["spellbook", "luxury"],
    ["ball-bearings-bag-of-1000", "tools"],
    ["iron-1-lb", "iron"],
    ["copper-1-lb", "materials"],
    ["salt-1-lb", "salt"],
    ["silk-1-sq-yd", "silk"],
    ["pepper-1-lb", "spices"],
    ["saffron-1-lb", "spices"],
    ["cow", "meat"],
    ["chicken", "meat"],
    ["meat-chunk", "meat"],
    ["bread-loaf", "food"],
    ["cheese-hunk", "food"],
    ["arrow", "weapons"],
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
  it("resolves by id, else the category's largest total capacity", () => {
    expect(resolveDriver("weapons", goods)).toBe("weapons");
    expect(resolveDriver("materials", goods)).toBe("iron");
    expect(resolveDriver("meat", goods)).toBe("meat");
    expect(resolveDriver("grain", goods)).toBe("grain");
    expect(resolveDriver("food", goods)).toBe("fish");
    expect(resolveDriver(null, goods)).toBeNull();
    expect(resolveDriver("food", [{ id: "cloth", category: "cloth", total_capacity: 1 }])).toBeNull();
  });
  it("ties go to the lower id (wine/spices -> spices)", () => {
    expect(resolveDriver("luxury", goods)).toBe("spices");
  });
  it("all-zero capacity falls back to the lowest id", () => {
    const z = [
      { id: "b", category: "x", total_capacity: 0 },
      { id: "a", category: "x", total_capacity: 0 },
    ];
    expect(resolveDriver("x", z)).toBe("a");
  });
});

describe("equivalentUnits", () => {
  it("scales by price ratio", () => expect(equivalentUnits(15, 2, 5)).toBe(6));
});
