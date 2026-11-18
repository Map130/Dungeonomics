// Minimal i18n (D5): two catalogs, a reactive current language and `t(key, params)`.
// No dependency on purpose — the UI is small and must work offline.

import type { Cause, Verdict } from "../types";
import { formatCoin, formatCoinsMixed, type CoinLabels } from "../coins";
import { en, type MessageKey, type Messages } from "./en";
import { ru } from "./ru";

export type Lang = "en" | "ru";
export const LANGS: { id: Lang; label: string }[] = [
  { id: "en", label: "English" },
  { id: "ru", label: "Русский" },
];

const CATALOGS: Record<Lang, Messages> = { en, ru };
const STORAGE_KEY = "dungeonomics.lang";

function detect(): Lang {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "en" || saved === "ru") return saved;
  } catch {
    // storage may be unavailable (private mode, blocked site data)
  }
  const nav = typeof navigator !== "undefined" ? navigator.language : "";
  return nav.toLowerCase().startsWith("ru") ? "ru" : "en";
}

class I18n {
  lang = $state<Lang>(detect());

  constructor() {
    this.apply();
  }

  set(lang: Lang): void {
    this.lang = lang;
    try {
      localStorage.setItem(STORAGE_KEY, lang);
    } catch {
      // not persisted — still switched for this session
    }
    this.apply();
  }

  private apply(): void {
    if (typeof document === "undefined") return;
    document.documentElement.lang = this.lang;
    document.title = this.t("app.title");
  }

  /** Translate a key, substituting `{name}` parameters. Falls back to EN, then the key. */
  t(key: MessageKey, params: Record<string, string | number> = {}): string {
    const template = CATALOGS[this.lang][key] ?? en[key] ?? key;
    return template.replace(/\{(\w+)\}/g, (m, name: string) =>
      name in params ? String(params[name]) : m,
    );
  }
}

export const i18n = new I18n();
export const t = (key: MessageKey, params?: Record<string, string | number>): string =>
  i18n.t(key, params);

/** Human-readable effect parameter (unknown params are shown as-is). */
export function paramLabel(param: string): string {
  const key = `param.${param}` as MessageKey;
  return key in en ? t(key) : param;
}

export function verdictLabel(v: Verdict): string {
  return t(`verdict.${v}`);
}

/** Render a structured cause from the core (§5.12) in the current language. */
export function causeText(c: Cause): string {
  if (c.kind === "stock") return t("cause.stock", { pct: (c.value * 100).toFixed(0) });
  const label =
    c.kind === "edge_effect"
      ? `${t("cause.road", { src: c.src ?? "", dst: c.dst ?? "" })} · ${paramLabel(c.param)}`
      : paramLabel(c.param);
  const factor = c.op === "mult" ? `×${c.value.toFixed(2)}` : `+${c.value.toFixed(1)}`;
  return `${label} ${factor}${c.source ? ` (${c.source})` : ""}`;
}

/** Coin labels in the current language (gp/sp/cp or зм/см/мм). */
export function coins(): CoinLabels {
  return { gp: t("coin.gp"), sp: t("coin.sp"), cp: t("coin.cp") };
}

/** A price in one coin, current language. */
export const price = (gp: number): string => formatCoin(gp, coins());

/** A trade total in mixed coins, current language. */
export const total = (gp: number): string => formatCoinsMixed(gp, coins());
