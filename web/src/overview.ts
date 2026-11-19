/** Multiplier at or above which a good counts as "expensive" in the overview. */
export const EXPENSIVE = 1.5;

/** The most expensive good (highest multiplier, ties by lower id) if it reaches the threshold. */
export function mostExpensive(
  goods: { good: string; multiplier: number }[],
  threshold = EXPENSIVE,
): { good: string; multiplier: number } | null {
  let best: { good: string; multiplier: number } | null = null;
  for (const g of goods) {
    if (g.multiplier < threshold) continue;
    if (!best || g.multiplier > best.multiplier || (g.multiplier === best.multiplier && g.good < best.good)) best = g;
  }
  return best;
}
