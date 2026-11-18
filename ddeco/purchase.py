"""Effect of a player's purchase/sale on the price (MODEL.md §5.7).

A large deal moves the price on every unit; the player pays the average along the curve, not
the price tag — protection against buying out the market. The marginal multiplier after u units:
m·(s/(s−u))^α for a purchase. The [m_min, m_max] corridor applies at every point,
so we sum numerically over steps rather than use a closed form. After
the deal m is set to the marginal multiplier at the last unit, then
the usual smoothing takes over.
"""

from __future__ import annotations

from dataclasses import dataclass

from . import formulas as F
from .state import SimState
from .world import World


@dataclass(frozen=True, slots=True)
class Trade:
    good: str
    qty: float
    total: float       # total price of the deal (spread included)
    avg_price: float   # average price per unit
    new_multiplier: float


def _price_multiplier(m0: float, s0: float, u: float, alpha: float, m_min: float, m_max: float) -> float:
    """Marginal multiplier after u units taken out (u>0 purchase, u<0 sale)."""
    denom = s0 - u
    if denom <= 0.0:
        return m_max
    return F.clamp(m0 * (s0 / denom) ** alpha, m_min, m_max)


def quote(world: World, state: SimState, node: str, good: str, qty: float,
          *, sell: bool = False, steps: int = 256) -> Trade:
    """Compute a deal WITHOUT changing the state (price preview for the GM)."""
    if qty <= 0.0:
        raise ValueError("qty must be > 0")
    g = world.catalog[good]
    s0 = state.stock[(node, good)]
    m0 = state.multiplier[(node, good)]
    spread = world.nodes[node].spread
    sign = -1.0 if sell else 1.0   # a purchase takes out (u>0), a sale adds (u<0)
    if not sell and qty >= s0:
        raise ValueError("cannot buy the whole stock (q < s)")

    du = qty / steps
    total = 0.0
    for i in range(steps):
        u = sign * (i + 0.5) * du                      # middle of the step
        m = _price_multiplier(m0, s0, u, g.alpha, g.m_min, g.m_max)
        unit = g.base_price * m * (1.0 - spread if sell else 1.0 + spread)
        total += unit * du
    new_m = _price_multiplier(m0, s0, sign * qty, g.alpha, g.m_min, g.m_max)
    return Trade(good=good, qty=qty, total=total, avg_price=total / qty, new_multiplier=new_m)


def apply(world: World, state: SimState, node: str, good: str, qty: float,
          *, sell: bool = False, steps: int = 256) -> Trade:
    """Execute a deal: update the stock and the multiplier. Returns the deal details."""
    trade = quote(world, state, node, good, qty, sell=sell, steps=steps)
    delta = qty if sell else -qty
    state.stock[(node, good)] = max(0.0, state.stock[(node, good)] + delta)
    state.multiplier[(node, good)] = trade.new_multiplier
    return trade
