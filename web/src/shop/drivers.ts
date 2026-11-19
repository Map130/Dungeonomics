import type { ShopItem } from "./catalog";

export type Role = string;

const TRADE: Record<string, Role> = {};
const put = (role: Role, ids: string[]) => ids.forEach((i) => (TRADE[i] = role));
put("grain", ["wheat-1-lb", "flour-1-lb"]);
put("meat", ["chicken", "goat", "sheep", "pig", "cow", "ox"]);
put("salt", ["salt-1-lb"]);
put("iron", ["iron-1-lb"]);
put("materials", ["copper-1-lb"]);
put("silk", ["silk-1-sq-yd"]);
put("spices", ["ginger-1-lb", "cinnamon-1-lb", "pepper-1-lb", "cloves-1-lb", "saffron-1-lb"]);
put("cloth", ["canvas-1-sq-yd", "cotton-cloth-1-sq-yd", "linen-1-sq-yd"]);
put("luxury", ["silver-1-lb", "gold-1-lb", "platinum-1-lb"]);

const CLOTH = ["clothes", "robe", "robes", "cloak", "blanket", "bedroll", "tent", "sack", "backpack", "pouch", "rope", "cloth", "net"];
const FOOD = ["rations", "waterskin"];
const LUXURY = ["perfume", "signet", "ring", "mirror", "spyglass", "book", "spellbook", "ink", "paper", "parchment"];
/** Whole id segments only (split on "-"), so "signet-ring" is not a "net". */
const hasWord = (id: string, words: string[]) => id.split("-").some((w) => words.includes(w));

export function roleOf(item: ShopItem): Role | null {
  const { id, category, subcategory } = item;
  switch (category) {
    case "weapon":
    case "armor":
      return "weapons";
    case "tools":
      return subcategory === "Musical Instrument" || subcategory === "Gaming Sets"
        ? "luxury"
        : "tools";
    case "adventuring-gear":
      switch (subcategory) {
        case "Ammunition":
          return "weapons";
        case "Kits":
          return "tools";
        case "Equipment Packs":
          return "cloth";
        case "Arcane Foci":
        case "Druidic Foci":
        case "Holy Symbols":
          return "luxury";
        case "Standard Gear":
          if (hasWord(id, CLOTH)) return "cloth";
          if (hasWord(id, FOOD)) return "food";
          if (hasWord(id, LUXURY)) return "luxury";
          return "tools";
        default:
          return null;
      }
    case "trade-goods":
      return TRADE[id] ?? null;
    case "food-drink":
      if (id.startsWith("wine")) return "wine";
      return id === "meat-chunk" ? "meat" : "food";
    case "mounts-and-vehicles":
      return subcategory === "Tack, Harness, and Drawn Vehicles" ? "tools" : null;
    default:
      return null;
  }
}

/** A role is either a good id or a good category; picks the good to price against. */
export function resolveDriver(
  role: Role | null,
  goods: { id: string; category: string }[],
  nodeGoods: { good: string; stock: number }[],
): string | null {
  if (role === null) return null;
  if (goods.some((g) => g.id === role)) return role;
  const stock = new Map(nodeGoods.map((n) => [n.good, n.stock]));
  const cands = goods
    .filter((g) => g.category === role)
    .sort((a, b) => (stock.get(b.id) ?? 0) - (stock.get(a.id) ?? 0) || (a.id < b.id ? -1 : 1));
  return cands[0]?.id ?? null;
}

export function equivalentUnits(itemGp: number, qty: number, driverBaseGp: number): number {
  return (itemGp * qty) / driverBaseGp;
}
