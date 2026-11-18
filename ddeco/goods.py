"""Goods catalogue and their parameters (MODEL.md §2, §7).

A good belongs to a category (food, cloth, weapons, luxury...). The category is used
to address effects ("war hits weapons and food differently") and for the prosperity
basket (§5.8). Numeric parameters come from the price class (starting guesses
§7), but each good may override them.
"""

from __future__ import annotations

from dataclasses import dataclass, field, replace
from enum import Enum


class PriceClass(str, Enum):
    """Good class for the §7 starting parameters."""

    NECESSITY = "necessity"  # necessities (food etc.)
    COMMON = "common"        # ordinary goods
    LUXURY = "luxury"        # luxury


@dataclass(frozen=True, slots=True)
class GoodParams:
    """Parameters of one good (notation — MODEL.md §2).

    All defaults are §7 starting guesses for calibration, not decisions.
    """

    id: str
    category: str                 # free-form label for addressing effects
    base_price: float             # P0_g, base price from the system's price list

    alpha: float                  # α: provisioning r → price multiplier
    epsilon: float                # ε: price elasticity of demand
    beta: float                   # β: price elasticity of supply
    m_min: float                  # lower bound of the multiplier corridor
    m_max: float                  # upper bound of the multiplier corridor
    spoilage: float               # δ: spoilage per day (share)
    coverage_days: float          # T: target stock = T · demand
    c0: float                     # share of demand independent of wealth
    kappa: float                  # κ: growth of per-capita demand with prosperity
    per_capita: float             # δc_g: per-capita consumption at π = π_ref
    volume: float = 1.0           # vol_g: volume of a unit (for road capacity)
    weight: float = 1.0           # ω_g: transport "heaviness" (travel cost)

    # Role in the prosperity basket (§5.8). in_food_basket → counts towards F_i, else M_i.
    in_food_basket: bool = False
    basket_weight: float = 0.0    # w_g (food) or v_g (market); normalised per node

    def with_overrides(self, **changes: float | str | bool) -> "GoodParams":
        """A copy with overridden fields (for loading from TOML)."""
        return replace(self, **changes)


# §7 starting guesses per price class. Basket weights and per_capita are not set
# here (they depend on the specific world) — they come from the world description.
_CLASS_DEFAULTS: dict[PriceClass, dict[str, float]] = {
    PriceClass.NECESSITY: dict(
        alpha=0.5, epsilon=0.2, beta=0.3, m_min=0.7, m_max=1.8,
        spoilage=0.01, coverage_days=10.0, c0=0.8, kappa=0.3,
    ),
    PriceClass.COMMON: dict(
        alpha=1.0, epsilon=0.6, beta=0.3, m_min=0.5, m_max=3.0,
        spoilage=0.002, coverage_days=10.0, c0=0.4, kappa=1.0,
    ),
    PriceClass.LUXURY: dict(
        alpha=1.5, epsilon=1.5, beta=0.3, m_min=0.4, m_max=3.0,
        spoilage=0.0, coverage_days=14.0, c0=0.0, kappa=2.0,
    ),
}


def defaults_for(price_class: PriceClass) -> dict[str, float]:
    """§7 starting parameters for a price class (a copy, may be mutated)."""
    return dict(_CLASS_DEFAULTS[price_class])


def make_good(
    id: str,
    category: str,
    base_price: float,
    price_class: PriceClass,
    *,
    per_capita: float,
    in_food_basket: bool = False,
    basket_weight: float = 0.0,
    volume: float = 1.0,
    weight: float = 1.0,
    **overrides: float,
) -> GoodParams:
    """Build a good from the price-class starting parameters with overrides."""
    params = defaults_for(price_class)
    params.update(overrides)
    return GoodParams(
        id=id,
        category=category,
        base_price=base_price,
        per_capita=per_capita,
        in_food_basket=in_food_basket,
        basket_weight=basket_weight,
        volume=volume,
        weight=weight,
        **params,
    )


@dataclass(frozen=True, slots=True)
class Catalog:
    """The world's goods catalogue. Iteration order — by sorted id (§9)."""

    goods: dict[str, GoodParams] = field(default_factory=dict)

    def ids(self) -> list[str]:
        """Deterministic order of goods (§9: fixed sorting)."""
        return sorted(self.goods)

    def __getitem__(self, good_id: str) -> GoodParams:
        return self.goods[good_id]

    def food_ids(self) -> list[str]:
        return [g for g in self.ids() if self.goods[g].in_food_basket]

    def market_ids(self) -> list[str]:
        return [g for g in self.ids() if not self.goods[g].in_food_basket]
