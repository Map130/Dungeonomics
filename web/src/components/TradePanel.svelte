<script lang="ts">
  import { app } from "../store.svelte";
  import { price, t, total } from "../i18n/index.svelte";
  import type { TradeResult } from "../types";

  // Player trade at one settlement for one good (§5.7). Totals in mixed coins.
  let { node, good }: { node: string; good: string } = $props();

  let qty = $state(50);
  let preview = $state<{ r: TradeResult; sell: boolean } | null>(null);

  // A new good or node invalidates the preview.
  $effect(() => {
    void node;
    void good;
    preview = null;
  });

  function quote(sell: boolean) {
    const r = app.quote(node, good, qty, sell);
    preview = r ? { r, sell } : null;
  }
  function execute() {
    if (!preview) return;
    app.trade(node, good, qty, preview.sell);
    preview = null;
  }
</script>

<div class="panel">
  <h3>{t("gm.trade")} · {good}</h3>
  <div class="row">
    <label class="muted">{t("gm.qty")} <input type="number" min="1" bind:value={qty} style="width:5rem;" /></label>
    <button onclick={() => quote(false)}>{t("gm.quoteBuy")}</button>
    <button onclick={() => quote(true)}>{t("gm.quoteSell")}</button>
  </div>
  {#if preview}
    <p style="margin:.5rem 0 .3rem;">
      <strong>{total(preview.r.total)}</strong>
      <span class="muted">· {t("trade.avg", { avg: price(preview.r.avg_price) })} · m→{preview.r.new_multiplier.toFixed(2)}</span>
    </p>
    <button onclick={execute}>{preview.sell ? t("gm.doSell") : t("gm.doBuy")}</button>
  {/if}
  {#if app.lastTrade}
    <p class="muted" style="margin:.4rem 0 0;">
      {t("gm.lastTrade", { good: app.lastTrade.good, qty: app.lastTrade.qty, total: total(app.lastTrade.total) })}
    </p>
  {/if}
</div>
