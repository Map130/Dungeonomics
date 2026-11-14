<script lang="ts">
  import { app } from "../store.svelte";

  const r = $derived(app.report);
  function verdictClass(v: string): string {
    if (v === "дефицит") return "bad";
    if (v === "избыток") return "good";
    return "norm";
  }
</script>

<div class="panel">
  <h2>Лог причин</h2>
  {#if r}
    <div class="row" style="justify-content: space-between;">
      <strong>{r.good} · {r.node}</strong>
      <span class={"tag " + verdictClass(r.verdict)}>{r.verdict}</span>
    </div>
    <p class="muted" style="margin:.4rem 0;">
      множитель ×{r.multiplier.toFixed(2)} · обеспеченность {(r.provisioning * 100).toFixed(0)}%
    </p>
    <ul style="margin:0; padding-left: 1.1rem;">
      {#each r.causes as c}
        <li>{c.text}</li>
      {/each}
    </ul>
  {:else}
    <p class="muted">Кликните по цене в таблице, чтобы увидеть причины её уровня.</p>
  {/if}
</div>
