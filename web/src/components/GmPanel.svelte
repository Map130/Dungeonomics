<script lang="ts">
  import { app } from "../store.svelte";
  import { t } from "../i18n/index.svelte";
  import type { TradeResult } from "../types";

  const nodes = $derived(app.world?.nodes.map((n) => n.id) ?? []);
  const goods = $derived(app.world?.goods.map((g) => g.id) ?? []);

  let node = $state("");
  let good = $state("");
  let qty = $state(50);
  let amount = $state(100);
  let rho = $state(0.2);
  let preview = $state<TradeResult | null>(null);

  $effect(() => {
    if (nodes.length && !nodes.includes(node)) node = nodes[0];
    if (goods.length && !goods.includes(good)) good = goods[0];
  });

  function doQuote(sell: boolean) {
    preview = app.quote(node, good, qty, sell) ?? null;
  }
</script>

<div class="panel">
  <h2>{t("gm.title")}</h2>

  <div class="row">
    <select bind:value={node}>{#each nodes as n}<option>{n}</option>{/each}</select>
    <select bind:value={good}>{#each goods as g}<option>{g}</option>{/each}</select>
  </div>

  <h3 style="margin-top:.6rem;">{t("gm.trade")}</h3>
  <div class="row">
    <label class="muted">{t("gm.qty")} <input type="number" bind:value={qty} style="width:5rem;" /></label>
    <button onclick={() => doQuote(false)}>{t("gm.quoteBuy")}</button>
    <button onclick={() => doQuote(true)}>{t("gm.quoteSell")}</button>
  </div>
  {#if preview}
    <p class="muted" style="margin:.4rem 0;">
      {t("gm.quote", {
        total: preview.total.toFixed(2),
        avg: preview.avg_price.toFixed(3),
        m: preview.new_multiplier.toFixed(2),
      })}
    </p>
    <div class="row">
      <button onclick={() => { app.trade(node, good, qty, false); preview = null; }}>{t("gm.doBuy")}</button>
      <button onclick={() => { app.trade(node, good, qty, true); preview = null; }}>{t("gm.doSell")}</button>
    </div>
  {/if}
  {#if app.lastTrade}
    <p class="muted" style="margin:.3rem 0;">
      {t("gm.lastTrade", {
        good: app.lastTrade.good,
        qty: app.lastTrade.qty,
        total: app.lastTrade.total.toFixed(2),
      })}
    </p>
  {/if}

  <h3 style="margin-top:.6rem;">{t("gm.stockTitle")}</h3>
  <div class="row">
    <label class="muted">{t("gm.qty")} <input type="number" bind:value={amount} style="width:5rem;" /></label>
    <button onclick={() => app.setStock(node, good, amount)}>{t("gm.setStock")}</button>
    <button onclick={() => app.addStock(node, good, amount)}>{t("gm.addStock")}</button>
    <button onclick={() => app.addStock(node, good, -amount)}>{t("gm.removeStock")}</button>
  </div>
  <div class="row" style="margin-top:.4rem;">
    <label class="muted">ρ <input type="number" step="0.05" bind:value={rho} style="width:4rem;" /></label>
    <button onclick={() => app.prosperityHit(node, rho)}>{t("gm.prosperityHit")}</button>
  </div>
</div>
