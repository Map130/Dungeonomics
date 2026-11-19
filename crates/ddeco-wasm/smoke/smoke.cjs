// Node smoke test of the packaged WASM module (the `nodejs` target). It checks exactly the
// serde-wasm-bindgen layer — the JSON shape that the facade's host tests cannot check.
//
// Usage (see README / CI):
//   wasm-pack build crates/ddeco-wasm --target nodejs --out-dir pkg-node
//   node crates/ddeco-wasm/smoke/smoke.cjs
//
// The package path can be overridden by the first argument (default ../pkg-node).

const fs = require("fs");
const path = require("path");
const assert = require("assert");

const pkgDir = process.argv[2] || path.join(__dirname, "..", "pkg-node");
const { Sim } = require(path.join(pkgDir, "ddeco_wasm.js"));

const worldPath = path.join(__dirname, "..", "..", "..", "worlds", "example_region.toml");
const world = fs.readFileSync(worldPath, "utf8");

function has(obj, keys, ctx) {
  for (const k of keys) {
    assert.ok(k in obj, `${ctx}: missing field "${k}" (${JSON.stringify(Object.keys(obj))})`);
  }
}

// --- Constructor and ticks -----------------------------------------------------
// `day` (i64) comes from wasm-bindgen as a BigInt — compare via Number().
const sim = new Sim(world, 0);
assert.strictEqual(Number(sim.day), 0, "starting day = 0");
sim.step();
assert.strictEqual(Number(sim.day), 1, "after step day = 1");
sim.run(5);
assert.strictEqual(Number(sim.day), 6, "after run(5) day = 6");

// --- world_info ---------------------------------------------------------------
const info = sim.world_info();
has(info, ["seed", "nodes", "goods", "edges"], "world_info");
assert.ok(Array.isArray(info.nodes) && info.nodes.length > 0, "world_info.nodes is non-empty");
has(info.nodes[0], ["id", "kind", "population", "spread", "hub", "produces"], "world_info.nodes[0]");
assert.ok(info.nodes.some((n) => n.produces.length > 0), "some node produces goods");
has(info.goods[0], ["id", "category", "base_price", "names"], "world_info.goods[0]");

// Display names come from `name = { en, ru }` on a good; absent -> empty map.
// The example world names every good; stripping the tables gives empty maps.
assert.ok(
  info.goods.every((g) => typeof g.names.en === "string" && typeof g.names.ru === "string"),
  "every example-world good carries en/ru names",
);
{
  const unnamed = world.replace(/^name = \{[^\n]*\}\n/gm, "");
  assert.notStrictEqual(unnamed, world, "the example world has name tables to strip");
  const ni = new Sim(unnamed, 0).world_info();
  assert.ok(ni.goods.every((g) => Object.keys(g.names).length === 0), "goods without names -> empty map");
}
const node = info.nodes[0].id;
const good = info.goods[0].id;

// --- snapshot -----------------------------------------------------------------
const snap = sim.snapshot();
has(snap, ["day", "nodes"], "snapshot");
assert.strictEqual(Number(snap.day), 6, "snapshot.day is consistent");
const g0 = snap.nodes[0].goods[0];
has(g0, ["good", "stock", "multiplier", "price", "buy_price", "sell_price", "in_transit", "provisioning", "verdict"], "GoodView");
assert.ok(["shortage", "surplus", "normal"].includes(g0.verdict), "GoodView.verdict is a code");
assert.ok(g0.buy_price >= g0.sell_price, "buy_price >= sell_price (spread)");

// --- explain (§5.12) ----------------------------------------------------------
const rep = sim.explain(node, good);
has(rep, ["node", "good", "multiplier", "provisioning", "verdict", "causes"], "explain");
assert.ok(Array.isArray(rep.causes) && rep.causes.length > 0, "explain.causes is non-empty");
has(rep.causes[0], ["kind", "magnitude", "value", "op", "param", "src", "dst", "source"], "cause");
assert.ok(["shortage", "surplus", "normal"].includes(rep.verdict), "verdict is a code");
assert.ok(["node_effect", "edge_effect", "stock"].includes(rep.causes[0].kind), "cause.kind is a code");

// --- quote (§5.7) -------------------------------------------------------------
const q = sim.quote(node, good, 10, false);
has(q, ["good", "qty", "total", "avg_price", "new_multiplier"], "quote");
assert.ok(q.avg_price > 0, "avg_price > 0");

// --- add_effect + effects() ---------------------------------------------------
sim.add_effect("node", node, "pcap", "mult", 0.3, 20, "linear", 1, null, "food", "blight");
const effects = sim.effects();
assert.ok(Array.isArray(effects) && effects.length >= 1, "effects() contains the added effect");
has(effects[0], ["index", "target_kind", "target_id", "param", "op", "factor", "days_left"], "EffectView");
assert.ok(typeof effects[0].factor === "string", "factor is a string");

// --- add_effects (atomic batch, one undo step) --------------------------------
const nBefore = sim.effects().length;
sim.add_effects([
  { target_kind: "node", target_id: node, param: "pcap", op: "mult", value: 0.5, duration: 10, decay: "linear", good: null, category: "food", source: "batch" },
  { target_kind: "node", target_id: node, param: "population", op: "mult", value: 1.2, duration: 10, decay: "step", source: "batch" },
]);
assert.strictEqual(sim.effects().length, nBefore + 2, "add_effects adds both effects");
assert.throws(() => sim.add_effects([]), /no effects/, "empty batch rejected");
sim.undo();
assert.strictEqual(sim.effects().length, nBefore, "one undo reverts the whole batch");

// --- flows() / caravans() -----------------------------------------------------
const flows = sim.flows();
assert.ok(Array.isArray(flows) && flows.length > 0, "flows() is non-empty");
has(flows[0], ["edge", "src", "dst", "in_transit", "accum", "threshold"], "FlowView");
assert.ok(Array.isArray(sim.caravans()), "caravans() is an array");

// --- save / load_state --------------------------------------------------------
const dump = sim.save();
assert.ok(dump.includes("day = 6"), "save() contains the day");
const other = new Sim(world, 0);
other.load_state(dump);
assert.strictEqual(Number(other.day), 6, "load_state restored the day");

console.log("wasm smoke: OK");
