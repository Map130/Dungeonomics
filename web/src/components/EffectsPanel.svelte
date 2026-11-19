<script lang="ts">
  import { app } from "../store.svelte";
  import { goodName, labelOf, paramLabel, t } from "../i18n/index.svelte";

  // `node`: show only effects on this settlement or on roads touching it (node page);
  // `showForm`: the manual "add effect" form (world overview only).
  let { node = "", showForm = true }: { node?: string; showForm?: boolean } = $props();

  const arcEnds = $derived(new Map((app.world?.edges ?? []).map((e) => [e.id, [e.src, e.dst]])));
  const shown = $derived(
    node
      ? app.effects.filter((e) =>
          e.target_kind === "node" ? e.target_id === node : (arcEnds.get(e.target_id) ?? []).includes(node),
        )
      : app.effects,
  );

  let targetKind = $state<"node" | "edge">("node");
  let targetId = $state("");
  let param = $state("pcap");
  let op = $state<"mult" | "add">("mult");
  let value = $state(0.3);
  let duration = $state(30);
  let decay = $state<"step" | "linear" | "exp">("linear");
  let tau = $state(5);
  let good = $state("");
  let category = $state("");
  let source = $state("");

  const targets = $derived(
    targetKind === "node"
      ? (app.world?.nodes.map((n) => n.id) ?? [])
      : (app.world?.edges.map((e) => e.id) ?? []),
  );
  const categories = $derived([...new Set((app.world?.goods ?? []).map((g) => g.category))].sort());
  const paramHints = $derived(
    targetKind === "node" ? ["pcap", "population", "garrison"] : ["difficulty", "capacity"],
  );

  // Keep targetId valid when the target kind changes.
  $effect(() => {
    if (targets.length && !targets.includes(targetId)) targetId = targets[0];
  });

  // Reset good/category when another world no longer has them.
  $effect(() => {
    const goods = app.world?.goods ?? [];
    if (good && !goods.some((g) => g.id === good)) good = "";
    if (category && !categories.includes(category)) category = "";
  });

  function submit() {
    if (!targetId) return;
    app.addEffect(
      targetKind,
      targetId,
      param,
      op,
      value,
      duration,
      decay,
      tau,
      good || null,
      category || null,
      source || "",
    );
  }
</script>

<div class="panel">
  <h2>{node ? t("node.effects") : t("effects.title")} <span class="muted">· {t("effects.active")}</span></h2>

  {#if shown.length}
    <ul style="margin:0 0 .6rem; padding-left: 1.1rem;">
      {#each shown as e}
        <li class="row" style="justify-content: space-between;">
          <span>
            <strong>{e.factor}</strong> {paramLabel(e.param)} · {t(e.target_kind === "edge" ? "effects.edge" : "effects.node")} {e.target_id}
            {#if e.good}· {goodName(e.good)}{:else if e.category}· {e.category}{/if}
            {#if e.source}<span class="muted">({e.source})</span>{/if}
            <span class="muted">{t("effects.daysLeft", { days: e.days_left.toFixed(0) })}</span>
          </span>
          <button onclick={() => app.removeEffect(e.index)}>{t("effects.remove")}</button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted" style="margin:.2rem 0 .6rem;">{t("effects.none")}</p>
  {/if}

  {#if showForm}
  <h3>{t("effects.add")}</h3>
  <div class="row">
    <select bind:value={targetKind}>
      <option value="node">{t("effects.node")}</option>
      <option value="edge">{t("effects.edge")}</option>
    </select>
    <select bind:value={targetId}>
      {#each targets as t}<option value={t}>{t}</option>{/each}
    </select>
    <input list="param-hints" bind:value={param} placeholder={t("effects.param")} style="width:7rem;" />
    <datalist id="param-hints">
      {#each paramHints as p}<option value={p}></option>{/each}
    </datalist>
    <select bind:value={op}>
      <option value="mult">×</option>
      <option value="add">+</option>
    </select>
    <input type="number" step="0.05" bind:value placeholder={t("effects.value")} style="width:5rem;" />
  </div>
  <div class="row" style="margin-top:.4rem;">
    <label class="muted">{t("effects.days")} <input type="number" bind:value={duration} style="width:4rem;" /></label>
    <select bind:value={decay}>
      <option value="step">step</option>
      <option value="linear">linear</option>
      <option value="exp">exp</option>
    </select>
    {#if decay === "exp"}
      <label class="muted">τ <input type="number" bind:value={tau} style="width:3.5rem;" /></label>
    {/if}
    <select bind:value={good} title={t("effects.good")}>
      <option value="">{t("effects.good")}</option>
      {#each app.world?.goods ?? [] as g}<option value={g.id}>{goodName(g.id)}</option>{/each}
    </select>
    <select bind:value={category} title={t("effects.category")}>
      <option value="">{t("effects.category")}</option>
      {#each categories as c}<option value={c}>{labelOf("category", c)}</option>{/each}
    </select>
    <input bind:value={source} placeholder={t("effects.source")} style="width:8rem;" />
    <button onclick={submit}>{t("effects.submit")}</button>
  </div>
  {/if}
</div>
