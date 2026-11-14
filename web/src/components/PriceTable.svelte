<script lang="ts">
  import { app } from "../store.svelte";
  import type { GoodView } from "../types";

  // Товары берём из world_info для стабильного порядка колонок.
  const goods = $derived(app.world?.goods ?? []);

  function verdictClass(g: GoodView): string {
    if (g.stock <= 0) return "bad";
    if (g.multiplier > 1.1) return "bad";
    if (g.multiplier < 0.9) return "good";
    return "norm";
  }
</script>

<div class="panel">
  <h2>Цены по узлам <span class="muted">· клик по ячейке — причины</span></h2>
  {#if app.snapshot}
    <div style="overflow:auto; max-height: 48vh;">
      <table>
        <thead>
          <tr>
            <th>Узел</th>
            <th>π</th>
            {#each goods as g}
              <th title={g.category}>{g.id}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each app.snapshot.nodes as n}
            <tr>
              <td title={n.kind}>{n.node}</td>
              <td>{n.prosperity.toFixed(2)}</td>
              {#each n.goods as gv}
                <td
                  class="clickable"
                  title={`запас ${gv.stock.toFixed(0)} · в пути ${gv.in_transit.toFixed(0)}\nпокупка ${gv.buy_price.toFixed(3)} / продажа ${gv.sell_price.toFixed(3)}`}
                  onclick={() => app.explain(n.node, gv.good)}
                >
                  <span class={"tag " + verdictClass(gv)}>×{gv.multiplier.toFixed(2)}</span>
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="muted">Мир не загружен.</p>
  {/if}
</div>
