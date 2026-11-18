<script lang="ts">
  import { app } from "../store.svelte";
  import type { Cause, Verdict } from "../types";

  const r = $derived(app.report);
  function verdictClass(v: Verdict): string {
    if (v === "shortage") return "bad";
    if (v === "surplus") return "good";
    return "norm";
  }

  // Временный русский рендер структурированных причин; каталоги EN/RU — в R12.
  const VERDICT: Record<Verdict, string> = { shortage: "дефицит", surplus: "избыток", normal: "норма" };
  const PARAM: Record<string, string> = { pcap: "производство", garrison: "гарнизон", population: "население" };

  function causeText(c: Cause): string {
    if (c.kind === "stock") return `запас ${(c.value * 100).toFixed(0)}% от целевого`;
    const label = c.kind === "edge_effect" ? `дорога ${c.src}→${c.dst}` : (PARAM[c.param] ?? c.param);
    const factor = c.op === "mult" ? `×${c.value.toFixed(2)}` : `+${c.value.toFixed(1)}`;
    return `${label} ${factor}${c.source ? ` (${c.source})` : ""}`;
  }
</script>

<div class="panel">
  <h2>Лог причин</h2>
  {#if r}
    <div class="row" style="justify-content: space-between;">
      <strong>{r.good} · {r.node}</strong>
      <span class={"tag " + verdictClass(r.verdict)}>{VERDICT[r.verdict]}</span>
    </div>
    <p class="muted" style="margin:.4rem 0;">
      множитель ×{r.multiplier.toFixed(2)} · обеспеченность {(r.provisioning * 100).toFixed(0)}%
    </p>
    <ul style="margin:0; padding-left: 1.1rem;">
      {#each r.causes as c}
        <li>{causeText(c)}</li>
      {/each}
    </ul>
  {:else}
    <p class="muted">Кликните по цене в таблице, чтобы увидеть причины её уровня.</p>
  {/if}
</div>
