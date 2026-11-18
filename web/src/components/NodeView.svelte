<script lang="ts">
  import { app } from "../store.svelte";
  import { labelOf, price, t } from "../i18n/index.svelte";
  import type { GoodView, Verdict } from "../types";
  import ReasonsPanel from "./ReasonsPanel.svelte";
  import TradePanel from "./TradePanel.svelte";
  import PresetsPanel from "./PresetsPanel.svelte";
  import EffectsPanel from "./EffectsPanel.svelte";
  import ShopPanel from "./ShopPanel.svelte";

  // One settlement's page: goods by category, reasons and trade for the selected good,
  // settlement actions (presets, stock, prosperity).
  let { id }: { id: string } = $props();

  const info = $derived(app.world?.nodes.find((n) => n.id === id) ?? null);
  const view = $derived(app.snapshot?.nodes.find((n) => n.node === id) ?? null);

  // Goods grouped by category, in the world's catalogue order.
  const groups = $derived.by(() => {
    const byGood = new Map((view?.goods ?? []).map((g) => [g.good, g]));
    const out = new Map<string, GoodView[]>();
    for (const g of app.world?.goods ?? []) {
      const gv = byGood.get(g.id);
      if (!gv) continue;
      if (!out.has(g.category)) out.set(g.category, []);
      out.get(g.category)!.push(gv);
    }
    return [...out.entries()];
  });

  let selected = $state("");
  let amount = $state(100);
  let rho = $state(0.2);

  // Leaving the node or loading another world clears the selection.
  $effect(() => {
    void id;
    selected = "";
  });

  // Keep the reasons log in sync with the selected good after every refresh (day, trade, undo).
  $effect(() => {
    void app.snapshot;
    if (selected) app.explain(id, selected);
  });

  function select(good: string) {
    selected = good;
  }

  function tagClass(v: Verdict): string {
    return v === "shortage" ? "bad" : v === "surplus" ? "good" : "norm";
  }
  const stockPct = (g: GoodView) => (Number.isFinite(g.provisioning) ? `${(g.provisioning * 100).toFixed(0)}%` : "—");
</script>

{#if info && view}
  <main style="padding: 1rem; display:grid; gap:1rem; grid-template-columns: 3fr 2fr; align-items:start;">
    <div style="display:grid; gap:1rem;">
      <div class="panel">
        <div class="row" style="justify-content: space-between;">
          <button onclick={() => app.openWorld()}>{t("nav.back")}</button>
          {#if info.hub}<span class="muted">{t("node.hub", { hub: info.hub })}</span>{/if}
        </div>
        <h2 style="margin-top:.6rem;">{id}</h2>
        <p class="muted" style="margin:0;">
          {t("node.header", { kind: labelOf("kind", info.kind), pop: info.population.toFixed(0), pi: view.prosperity.toFixed(2) })}
        </p>
      </div>

      <div class="panel">
        <h2>{t("node.goods")} <span class="muted">· {t("node.hint")}</span></h2>
        <table>
          <thead>
            <tr><th></th><th>{t("node.buy")}</th><th>{t("node.sell")}</th><th>{t("node.stock")}</th><th></th></tr>
          </thead>
          <tbody>
            {#each groups as [category, goods]}
              <tr><td colspan="5" class="muted" style="padding-top:.6rem;"><strong>{labelOf("category", category)}</strong></td></tr>
              {#each goods as g}
                <tr class="clickable" class:selected={g.good === selected} onclick={() => select(g.good)}>
                  <td>{g.good}</td>
                  <td>{price(g.buy_price)}</td>
                  <td>{price(g.sell_price)}</td>
                  <td>{stockPct(g)}</td>
                  <td><span class={"tag " + tagClass(g.verdict)}>×{g.multiplier.toFixed(2)}</span></td>
                </tr>
              {/each}
            {/each}
          </tbody>
        </table>
      </div>

      <ShopPanel node={id} />

      <EffectsPanel node={id} showForm={false} />
    </div>

    <div style="display:grid; gap:1rem;">
      {#if selected}
        <ReasonsPanel />
        <TradePanel node={id} good={selected} />
      {:else}
        <div class="panel"><p class="muted" style="margin:0;">{t("node.hint")}</p></div>
      {/if}

      <div class="panel">
        <h2>{t("node.actions")}</h2>
        <PresetsPanel mode="node" node={id} />
        {#if selected}
          <h3 style="margin-top:.6rem;">{t("gm.stockTitle")} · {selected}</h3>
          <div class="row">
            <label class="muted">{t("gm.qty")} <input type="number" bind:value={amount} style="width:5rem;" /></label>
            <button onclick={() => app.setStock(id, selected, amount)}>{t("gm.setStock")}</button>
            <button onclick={() => app.addStock(id, selected, amount)}>{t("gm.addStock")}</button>
            <button onclick={() => app.addStock(id, selected, -amount)}>{t("gm.removeStock")}</button>
          </div>
        {/if}
        <div class="row" style="margin-top:.6rem;">
          <label class="muted">ρ <input type="number" step="0.05" bind:value={rho} style="width:4rem;" /></label>
          <button onclick={() => app.prosperityHit(id, rho)}>{t("gm.prosperityHit")}</button>
        </div>
      </div>
    </div>
  </main>
{:else}
  <main style="padding:1rem;">
    <div class="panel">
      <button onclick={() => app.openWorld()}>{t("nav.back")}</button>
      <p class="muted">{t("prices.noWorld")}</p>
    </div>
  </main>
{/if}

<style>
  tr.selected td {
    background: var(--panel-2);
  }
</style>
