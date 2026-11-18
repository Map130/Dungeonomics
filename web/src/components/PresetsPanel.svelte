<script lang="ts">
  import { app } from "../store.svelte";
  import { t } from "../i18n/index.svelte";
  import { PRESETS, expandPreset, presetName } from "../presets";

  let presetId = $state(PRESETS[0].id);
  let targetId = $state("");
  let good = $state("");
  let duration = $state(PRESETS[0].duration);

  const preset = $derived(PRESETS.find((p) => p.id === presetId) ?? PRESETS[0]);
  const targets = $derived(
    preset.target === "node"
      ? (app.world?.nodes.map((n) => n.id) ?? [])
      : (app.world?.edges.map((e) => e.id) ?? []),
  );
  const goods = $derived(app.world?.goods.map((g) => g.id) ?? []);
  const blocked = $derived(preset.needsGood && !good);

  // Keep targetId valid when the preset (and so the target kind) changes.
  $effect(() => {
    if (targets.length && !targets.includes(targetId)) targetId = targets[0];
  });

  // Reset duration to the preset default on preset change.
  $effect(() => {
    duration = preset.duration;
  });

  function apply() {
    if (!targetId || blocked) return;
    app.addEffects(
      expandPreset(preset, targetId, { good: good || undefined, duration, source: presetName(preset.id) }),
    );
  }
</script>

<div class="panel">
  <h2>{t("presets.title")}</h2>
  <div class="row">
    <select bind:value={presetId}>
      {#each PRESETS as p}<option value={p.id}>{presetName(p.id)}</option>{/each}
    </select>
    <select bind:value={targetId}>
      {#each targets as id}<option value={id}>{id}</option>{/each}
    </select>
    {#if preset.needsGood}
      <select bind:value={good} aria-label={t("presets.good")}>
        <option value="">{t("presets.good")}…</option>
        {#each goods as g}<option value={g}>{g}</option>{/each}
      </select>
    {/if}
    <label class="muted">{t("effects.days")} <input type="number" min="1" bind:value={duration} style="width:4rem;" /></label>
    <button onclick={apply} disabled={blocked} title={blocked ? t("presets.needsGood") : ""}>{t("presets.apply")}</button>
  </div>
</div>
