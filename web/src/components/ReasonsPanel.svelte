<script lang="ts">
  import { app } from "../store.svelte";
  import { causeText, t, verdictLabel } from "../i18n/index.svelte";
  import type { Verdict } from "../types";

  const r = $derived(app.report);
  function verdictClass(v: Verdict): string {
    if (v === "shortage") return "bad";
    if (v === "surplus") return "good";
    return "norm";
  }
</script>

<div class="panel">
  <h2>{t("reasons.title")}</h2>
  {#if r}
    <div class="row" style="justify-content: space-between;">
      <strong>{r.good} · {r.node}</strong>
      <span class={"tag " + verdictClass(r.verdict)}>{verdictLabel(r.verdict)}</span>
    </div>
    <p class="muted" style="margin:.4rem 0;">
      {t("reasons.summary", { m: r.multiplier.toFixed(2), r: (r.provisioning * 100).toFixed(0) })}
    </p>
    <ul style="margin:0; padding-left: 1.1rem;">
      {#each r.causes as c}
        <li>{causeText(c)}</li>
      {/each}
    </ul>
  {:else}
    <p class="muted">{t("reasons.empty")}</p>
  {/if}
</div>
