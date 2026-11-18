import raw from "../../../references/srd_gear.csv?raw";

export interface ShopItem {
  id: string;
  name: string;
  category: string;
  subcategory: string;
  priceGp: number;
}

function splitLine(line: string): string[] {
  const out: string[] = [];
  let cur = "";
  let quoted = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (quoted) {
      if (c === '"' && line[i + 1] === '"') {
        cur += '"';
        i++;
      } else if (c === '"') quoted = false;
      else cur += c;
    } else if (c === '"') quoted = true;
    else if (c === ",") {
      out.push(cur);
      cur = "";
    } else cur += c;
  }
  out.push(cur);
  return out;
}

export function parseCatalog(csv: string): ShopItem[] {
  const lines = csv
    .split(/\r?\n/)
    .filter((l) => l.trim() !== "" && !l.startsWith("#"));
  const items: ShopItem[] = [];
  for (const line of lines.slice(1)) {
    const [id, name, category, subcategory, price] = splitLine(line);
    const priceGp = Number(price);
    if (!(priceGp > 0)) continue;
    items.push({ id, name, category, subcategory, priceGp });
  }
  return items;
}

export const CATALOG: ShopItem[] = parseCatalog(raw).sort((a, b) =>
  a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
);
