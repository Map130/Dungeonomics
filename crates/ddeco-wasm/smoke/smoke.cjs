// Node smoke-тест упакованного WASM-пакета (цель `nodejs`). Проверяет именно слой
// serde-wasm-bindgen — форму JSON, которую host-тесты фасада проверить не могут.
//
// Запуск (см. README / CI):
//   wasm-pack build crates/ddeco-wasm --target nodejs --out-dir pkg-node
//   node crates/ddeco-wasm/smoke/smoke.cjs
//
// Путь к пакету переопределяется первым аргументом (по умолчанию ../pkg-node).

const fs = require("fs");
const path = require("path");
const assert = require("assert");

const pkgDir = process.argv[2] || path.join(__dirname, "..", "pkg-node");
const { Sim } = require(path.join(pkgDir, "ddeco_wasm.js"));

const worldPath = path.join(__dirname, "..", "..", "..", "worlds", "example_region.toml");
const world = fs.readFileSync(worldPath, "utf8");

function has(obj, keys, ctx) {
  for (const k of keys) {
    assert.ok(k in obj, `${ctx}: нет поля "${k}" (${JSON.stringify(Object.keys(obj))})`);
  }
}

// --- Конструктор и такты ------------------------------------------------------
// `day` (i64) приходит из wasm-bindgen как BigInt — сравниваем через Number().
const sim = new Sim(world, 0);
assert.strictEqual(Number(sim.day), 0, "стартовый день = 0");
sim.step();
assert.strictEqual(Number(sim.day), 1, "после step день = 1");
sim.run(5);
assert.strictEqual(Number(sim.day), 6, "после run(5) день = 6");

// --- world_info ---------------------------------------------------------------
const info = sim.world_info();
has(info, ["seed", "nodes", "goods", "edges"], "world_info");
assert.ok(Array.isArray(info.nodes) && info.nodes.length > 0, "world_info.nodes непустой");
has(info.nodes[0], ["id", "kind", "population", "spread", "hub"], "world_info.nodes[0]");
has(info.goods[0], ["id", "category", "base_price"], "world_info.goods[0]");
const node = info.nodes[0].id;
const good = info.goods[0].id;

// --- snapshot -----------------------------------------------------------------
const snap = sim.snapshot();
has(snap, ["day", "nodes"], "snapshot");
assert.strictEqual(Number(snap.day), 6, "snapshot.day согласован");
const g0 = snap.nodes[0].goods[0];
has(g0, ["good", "stock", "multiplier", "price", "buy_price", "sell_price", "in_transit"], "GoodView");
assert.ok(g0.buy_price >= g0.sell_price, "buy_price >= sell_price (спред)");

// --- explain (§5.12) ----------------------------------------------------------
const rep = sim.explain(node, good);
has(rep, ["node", "good", "multiplier", "provisioning", "verdict", "causes"], "explain");
assert.ok(Array.isArray(rep.causes) && rep.causes.length > 0, "explain.causes непустой");
has(rep.causes[0], ["kind", "magnitude", "value", "op", "param", "src", "dst", "source"], "cause");
assert.ok(["shortage", "surplus", "normal"].includes(rep.verdict), "verdict is a code");
assert.ok(["node_effect", "edge_effect", "stock"].includes(rep.causes[0].kind), "cause.kind is a code");

// --- quote (§5.7) -------------------------------------------------------------
const q = sim.quote(node, good, 10, false);
has(q, ["good", "qty", "total", "avg_price", "new_multiplier"], "quote");
assert.ok(q.avg_price > 0, "avg_price > 0");

// --- add_effect + effects() ---------------------------------------------------
sim.add_effect("node", node, "pcap", "mult", 0.3, 20, "linear", 1, null, "food", "неурожай");
const effects = sim.effects();
assert.ok(Array.isArray(effects) && effects.length >= 1, "effects() содержит добавленный эффект");
has(effects[0], ["index", "target_kind", "target_id", "param", "op", "factor", "days_left"], "EffectView");
assert.ok(typeof effects[0].factor === "string", "factor — строка");

// --- flows() / caravans() -----------------------------------------------------
const flows = sim.flows();
assert.ok(Array.isArray(flows) && flows.length > 0, "flows() непустой");
has(flows[0], ["edge", "src", "dst", "in_transit", "accum", "threshold"], "FlowView");
assert.ok(Array.isArray(sim.caravans()), "caravans() — массив");

// --- save / load_state --------------------------------------------------------
const dump = sim.save();
assert.ok(dump.includes("day = 6"), "save() содержит день");
const other = new Sim(world, 0);
other.load_state(dump);
assert.strictEqual(Number(other.day), 6, "load_state восстановил день");

console.log("wasm smoke: OK");
