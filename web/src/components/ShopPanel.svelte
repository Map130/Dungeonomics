<script lang="ts">
  import { app } from "../store.svelte";
  import { price, t, total } from "../i18n/index.svelte";
  import type { MessageKey } from "../i18n/en";
  import type { TradeResult } from "../types";
  import { CATALOG, type ShopItem } from "../shop/catalog";
  import { equivalentUnits, resolveDriver, roleOf } from "../shop/drivers";

  // Settlement shop: SRD items priced off the driver good's multiplier at this node;
  // a trade moves the driver good by the value-equivalent number of units.
  let { node }: { node: string } = $props();

  const CAP = 50;
  const categories = [...new Set(CATALOG.map((i) => i.category))].sort();

  let query = $state("");
  let category = $state("");
  let selected = $state<ShopItem | null>(null);
  let qty = $state(1);
  let preview = $state<{ r: TradeResult; sell: boolean; units: number } | null>(null);
  let done = $state("");

  // A new node invalidates selection, preview and the last-trade line.
  $effect(() => {
    void node;
    selected = null;
    preview = null;
    done = "";
  });
  // Changing the item or quantity invalidates the quote.
  $effect(() => {
    void selected;
    void qty;
    preview = null;
  });

  const spread = $derived(app.world?.nodes.find((n) => n.id === node)?.spread ?? 0);
  const nodeGoods = $derived(app.snapshot?.nodes.find((n) => n.node === node)?.goods ?? []);
  const meanM = $derived(nodeGoods.length ? nodeGoods.reduce((s, g) => s + g.multiplier, 0) / nodeGoods.length : 1);

  type Priced = { item: ShopItem; driver: string | null; m: number; buy: number; sell: number };
  function pricedOf(item: ShopItem): Priced {
    const driver = resolveDriver(roleOf(item), app.world?.goods ?? [], nodeGoods);
    const m = (driver && nodeGoods.find((g) => g.good === driver)?.multiplier) || meanM;
    return { item, driver, m, buy: item.priceGp * m * (1 + spread), sell: item.priceGp * m * (1 - spread) };
  }

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return CATALOG.filter((i) => (!category || i.category === category) && (!q || i.name.toLowerCase().includes(q)));
  });
  const rows = $derived(filtered.slice(0, CAP).map(pricedOf));
  const current = $derived(selected ? pricedOf(selected) : null);
  const driverBase = $derived(current?.driver ? (app.world?.goods.find((g) => g.id === current.driver)?.base_price ?? 0) : 0);
  const canTrade = $derived(!!current?.driver && driverBase > 0 && qty >= 1);

  const driverLabel = (p: Priced) =>
    p.driver ? `← ${p.driver} ×${p.m.toFixed(2)}` : `← ${p.m.toFixed(2)} (${t("shop.noDriver").split(" — ")[0]})`;

  function quote(sell: boolean) {
    if (!current?.driver || !canTrade) return;
    const units = equivalentUnits(current.item.priceGp, qty, driverBase);
    const r = app.quote(node, current.driver, units, sell);
    preview = r ? { r, sell, units } : null;
  }
  function execute() {
    if (!preview || !current?.driver) return;
    const { r, sell, units } = preview;
    const text = t("shop.done", {
      verb: t(sell ? "shop.sold" : "shop.bought"),
      qty,
      item: current.item.name,
      total: total(r.total),
    });
    app.trade(node, current.driver, units, sell);
    preview = null;
    done = text;
  }
</script>

<div class="panel">
  <h2>{t("shop.title")}</h2>
  <div class="row">
    <input type="search" placeholder={t("shop.search")} bind:value={query} />
    <select bind:value={category}>
      <option value="">{t("shop.all")}</option>
      {#each categories as c}
        <option value={c}>{t(`shopcat.${c}` as MessageKey)}</option>
      {/each}
    </select>
  </div>

  {#if rows.length === 0}
    <p class="muted">{t("shop.none")}</p>
  {:else}
    <table>
      <thead><tr><th></th><th>{t("node.buy")}</th><th>{t("node.sell")}</th><th></th></tr></thead>
      <tbody>
        {#each rows as p (p.item.id)}
          <tr class="clickable" class:selected={p.item.id === selected?.id} onclick={() => (selected = p.item)}>
            <td>{p.item.name}</td>
            <td>{price(p.buy)}</td>
            <td>{price(p.sell)}</td>
            <td class="muted">{driverLabel(p)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if filtered.length > CAP}
      <p class="muted" style="margin:.4rem 0 0;">{t("shop.showing", { n: CAP, total: filtered.length })}</p>
    {/if}
  {/if}

  {#if current}
    <h3 style="margin-top:.8rem;">
      {current.item.name}
      {#if current.driver}<span class="muted">· {t("shop.via", { good: current.driver })}</span>{/if}
    </h3>
    <div class="row">
      <label class="muted">{t("gm.qty")} <input type="number" min="1" bind:value={qty} style="width:5rem;" /></label>
      <button disabled={!canTrade} onclick={() => quote(false)}>{t("gm.quoteBuy")}</button>
      <button disabled={!canTrade} onclick={() => quote(true)}>{t("gm.quoteSell")}</button>
    </div>
    {#if !current.driver}
      <p class="muted" style="margin:.4rem 0 0;">{t("shop.noDriver")}</p>
    {/if}
    {#if preview}
      <p style="margin:.5rem 0 .3rem;">
        <strong>{total(preview.r.total)}</strong>
        <span class="muted">· {t("trade.avg", { avg: price(preview.r.total / qty) })} · m→{preview.r.new_multiplier.toFixed(2)}</span>
      </p>
      <button onclick={execute}>{preview.sell ? t("gm.doSell") : t("gm.doBuy")}</button>
    {/if}
  {/if}
  {#if done}<p class="muted" style="margin:.4rem 0 0;">{done}</p>{/if}
</div>

<style>
  tr.selected td {
    background: var(--panel-2);
  }
</style>
