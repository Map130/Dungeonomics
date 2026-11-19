<script lang="ts">
  import { app } from "../store.svelte";
  import { goodName, t } from "../i18n/index.svelte";
  import { PRESETS, expandPresetOnTargets, presetName, roadsFromEdges } from "../presets";

  // `mode="edge"`: road presets on the world overview (pick a road, both arcs).
  // `mode="node"`: settlement presets on a node page, targeting `node`.
  let { mode, node = "" }: { mode: "node" | "edge"; node?: string } = $props();

  const presets = $derived(PRESETS.filter((p) => p.target === mode));
  let presetId = $state(PRESETS.find((p) => p.target === mode)?.id ?? PRESETS[0].id);
  let targetId = $state("");
  let good = $state("");
  let duration = $state(PRESETS[0].duration);

  const preset = $derived(presets.find((p) => p.id === presetId) ?? presets[0]);
  const targets = $derived(
    mode === "node"
      ? node
        ? [{ key: node, label: node, arcs: [node] }]
        : []
      : roadsFromEdges(app.world?.edges ?? []),
  );
  const goods = $derived(app.world?.goods.map((g) => g.id) ?? []);
  const durationOk = $derived(Number.isFinite(duration) && duration >= 1);
  const blocked = $derived(!preset || (preset.needsGood && !good) || !targetId || !durationOk);

  // Keep targetId valid when the target list changes (another node, another world).
  $effect(() => {
    const keys = targets.map((r) => r.key);
    if (keys.length && !keys.includes(targetId)) targetId = keys[0];
    else if (!keys.length && targetId) targetId = "";
  });

  // Keep good valid when the world changes.
  $effect(() => {
    if (good && !goods.includes(good)) good = "";
  });

  // Reset duration to the preset default on preset change.
  $effect(() => {
    if (preset) duration = preset.duration;
  });

  function apply() {
    const target = targets.find((r) => r.key === targetId);
    if (!target || blocked) return;
    app.addEffects(
      expandPresetOnTargets(preset, target.arcs, {
        good: good || undefined,
        duration,
        source: presetName(preset.id),
      }),
    );
  }
</script>

<div class="panel">
  <h3>{t("presets.title")}</h3>
  <div class="row">
    <select bind:value={presetId}>
      {#each presets as p}<option value={p.id}>{presetName(p.id)}</option>{/each}
    </select>
    {#if mode === "edge"}
      <select bind:value={targetId}>
        {#each targets as r}<option value={r.key}>{r.label}</option>{/each}
      </select>
    {/if}
    {#if preset?.needsGood}
      <select bind:value={good} aria-label={t("presets.good")}>
        <option value="">{t("presets.good")}…</option>
        {#each goods as g}<option value={g}>{goodName(g)}</option>{/each}
      </select>
    {/if}
    <label class="muted">{t("effects.days")} <input type="number" min="1" bind:value={duration} style="width:4rem;" /></label>
    <button onclick={apply} disabled={blocked} title={preset?.needsGood && !good ? t("presets.needsGood") : ""}>{t("presets.apply")}</button>
  </div>
</div>
