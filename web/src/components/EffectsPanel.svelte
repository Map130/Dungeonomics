<script lang="ts">
  import { app } from "../store.svelte";

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
  const paramHints = $derived(
    targetKind === "node" ? ["pcap", "population", "garrison"] : ["difficulty", "capacity"],
  );

  // Держим targetId валидным при смене вида цели.
  $effect(() => {
    if (targets.length && !targets.includes(targetId)) targetId = targets[0];
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
  <h2>Эффекты <span class="muted">· активны сейчас</span></h2>

  {#if app.effects.length}
    <ul style="margin:0 0 .6rem; padding-left: 1.1rem;">
      {#each app.effects as e}
        <li class="row" style="justify-content: space-between;">
          <span>
            <strong>{e.factor}</strong> {e.param} · {e.target_kind} {e.target_id}
            {#if e.good}· {e.good}{:else if e.category}· {e.category}{/if}
            {#if e.source}<span class="muted">({e.source})</span>{/if}
            <span class="muted">— ещё {e.days_left.toFixed(0)} дн.</span>
          </span>
          <button onclick={() => app.removeEffect(e.index)}>снять</button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted" style="margin:.2rem 0 .6rem;">Активных эффектов нет.</p>
  {/if}

  <h3>Добавить эффект</h3>
  <div class="row">
    <select bind:value={targetKind}>
      <option value="node">узел</option>
      <option value="edge">дорога</option>
    </select>
    <select bind:value={targetId}>
      {#each targets as t}<option value={t}>{t}</option>{/each}
    </select>
    <input list="param-hints" bind:value={param} placeholder="параметр" style="width:7rem;" />
    <datalist id="param-hints">
      {#each paramHints as p}<option value={p}></option>{/each}
    </datalist>
    <select bind:value={op}>
      <option value="mult">×</option>
      <option value="add">+</option>
    </select>
    <input type="number" step="0.05" bind:value placeholder="значение" style="width:5rem;" />
  </div>
  <div class="row" style="margin-top:.4rem;">
    <label class="muted">дн. <input type="number" bind:value={duration} style="width:4rem;" /></label>
    <select bind:value={decay}>
      <option value="step">step</option>
      <option value="linear">linear</option>
      <option value="exp">exp</option>
    </select>
    {#if decay === "exp"}
      <label class="muted">τ <input type="number" bind:value={tau} style="width:3.5rem;" /></label>
    {/if}
    <input bind:value={good} placeholder="товар (опц.)" style="width:6rem;" />
    <input bind:value={category} placeholder="категория (опц.)" style="width:7rem;" />
    <input bind:value={source} placeholder="причина" style="width:8rem;" />
    <button onclick={submit}>добавить</button>
  </div>
</div>
