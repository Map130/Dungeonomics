# A living economy for tabletop RPGs: mathematical model v1

Status: draft. Complements `REQUIREMENTS.md`. All numeric defaults are starting guesses for calibration, not decisions.
What has been verified: the stability conditions of section 6 were checked by a numerical single-node run (see 6.3). Trade flows, prosperity and multi-node dynamics have since been implemented and are covered by the §8 autotests in the Python prototype and the Rust core.

## 1. Model principles

1. Discrete time, one-day step. Everything is computed synchronously: the new state is derived from the state at the start of the day.
2. **Parameters** (production, population, road difficulty, capacity) are changed by effects. The **state** (stocks, price multipliers, goods in transit, prosperity) evolves on its own.
3. A node's price depends only on that node's state. Nodes are coupled through trade flows (arbitrage), not through a general equilibrium.
4. Playability over realism: the price multiplier is clamped to a corridor; the spread and the price impact of purchases protect against buying out the market.

## 2. Notation

| Symbol | Meaning |
|---|---|
| i, j | nodes; e = (i→j) a directed arc (a road has two arcs) |
| g | a good; each good belongs to a category (food, cloth, weapons, luxury...) |
| P0_g | base price of the good (from the system's price list) |
| m_ig | price multiplier, **state**, smoothed; price P_ig = P0_g · m_ig |
| s_ig | stock at the node |
| s*_ig | target stock: s* = T_g · D_ig (T_g days of coverage) |
| r_ig | s_ig / s*_ig, provisioning |
| D_ig | base demand (before the price response), see 5.3 |
| p_ig | production per day, see 5.2 |
| π_i | node prosperity, [0, 1], **state** |
| N_i | population (demand scale) |
| L_e, k_e | base travel time (days) and road difficulty multiplier (effective value) |
| cap_e | arc throughput (volume units per day) |
| q_e,g,d | good g on arc e with d days left until arrival (pipeline) |
| Q_jg | total good g in transit to node j |

Good/category parameters: α (price elasticity with respect to provisioning), ε (price elasticity of demand), β (price elasticity of supply), corridor [m_min, m_max], δ (spoilage per day), T (days of coverage), ω (transport "heaviness"), vol (volume of a unit).

## 3. Effects

Each parametric effect k acts on one parameter x (node production, road k_e, N_i, cap_e, spread, etc.):

```
w_k(t) = decay weight in [0, 1]:
  step:        1 until the end of the duration, then 0
  linear:      max(0, 1 - (t - t0)/D)
  exponential: exp(-(t - t0)/τ)

x_eff = clamp( x_base · Π_k (1 + (mult_k - 1)·w_k)  +  Σ_k add_k·w_k ,  x_min, x_max )
```

The multiplier is interpolated towards 1 as it decays, so the end of an effect causes no jump, and two effects on one parameter combine independently. An effect stores its source and target (node, arc, good or category) for the reasons log.

**One-shot events** change the state once and are not part of the parameters: caravan robbery (remove a share of the cargo from the pipeline), GM stock edits, a prosperity hit (π ← π · (1 − ρ)).

## 4. Per-day update order

```
0. Events: background shocks (5.11) create effects; the GM adds effects and one-shot events
1. Recompute effective parameters (section 3), expired effects are removed
2. Arrivals: pipelines shift by a day, cargo with d = 0 goes into the destination node's stock
3. Production (5.2)
4. Consumption (5.3)
5. Spoilage (5.4)
6. Trade dispatch (5.6): stock leaves the sending node into the pipeline
7. Prosperity (5.8)
8. Price (5.5)
9. Caravan materialisation (5.10)
10. Reasons log (5.12)
```

All arcs and nodes are processed from a synchronous snapshot of the state at the start of the respective step, so the traversal order does not affect the result.

## 5. Formulas

### 5.1 Target stock
```
s*_ig = T_g · D_ig
```
If an effect raised demand (a billeted army), s* grows, r falls, the price rises. That is how a consumption shock turns into rising prices without special rules.

### 5.2 Production (with a supply response to price)
```
p_ig = Pcap_ig · clamp( m_ig^β_g , lo_p , hi_p )          lo_p ≈ 0.25, hi_p ≈ 2
```
Pcap is the node's capacity for the good (after effects: a murdered craftsman gives Pcap = 0). When the price is low for long, the producer cuts output; when it is high, they ramp it up. This is the main closing loop against runaway growth (requirement 5.10).

In v1 production **does not depend on prosperity**. Such a dependency would create a positive loop (π grows, production grows, π grows), and then consumption would have to grow with π no slower than production (see 5.8).

### 5.3 Demand and consumption
```
D_ig = N_i · δc_g · c_g(π_i) + Garrison_ig                 base demand per day
c_g(π) = c0_g + (1 - c0_g) · (π/π_ref)^κ_g                 dependence on wealth
d_ig   = min( D_ig · m_ig^(-ε_g) ,  s_ig )                  actual consumption
```
- δc_g: per-capita consumption; c0_g the share of demand independent of wealth (food: about 0.8, luxury: 0); κ_g the growth exponent (luxury about 2); π_ref = 0.5.
- Garrison_ig and a billeted army are additions to D (effects or node parameters), not a separate mechanic.
- If d < D · m^(-ε), a shortage is recorded (needed for prosperity and the log).

### 5.4 Spoilage
```
s_ig ← s_ig · (1 - δ_g)
```

### 5.5 Price (multiplier)
```
r_ig  = s_ig / s*_ig
m*_ig = clamp( r_ig^(-α_g) , m_min_g , m_max_g )           target multiplier
m_ig ← m_ig + λ · ( m*_ig - m_ig )                         smoothing, λ ∈ (0, 1]
```
Price for the player: buying P · (1 + σ), selling P · (1 - σ), where σ is the node's spread (an effect target). Background traders work without the spread.

### 5.6 Trade flows (arbitrage)
The decision to ship is taken on the **expected** multiplier at the destination, accounting for goods in transit (otherwise traders ship every day while the first batch is on the road, and we get overflow and oscillations):
```
m̂_ig = clamp( ((s_ig + Q_ig) / s*_ig)^(-α_g) , m_min_g , m_max_g )
γ_e   = c_γ · L_e · k_e                                    share of the base price spent on transport
Δ_eg  = P0_g · ( m̂_jg - m̂_ig - γ_e · ω_g )                gain from shipping, e = (i→j)
```
If Δ ≤ 0 the flow is zero (in equilibrium the price difference does not exceed the travel cost). Otherwise:
```
A_ig   = max(0, s_ig - ρ_res · s*_ig)                      available for export, ρ_res ≈ 0.5
u_eg   = κ_t · clamp(Δ_eg / P0_g, 0, 1) · A_ig             desired flow
ψ_ig   = min(1, A_ig / Σ_e∈out(i) u_eg)                    so as not to ship more than there is
φ_e    = min(1, cap_e / Σ_g vol_g · ψ_ig · u_eg)           road capacity limit
f_eg   = φ_e · ψ_ig · u_eg
```
The sender's stock decreases immediately; f_eg is put into the pipeline `q_e,g,τ_e` with delay `τ_e = max(1, ceil(L_e · k_e))`. κ_t ≈ 0.3 is the share of the available stock shipped per day.

**Mandatory hub delivery** (an option for villages, the analogue of Bannerlord's constant deliveries): a village sends a share η_v ≈ 0.6 of its production to its bound city regardless of the price difference; arbitrage works on the remainder. The tight coupling of village and city prices comes from arbitrage itself: the difference cannot exceed the travel cost.

Hub delivery is not limited by cap_e (D7); changing that is *(proposed)*.

### 5.7 Price impact of a player purchase
A purchase of q units with stock s and current multiplier m. The marginal multiplier after u units is `m·(s/(s-u))^α`; the average price for q units:
```
α ≠ 1:  P_avg = P0 · m · s^α · ( s^(1-α) - (s-q)^(1-α) ) / ( (1-α) · q )
α = 1:  P_avg = P0 · m · s · ln( s / (s-q) ) / q
```
For a sale the stock grows and the integral is taken over `(s+u)^(-α)`. The [m_min, m_max] corridor applies at every point, so this is implemented by numerical summation over steps rather than a closed form. After the deal m is set to the marginal multiplier at the last unit, then the usual smoothing takes over. You cannot buy the whole stock at the price tag, and the bigger the batch, the pricier its tail. Condition: q < s.

### 5.8 Prosperity
```
F_i  = Σ_g∈Food   w_g · min(r_ig, 1)                       w_g: basket shares, Σ w_g = 1
M_i  = Σ_g∈Market v_g · min(r_ig, 1)                       other categories, Σ v_g = 1
π*_i = clamp( w_F · F_i + w_M · M_i , 0 , 1 )              w_F ≈ 0.6, w_M ≈ 0.4
π_i ← π_i + μ · ( π*_i - π_i )                             μ ≈ 0.03
```
- The basket: a shortage of one kind of food is not offset by a surplus of another (cap at min(r, 1)), so variety counts.
- Natural ceiling: the higher π, the higher D (formula 5.3), the lower r, so π levels off where consumption eats up the production surplus. A separate "housing" ceiling as in Bannerlord is not needed, but a `(1 - π^n)` factor on growth can be added.
- A food shortage lowers F and hence π (and later loyalty, if it appears).
- One-shot hits (raids) on π are set by an event.

### 5.9 Pipeline and losses
An entry in `q_e,g,d` is a batch. Arrival is in section 4, step 2. Caravan robbery: `q ← q · (1 - ρ_loot)` for the chosen batch. The loot either vanishes or goes to the players at the GM's decision.

### 5.10 Caravan materialisation
For arc e we accumulate the export value since the last materialisation: `V_e = Σ_g P0_g · f_eg`. When `V_e ≥ θ_car`, a "named caravan" event is created: the cargo is the batches currently in this arc's pipeline, the guard strength is drawn from a distribution that depends on road danger (k_e), and the GM feed gets a warning 1-3 days before arrival. Robbery applies 5.9. A caravan is the face of a flow already under way, not a source of new goods.

### 5.11 Background shocks
For each shock template (blight, epidemic, raid, bandits on the road) and each target: an event on a day with probability `1 - exp(-rate)`, rate of order 1/120 per day. A shock creates an effect from the template. The GM can turn the generator off. The RNG must be counter-based: a value is derived from `(seed, day, target id, template id)`, not from a call sequence. Then undoing and replaying a day gives the same result.

### 5.12 Reasons log
The cause of a price change is traced along the chain: price ← provisioning r ← {effective production, effective demand, inbound and outbound flows, spoilage} ← {active effects, road difficulty}. At each step the log keeps the effective parameters and their ratio to the base values, flows that deviate from yesterday, and the list of effects whose decay weight is not 0. The feed shows the three main causes by contribution and the source of each effect. If the contribution of causes is needed numerically, it is computed counterfactually (a step with the effect disabled; the graph is small).

The cause format is structured (D5): the core does not produce text. The verdict is a code `shortage | surplus | normal` (r < 0.9 / r > 1.1 / otherwise). A cause is a `kind` plus parameters: `node_effect` (an effect on a node parameter: `param`, `op`, `value` — the current multiplier or addend, `source`), `edge_effect` (an effect on an inbound road: the same plus `src`, `dst`), `stock` (no active effects, `value` = r). The UI builds the text and its translation (EN/RU).

### 5.13 Production chains (extension, v1.1 — D3)
For a good h with inputs `a_gh` (units of g per unit of h):
```
p_ih = min( Pcap_ih · m_ih^β , min_g s_ig / a_gh )
```
the consumption of inputs `a_gh · p_ih` is added to demand D_ig. Then a grain shortage raises the price of flour and bread along the chain. Deferred to v1.1 (D3).

### 5.14 Initial state and warm-up
If the world sets `Pcap = D` for every good, it starts in equilibrium: `s = s*`, `m = 1`, π = π*. If not, the no-trade equilibrium multiplier solves `m^(β+ε) = D/Pcap`, and the starting state is taken from it (auto-calibration). Warming up the world is a run without events until convergence.

## 6. Stability

### 6.1 The price–stock loop (one node, one good, no trade)
Linearise around the equilibrium `Pcap = D`. Let
```
a = α · (β + ε) / T
```
The step order of section 4 (the stock is updated, then the price is computed from the new stock) gives a transition matrix with determinant `1 - λ` and trace `2 - λ·(1 + a)`. Consequences:

- **Stability:** `λ < 4 / (2 + a)`. With `a < 2`, i.e. α(β+ε) < 2T, any λ ≤ 1 is stable.
- **No oscillations (monotone convergence):** `λ ≥ 4a / (1 + a)²`.
- **For smaller λ** the oscillations decay, the amplitude shrinks by a factor of √(1-λ) per day.
- If `a` is large (small T, steep α and ε), the price overshoots the stock; this is what to watch for when tuning goods parameters.

### 6.2 What the formulas do not cover
- The price corridor and the `d ≤ s` threshold (non-linearity). Near the corridor edges the dynamics may stick; hence the autotest (section 8).
- Trade flows and transit delays add delayed loops; "overflow–shortage" oscillations are possible. Including goods in transit in m̂ (5.6) dampens them, but this is not proven.
- Prosperity: a slow variable that must be closed by negative feedback (see 5.8).

### 6.3 Numerical check (one node, a non-linear 1000-day run, start s = 0.8·s*, λ = 0.3)

| Category | α | β | ε | T | a | Spectral radius | Behaviour |
|---|---|---|---|---|---|---|---|
| Necessities | 0.5 | 0.3 | 0.2 | 10 | 0.025 | 0.973 | monotone, slow (about 100 days to 1%) |
| Common | 1.0 | 0.3 | 0.6 | 10 | 0.090 | 0.837 | damped oscillations |
| Luxury | 1.5 | 0.3 | 1.5 | 14 | 0.193 | 0.837 | damped oscillations |
| Stress test | 2.0 | 0.5 | 1.5 | 2 | 2.000 | 1.000 (λ = 1) | undamped oscillations (stability edge) |
| Same with λ = 0.5 | 2.0 | 0.5 | 1.5 | 2 | 2.000 | 0.707 | damped oscillations |

The analytical determinant (`1 - λ`), the oscillation region and the stability edge matched the run. Consequence for play: for necessities the return to normal after a shock takes months, a food shortage "hangs" for a long time. If that is too slow, increase λ or reduce T.

## 7. Default parameters (starting guesses)

| Parameter | Necessities | Common | Luxury |
|---|---|---|---|
| α (provisioning → price) | 0.5 | 1.0 | 1.5 |
| ε (demand ← price) | 0.2 | 0.6 | 1.5 |
| β (supply ← price) | 0.3 | 0.3 | 0.3 |
| Multiplier corridor | 0.7 … 1.8 | 0.5 … 3.0 | 0.4 … 3.0 |
| T, days of coverage | 10 | 10 | 14 |
| Spoilage δ per day | 0.01 | 0.002 | 0 |
| c0 (share of demand independent of wealth) | 0.8 | 0.4 | 0 |
| κ (demand growth with π) | 0.3 | 1 | 2 |

Global: λ = 0.3, spread σ = 0.1, κ_t = 0.3, ρ_res = 0.5, μ = 0.03, η_v = 0.6; pick c_γ so that a 3-day road costs 5-10% of a light good's price.

## 8. Stability autotest

1. **Single node**: a 1000-day run for each category of table 7 with a ±20% starting stock perturbation. Check: |s/s* - 1| < 0.01 by day 300, the multiplier is not at the corridor edge.
2. **Multi-node**: a world of 3-4 cities, 15 nodes, no events, ±20% perturbation of stocks and production. Check: after 300 days the maximum |m - 1| < 0.05, stocks within [0.5, 2]·s*, the slope of `log s` over the last 300 days below 1e-4 per day in absolute value, road flows do not oscillate (amplitude below 5% of the mean).
3. **Runaway**: a world with excess production in the central city (Pcap = 1.5·D), 2000 days. Check: π and stocks reach a ceiling rather than growing forever, the multiplier does not stick to the edge (per 5.2 production must fall).
4. **Determinism**: two runs with one seed match bit for bit; undoing N days and replaying gives the same result.

The thresholds above are starting values for calibration.

## 9. Implementation requirements

- The traversal order of nodes and arcs is fixed (sorted by id); parallel floating-point reductions only with a fixed summation order.
- `pow`, `exp`, `ln` give different last bits on different platforms and in WASM, so cross-build reproducibility needs a single implementation (e.g. the `libm` library or a custom one) or recording reference runs "with tolerance" instead of bitwise equality.
- The RNG is counter-based (see 5.11).
- A reference prototype can be written in another language and used to check the main implementation: a shared set of "golden" runs with a tolerance.

## 10. Open questions of the model

1. Units of goods and volume (pieces, weight, "batches") and the values of vol_g, ω_g.
2. ~~Whether to include production chains (5.13) in v1.~~ Decided: no, v1.1 (`DECISIONS.md` D3).
3. Dependence of production on prosperity (removed in v1 because of the positive loop).
4. Whether a hard "housing" ceiling is needed on top of the natural one (5.8).
5. Money model: v1 does not model money (goods only); taxes are implemented as a share of production or garrison demand.
6. Calibration of base prices and player income for the chosen system (D&D 5e: SRD 5.1 prices, D5).
7. Default decay shape of effects for different event types.
8. Distribution of caravan guard strength and the materialisation threshold θ_car.
