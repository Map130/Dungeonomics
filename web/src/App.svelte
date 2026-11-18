<script lang="ts">
  import { app } from "./store.svelte";
  import defaultWorld from "./defaultWorld";
  import PriceTable from "./components/PriceTable.svelte";
  import ReasonsPanel from "./components/ReasonsPanel.svelte";
  import EffectsPanel from "./components/EffectsPanel.svelte";
  import PresetsPanel from "./components/PresetsPanel.svelte";
  import FlowsPanel from "./components/FlowsPanel.svelte";
  import GmPanel from "./components/GmPanel.svelte";
  import { i18n, LANGS, t, type Lang } from "./i18n/index.svelte";

  let runDays = $state(30);
  let warmup = $state(0);

  $effect(() => {
    // Initialise WASM once, then load the default world.
    app.init().then(() => app.loadWorld(defaultWorld, 0));
  });

  async function onWorldFile(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (f) app.loadWorld(await f.text(), warmup);
  }
  async function onStateFile(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (f) app.loadState(await f.text());
  }
  function download(name: string, text: string) {
    const url = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = name;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<header class="panel" style="border-radius:0; display:flex; gap:1rem; align-items:center; flex-wrap:wrap;">
  <h1 style="margin:0; font-size:1.1rem;">Dungeonomics</h1>
  {#if app.ready && app.sim}
    <span class="tag">{t("app.day", { day: app.day })}</span>
    <div class="row">
      <button onclick={() => app.step()}>{t("app.step")}</button>
      <button onclick={() => app.run(runDays)}>+{runDays}</button>
      <input type="number" bind:value={runDays} style="width:4rem;" />
      <button onclick={() => app.undo()}>{t("app.undo")}</button>
    </div>
    {#if app.hasShocks}
      <label class="row">
        <input
          type="checkbox"
          checked={app.shocksEnabled}
          onchange={(e) => app.setShocks((e.target as HTMLInputElement).checked)}
        />
        {t("app.shocks")}
      </label>
    {/if}
    <div class="row" style="margin-left:auto;">
      <label class="muted">{t("app.warmup")} <input type="number" bind:value={warmup} style="width:4rem;" /></label>
      <label class="muted">{t("app.world")} <input type="file" accept=".toml" onchange={onWorldFile} /></label>
      <label class="muted">{t("app.state")} <input type="file" accept=".toml" onchange={onStateFile} /></label>
      <button onclick={() => download("state.toml", app.save())}>{t("app.save")}</button>
    </div>
  {:else}
    <span class="muted">{t("app.loading")}</span>
  {/if}
  <select
    aria-label={t("app.language")}
    value={i18n.lang}
    onchange={(e) => i18n.set((e.target as HTMLSelectElement).value as Lang)}
    style={app.ready && app.sim ? "" : "margin-left:auto;"}
  >
    {#each LANGS as l}<option value={l.id}>{l.label}</option>{/each}
  </select>
</header>

{#if app.error}
  <div class="panel" style="border-radius:0; border-color: var(--bad); color: var(--bad);">
    {app.error}
  </div>
{/if}

<main
  style="padding: 1rem; display:grid; gap:1rem; grid-template-columns: 2fr 1fr; align-items:start;"
>
  <div style="display:grid; gap:1rem;">
    <PriceTable />
    <FlowsPanel />
  </div>
  <div style="display:grid; gap:1rem;">
    <ReasonsPanel />
    <GmPanel />
    <PresetsPanel />
    <EffectsPanel />
  </div>
</main>
