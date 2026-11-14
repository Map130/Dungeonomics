<script lang="ts">
  import { app } from "../store.svelte";
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
  <h2>Действия мастера</h2>

  <div class="row">
    <select bind:value={node}>{#each nodes as n}<option>{n}</option>{/each}</select>
    <select bind:value={good}>{#each goods as g}<option>{g}</option>{/each}</select>
  </div>

  <h3 style="margin-top:.6rem;">Сделка игрока</h3>
  <div class="row">
    <label class="muted">кол-во <input type="number" bind:value={qty} style="width:5rem;" /></label>
    <button onclick={() => doQuote(false)}>котировка покупки</button>
    <button onclick={() => doQuote(true)}>котировка продажи</button>
  </div>
  {#if preview}
    <p class="muted" style="margin:.4rem 0;">
      итого {preview.total.toFixed(2)} · средняя {preview.avg_price.toFixed(3)} · m→{preview.new_multiplier.toFixed(2)}
    </p>
    <div class="row">
      <button onclick={() => { app.trade(node, good, qty, false); preview = null; }}>провести покупку</button>
      <button onclick={() => { app.trade(node, good, qty, true); preview = null; }}>провести продажу</button>
    </div>
  {/if}
  {#if app.lastTrade}
    <p class="muted" style="margin:.3rem 0;">
      последняя сделка: {app.lastTrade.good} ×{app.lastTrade.qty} = {app.lastTrade.total.toFixed(2)}
    </p>
  {/if}

  <h3 style="margin-top:.6rem;">Запас и процветание</h3>
  <div class="row">
    <label class="muted">кол-во <input type="number" bind:value={amount} style="width:5rem;" /></label>
    <button onclick={() => app.setStock(node, good, amount)}>задать запас</button>
    <button onclick={() => app.addStock(node, good, amount)}>+ к запасу</button>
    <button onclick={() => app.addStock(node, good, -amount)}>− из запаса</button>
  </div>
  <div class="row" style="margin-top:.4rem;">
    <label class="muted">ρ <input type="number" step="0.05" bind:value={rho} style="width:4rem;" /></label>
    <button onclick={() => app.prosperityHit(node, rho)}>удар по процветанию</button>
  </div>
</div>
