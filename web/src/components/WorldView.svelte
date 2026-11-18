<script lang="ts">
  import SettlementsTable from "./SettlementsTable.svelte";
  import FlowsPanel from "./FlowsPanel.svelte";
  import PresetsPanel from "./PresetsPanel.svelte";
  import EffectsPanel from "./EffectsPanel.svelte";

  // The world overview: settlements, road flows and caravans, events. Per-good prices live on
  // the settlement pages (NodeView).
</script>

<main style="padding: 1rem; display:grid; gap:1rem; grid-template-columns: 3fr 2fr; align-items:start;">
  <div style="display:grid; gap:1rem;">
    <SettlementsTable />
    <FlowsPanel />
  </div>
  <div style="display:grid; gap:1rem;">
    <PresetsPanel mode="edge" />
    <EffectsPanel />
  </div>
</main>
