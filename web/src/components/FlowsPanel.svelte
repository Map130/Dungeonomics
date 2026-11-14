<script lang="ts">
  import { app } from "../store.svelte";

  // Показываем только дуги, где что-то едет или копится поток.
  const active = $derived(app.flows.filter((f) => f.in_transit > 0 || f.accum > 0));
</script>

<div class="panel">
  <h2>Потоки и караваны</h2>

  <h3>Поток по дорогам</h3>
  {#if active.length}
    <table>
      <thead>
        <tr><th>дорога</th><th>в пути</th><th>накоплено</th><th>до θ</th></tr>
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
    <p class="muted" style="margin:.2rem 0;">Активных потоков нет.</p>
  {/if}

  <h3 style="margin-top:.7rem;">Караваны <span class="muted">({app.caravans.length})</span></h3>
  {#if app.caravans.length}
    <ul style="margin:0; padding-left:1.1rem; max-height: 22vh; overflow:auto;">
      {#each app.caravans.slice().reverse() as c}
        <li class="row" style="justify-content: space-between;">
          <span>
            <strong>{c.source}</strong>
            <span class="muted">
              дни {c.day_departed}→{c.day_arrival} · ценность {c.value.toFixed(0)} · охрана {c.guard.toFixed(1)}
            </span>
          </span>
          <button onclick={() => app.lootCaravan(c.edge, 0.5)}>ограбить ½</button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted" style="margin:.2rem 0;">Караванов пока нет.</p>
  {/if}
</div>
