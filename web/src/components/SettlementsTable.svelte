<script lang="ts">
  import { app } from "../store.svelte";
  import { goodName, labelOf, t } from "../i18n/index.svelte";
  import { mostExpensive } from "../overview";

  // Static info (produces, hub) comes from world_info; dynamic (π, goods) from the snapshot.
  const info = $derived(new Map((app.world?.nodes ?? []).map((n) => [n.id, n])));
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
          {@const dear = mostExpensive(n.goods)}
          <tr class="clickable" onclick={() => app.openNode(n.node)}>
            <td><strong>{n.node}</strong></td>
            <td class="muted">{labelOf("kind", n.kind)}</td>
            <td>{n.population.toFixed(0)}</td>
            <td>{n.prosperity.toFixed(2)}</td>
            <td>{info.get(n.node)?.produces.map(goodName).join(", ") || "—"}</td>
            <td>{#if dear}<span class="tag bad">⚠ {t("settlements.expensive")}: {goodName(dear.good)} ×{dear.multiplier.toFixed(2)}</span>{/if}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="muted">{t("prices.noWorld")}</p>
  {/if}
</div>
