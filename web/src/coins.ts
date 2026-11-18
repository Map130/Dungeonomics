// Prices in coins (M6). The core keeps prices as gp floats (§5.5); the UI converts to copper
// (1 gp = 10 sp = 100 cp) and picks a coin by magnitude. Electrum and platinum are not used.

export interface CoinLabels {
  gp: string;
  sp: string;
  cp: string;
}

/** Fixed decimals with trailing zeros trimmed: 1.50 → "1.5", 5.0 → "5". */
function trim(x: number, decimals: number): string {
  return x.toFixed(decimals).replace(/\.?0+$/, "");
}

/** One coin chosen by magnitude: ≥ 100 cp → gp, ≥ 10 cp → sp, otherwise cp (fractions below 1 cp kept). */
export function formatCoin(gp: number, labels: CoinLabels): string {
  const cp = gp * 100;
  if (cp >= 100) return `${trim(gp, 2)} ${labels.gp}`;
  if (cp >= 10) return `${trim(cp / 10, 1)} ${labels.sp}`;
  if (cp >= 1 || cp === 0) return `${trim(cp, 1)} ${labels.cp}`;
  return `${Number(cp.toPrecision(2))} ${labels.cp}`;
}

/** Mixed coins for trade totals, rounded to whole copper: "1 gp 4 sp 7 cp". */
export function formatCoinsMixed(gp: number, labels: CoinLabels): string {
  if (gp > 0 && gp * 100 < 1) return formatCoin(gp, labels); // below a copper: keep the fraction
  const total = Math.round(gp * 100);
  if (total === 0) return `0 ${labels.cp}`;
  const parts: [number, string][] = [
    [Math.floor(total / 100), labels.gp],
    [Math.floor((total % 100) / 10), labels.sp],
    [total % 10, labels.cp],
  ];
  return parts
    .filter(([n]) => n > 0)
    .map(([n, l]) => `${n} ${l}`)
    .join(" ");
}
