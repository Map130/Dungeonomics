/** Map the facade's known English error strings to catalog keys; unknown text passes through. */
type T = (key: any, params?: Record<string, string | number>) => string;

export function translateError(msg: string, t: T): string {
  const m = msg.replace(/^Error:\s*/, "");
  if (m === "qty must be > 0") return t("err.qty");
  if (m === "cannot buy the node's whole stock") return t("err.wholeStock");
  const id = (prefix: string) => (m.startsWith(prefix) ? m.slice(prefix.length) : null);
  let v: string | null;
  if ((v = id("no node ")) !== null) return t("err.noNode", { id: v });
  if ((v = id("no arc ")) !== null) return t("err.noArc", { id: v });
  if ((v = id("no good ")) !== null) return t("err.noGood", { id: v });
  if ((v = id("no category ")) !== null) return t("err.noCategory", { id: v });
  return msg;
}
