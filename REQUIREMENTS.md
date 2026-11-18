# A living economy for tabletop RPGs: requirements v1

Status: draft. The mathematical model is described in a separate document (`MODEL.md`).
Items marked "(proposed)" come from an analysis of the Bannerlord economy and a discussion of stability. The author has not confirmed them yet.

## 1. Goal

An open-source tool for tabletop RPG game masters (primarily D&D). The GM describes a region once (nodes, roads, goods), and from then on player actions and world events change prices, availability and trade flows on their own. No manual bookkeeping is needed.

The idea is borrowed from the economy of Mount & Blade: Bannerlord, where villages, towns, workshops and caravans form a connected system that the player influences directly. No such tool exists for D&D yet.

## 2. User and use case

- **User:** a GM, not a programmer. They need quick setup and clear explanations of "why is it like this".
- **Scenario:** a one-shot or a short campaign. The game involves a region of 3-4 cities; the rest of the world is not modelled.
- **Typical evening:** the players take 3-5 notable actions (kill a craftsman, rob a caravan, buy out a stock, block a road). Each should produce a visible and explainable effect within 1-3 in-game days.

## 3. Design principles

1. **Playability over realism.** The economy is calibrated to player income. An "honest" calculation in which bread costs 50 gold is considered a bug.
2. **Parameters separate from state.** Events change only parameters (production, consumption, travel difficulty, capacity). The state (stocks, prices, goods in transit) evolves on its own.
3. **Explainability.** For every change in price or availability the system can name the cause. This matters more than mathematical precision.
4. **System-agnostic.** Prices are set in abstract units relative to a base price list (the SRD or the GM's homebrew). The tool is not tied to a specific system.
5. **A hybrid of the "invisible hand" and agents.** Background trade runs as a flow without agents. Named caravans appear only where the story needs them.
6. **Determinism.** With one seed and the same inputs the result is reproducible.
7. **Stability without external input (proposed).** A world without events does not "run away": prices do not drift to the corridor edges, stocks do not grow without bound (see 5.10).
8. **Player income is bounded by market capacity (proposed).** The players' own production or trade runs into market demand, not into infinity.

## 4. Scope of v1

- Region: 3-4 cities, about 10-20 nodes (villages, cities, camps).
- One simulation, one GM, local use.
- Expansion to a continent is possible later and must not require a change of architecture.

## 5. Functional requirements

### 5.1 World
- Nodes: village, city, camp, etc. Villages are bound to the nearest city (a hierarchy on top of the graph, as in Bannerlord).
- Edges (roads): base difficulty, throughput, length or travel time.
- Supply graph: producers, consumers, routes.

### 5.2 Goods
- Goods catalogue: base price, category, demand elasticity, spoilage or storage cost.
- Goods categories (food, cloth, weapons, etc.) for demand calculation, so the GM does not have to describe every item separately.
- Production chains (grain → flour → bread): desirable in v1, v1.1 at the latest. **Decided: v1.1 (`DECISIONS.md` D3).**

### 5.3 Nodes
- Per good: production per day, consumption per day, stock, target stock.
- Craftsmen and workshops as producers with a "capacity". Killing a craftsman zeroes or reduces the capacity.
- Consumption depends on the node's population and prosperity.
- **Node prosperity (proposed):** a slow variable. It grows with the food stock and the variety of goods on the market, hits a soft ceiling (an analogue of Bannerlord's "housing cost"), and falls with shortages and raids. Consumption and taxes depend on it. Formulas and numbers are defined in the mathematical model.

### 5.4 Effects (events)
- An effect is a parameter modifier: a multiplier or an addend.
- Attributes: target (node, edge, good or category), type, magnitude, duration, decay (instant, linear, exponential), source (text for the log).
- The resulting parameter value: `base × Π(multipliers) + Σ(addends)`. Effects combine correctly; removing one does not break the others.
- One effect can hit different goods differently (war cuts weapons and food with different strength).
- Examples: a monster on the road (×2.5 difficulty, 10 days, linear decay), a murdered craftsman, an epidemic, a war, a raid on a village.

### 5.5 Prices
- Price = base price × multiplier. The simulation moves only the multiplier.
- The multiplier is derived from `r = stock / target_stock` and clamped to a corridor. Guideline: ordinary goods ×0.5…×3, necessities ×0.7…×1.8. The values are configurable.
- Smoothing: the multiplier drifts towards its target rather than jumping.
- A buy/sell spread (around ±10-15%), configurable per node or guild.
- A shortage shows both in price and in availability. The price is bounded by the corridor.
- **Food as a "basket" (proposed):** the contribution to stock and prosperity accounts for variety, not only total volume.
- **Luxury (proposed):** consumed by high-prosperity nodes; demand for it falls in poor nodes.
- **Village and city prices are linked (proposed):** a producing village is tied to its city's market; arrivals and purchases move the price in both places.
- **Purchases move the price (proposed):** a large purchase or sale moves the price on every unit; the player pays the average along the curve, not the price tag. This protects against buying out the market.

### 5.6 Trade and flow
- Background trade: if the price difference between nodes exceeds the travel cost, part of the stock moves; the volume is limited by the edge capacity.
- Transport delay: goods in transit exist as a buffer on the edge.
- Named caravans as auto-generated events: the system periodically "materialises" a large flow ("in 3 days a caravan with 40 coins' worth of grain leaves A for B, weakly guarded"). The GM decides whether to let the players intervene.
- A game action on a caravan (robbery) removes part of the goods from transit; everything else is computed as usual.

### 5.7 Simulation
- Time step: 1 day.
- A "run N days" button.
- A "warm up the world" button: a simulation without events until prices settle, so the world starts in equilibrium.
- State snapshots and undo.

### 5.8 Output and interface
- A table of prices by node (buy and sell) for quick use at the table.
- Flows along roads, shortages and surpluses.
- A feed of events and auto-generated caravans.
- **Reasons log:** "the price of bread in X went up because road Y→X is ×2.5 and the stock fell to 40%". The core returns causes in structured form; the UI builds the text in the user's language (D5, MODEL §5.12).
- Quick addition of an effect from the interface, including one-click event presets (craftsman killed, blight, army billeted, epidemic, bandits, bridge collapsed).
- Direct stock edits (a player bought out a good, the GM enters it by hand).
- A settlement shop lists every item of the base price list; a deal moves the price of the item's driver good (D8).

### 5.9 Data
- The world is stored in a single text file, convenient for versioning and sharing templates. **Decided: TOML (`DECISIONS.md` D1).**
- The simulation state is saved separately from the world description.

### 5.10 Stability and sinks (proposed)
Reason: in Bannerlord, without major wars central cities run away — prosperity grows, prices fall, the economy breaks (positive feedback without sinks). The model must have closing loops.
- **Supply responds to price:** with a long price decline production capacity falls (e.g. `p = p₀·(P/P₀)^β` with a clamp).
- **Consumption grows with wealth** no slower than production.
- **Spoilage and storage cost:** the stock loses a share daily; unbounded stock growth is impossible.
- **Prosperity ceiling:** soft, but firm enough to stop runaway growth.
- **Sinks:** taxes, upkeep of garrisons and armies, and tolls constantly remove part of the goods and money from the system.
- **Background events:** a seeded generator of random shocks (blight, epidemic, raid). The GM can turn it off.
- **A billeted army** eats up a node's market: a standard consumption shock.
- **Players hauling in goods by hand** prolong a node's growth, but once it stops a deeper slump is possible. The model must not smooth this effect away (an observation from Bannerlord).

## 6. Non-functional requirements

- **Local:** works without accounts, servers or internet (after loading).
- **Low barrier to entry:** the GM opens a page or runs one file, installing nothing.
- **Performance:** recomputing a day on 10-20 nodes is instant. Optimisation is not a priority, but the architecture must not prevent growth to hundreds of nodes.
- **Reproducibility:** a seed for random events.
- **Stability, acceptance criterion (proposed):** an autotest. A 1000+ day run without events converges to a steady state, prices do not stick to the corridor edges, stocks do not grow without bound.
- **Licence:** open source. **Decided: MIT (`DECISIONS.md` D4).**

## 7. Architecture (high level)

- **Core:** a Rust library (simulation, effects, reasons log). Independent of the UI.
- **Web interface:** the core compiles to WASM, a static site that works offline (PWA); the world is saved and loaded as a file.
- **Desktop:** the same interface packaged as an .exe (e.g. Tauri) at release time.
- **Maths prototype:** in Python if needed, before porting to Rust.

## 8. Non-goals of v1

- Real time and multiplayer.
- Traders as full agents with their own logic and wallets.
- Tracking individual player finances.
- Modelling a continent and long campaigns (years of in-game time).
- Strict global equilibrium (dual variables of min-cost flow). A local price plus arbitrage was chosen instead.

## 9. Risks

| Risk | Mitigation |
|---|---|
| The economy runs away or oscillates (cobweb model) | Price smoothing, multiplier corridor, small step |
| Prices do not match player income | Base from the system's price list, corridor, spread |
| The GM does not understand what is going on | Reasons log, event feed |
| Too much setup up front | World templates, categories instead of individual goods, world warm-up |
| Growing model complexity | Clear separation of parameters and state, the model is described separately |
| Runaway growth without external shocks (positive feedback, as in Bannerlord's central cities) | Supply response to price, spoilage, sinks, prosperity ceiling, a 1000+ day autotest |
| Players break the economy with their own production | Market capacity, purchases moving the price, spread |

## 10. Open questions

1. ~~World file format (YAML, JSON or TOML).~~ Decided: TOML, see `DECISIONS.md` D1.
2. ~~Production chains: in v1 or later.~~ Decided: v1.1, see D3.
3. Caravan materialisation rules: flow threshold, frequency, how guard strength is set.
4. ~~Where the base price list comes from: a built-in PHB template, import or manual entry.~~ Decided: SRD 5.1, `references/srd_gear.csv`, see D5 (superseded D2).
5. Demand and prosperity: a slow prosperity variable is proposed (food, variety, ceiling); its exact form is not chosen.
6. Units of goods: pieces, weight units or abstract "batches". Shop items: value-equivalent units of a driver good, see `DECISIONS.md` D8.
7. ~~Project name and licence.~~ Decided: Dungeonomics, MIT, see `DECISIONS.md` D4.
8. Stabilisation parameters: price elasticity of supply (β), spoilage rate, size of the prosperity ceiling, list of sinks.
9. Background event generator: on by default or not.

## 11. Next steps

1. Describe the mathematical model: variables, formulas, per-tick update order, the form of effects, stability loops (5.10).
2. Sketch an example world of 3 cities with numbers and check that the prices "feel" right.
3. Choose the world file format and pin down the schema.
4. A core prototype (Python or straight to Rust), then WASM and UI. The stability autotest is written together with the first prototype.

## 12. Appendix: observations from the Bannerlord economy

Source: the forum thread "Economic model: town, village, workshop, caravans, peasants" (rusmnb.ru, topic 23729), 2021, versions 1.5-1.7 beta. These are players' observations, not formulas from the code. The game has changed since, and some observations contradict each other (e.g. about the link between village wealth and supply volume). The real formulas are in the game code and have yet to be found.

**Supports our design**
- An unreachable route leads to shortage and rising prices; the cheapest goods are where it is easiest to deliver.
- Villages are bound to a town and give a steady base flow, on top of which peasant parties travel.
- A raid lowers a village's prosperity, and both the flow and the taxes fall: the event changes a parameter, the rest is computed on its own.
- A scripted "levelling" ships out the surplus of rich towns; you cannot build a rich town in a poor country.

**Ideas that went into the requirements (proposed)**
- Prosperity as a slow variable: it grows from the food stock (up to +3 per day) and goods on the market (about +2), with a ceiling via "housing cost" (above 6K a −1 penalty, above 9K −2, a bonus for poor towns).
- Food variety matters more than volume (1000 grain is worth less than 500 each of fish and grapes).
- Luxury (jewellery, velvet) is consumed by rich towns and loses money in poor ones.
- Village and town prices move together; buying in bulk moves the price on every unit.
- Caravan profit is noisy: from a loss to 1000+ per day, with an average around 250.
- A billeted army eats up the market; hauling in food by hand prolongs growth, but the slump afterwards is deeper.

**Warnings from the thread**
- The player's own production (a smithy) breaks the economy: player income must run into market capacity.
- Without major wars central cities run away, prices fall, the system breaks (the project author's observation). Hence section 5.10.
