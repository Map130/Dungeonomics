<script lang="ts">
  import { app } from "../store.svelte";
  import { t } from "../i18n/index.svelte";

  // Only show arcs where something is in transit or flow is accumulating.
  const active = $derived(app.flows.filter((f) => f.in_transit > 0 || f.accum > 0));
</script>

<div class="panel">
  <h2>{t("flows.title")}</h2>

  <h3>{t("flows.roads")}</h3>
  {#if active.length}
    <table>
      <thead>
        <tr>
          <th>{t("flows.road")}</th><th>{t("flows.inTransit")}</th><th>{t("flows.accum")}</th><th
            >{t("flows.toTheta")}</th
          >
        </tr>
      </thead>
      <tbody>
        {#each active as f}
          <tr>
            <td>{f.src}→{f.dst}</td>
            <td>{f.in_transit.toFixed(0)}</td>
            <td>{f.accum.toFixed(0)}</td>
            <td class="muted">{(100 * (f.accum / f.threshold)).toFixed(0)}%</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="muted" style="margin:.2rem 0;">{t("flows.none")}</p>
  {/if}

  <h3 style="margin-top:.7rem;">{t("caravans.title")} <span class="muted">({app.caravans.length})</span></h3>
  {#if app.caravans.length}
    <ul style="margin:0; padding-left:1.1rem; max-height: 22vh; overflow:auto;">
      {#each app.caravans.slice().reverse() as c}
        <li class="row" style="justify-content: space-between;">
          <span>
            <strong>{t("caravans.name", { src: c.src, dst: c.dst })}</strong>
            <span class="muted">
              {t("caravans.details", {
                from: c.day_departed,
                to: c.day_arrival,
                value: c.value.toFixed(0),
                guard: c.guard.toFixed(1),
              })}
            </span>
          </span>
          <button onclick={() => app.lootCaravan(c.edge, 0.5)}>{t("caravans.loot")}</button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted" style="margin:.2rem 0;">{t("caravans.none")}</p>
  {/if}
</div>
