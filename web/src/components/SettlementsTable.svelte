<script lang="ts">
  import { app } from "../store.svelte";
  import { goodName, labelOf, t } from "../i18n/index.svelte";
  import type { NodeView } from "../types";

  // Static info (produces, hub) comes from world_info; dynamic (π, goods) from the snapshot.
  const info = $derived(new Map((app.world?.nodes ?? []).map((n) => [n.id, n])));

  /** The worst shortage at a node (lowest provisioning among "shortage" goods), if any. */
  function worstShortage(n: NodeView): string | null {
    const short = n.goods.filter((g) => g.verdict === "shortage");
    if (!short.length) return null;
    return short.reduce((a, b) => (b.provisioning < a.provisioning ? b : a)).good;
  }
</script>

<div class="panel">
  <h2>{t("settlements.title")} <span class="muted">· {t("settlements.hint")}</span></h2>
  {#if app.snapshot}
    <table>
      <thead>
        <tr>
          <th></th>
          <th>{t("settlements.kind")}</th>
          <th>{t("settlements.population")}</th>
          <th>π</th>
          <th>{t("settlements.produces")}</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each app.snapshot.nodes as n}
          {@const short = worstShortage(n)}
          <tr class="clickable" onclick={() => app.openNode(n.node)}>
            <td><strong>{n.node}</strong></td>
            <td class="muted">{labelOf("kind", n.kind)}</td>
            <td>{n.population.toFixed(0)}</td>
            <td>{n.prosperity.toFixed(2)}</td>
            <td>{info.get(n.node)?.produces.map(goodName).join(", ") || "—"}</td>
            <td>{#if short}<span class="tag bad">⚠ {t("settlements.alert")}: {goodName(short)}</span>{/if}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="muted">{t("prices.noWorld")}</p>
  {/if}
</div>
