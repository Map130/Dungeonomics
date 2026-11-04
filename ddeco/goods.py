"""Справочник товаров и их параметры (MODEL.md §2, §7).

Товар принадлежит категории (еда, ткани, оружие, роскошь...). Категория нужна
для адресации эффектов («война режет оружие и еду по-разному») и для корзины
процветания (§5.8). Числовые параметры берутся из прайс-класса (стартовые догадки
§7), но каждый товар может их переопределить.
"""

from __future__ import annotations

from dataclasses import dataclass, field, replace
from enum import Enum


class PriceClass(str, Enum):
    """Класс товара для стартовых параметров §7."""

    NECESSITY = "necessity"  # первая необходимость (еда и т.п.)
    COMMON = "common"        # обычные товары
    LUXURY = "luxury"        # роскошь


@dataclass(frozen=True, slots=True)
class GoodParams:
    """Параметры одного товара (обозначения — MODEL.md §2).

    Все значения по умолчанию — стартовые догадки §7 для калибровки, не решения.
    """

    id: str
    category: str                 # произвольная метка для адресации эффектов
    base_price: float             # P0_g, базовая цена из прайс-листа системы

    alpha: float                  # α: обеспеченность r → множитель цены
    epsilon: float                # ε: эластичность спроса по цене
    beta: float                   # β: эластичность предложения по цене
    m_min: float                  # нижняя граница коридора множителя
    m_max: float                  # верхняя граница коридора множителя
    spoilage: float               # δ: порча в день (доля)
    coverage_days: float          # T: целевой запас = T · спрос
    c0: float                     # доля спроса, не зависящая от богатства
    kappa: float                  # κ: рост душевого спроса с процветанием
    per_capita: float             # δc_g: душевое потребление при π = π_ref
    volume: float = 1.0           # vol_g: объём единицы (для ёмкости дорог)
    weight: float = 1.0           # ω_g: транспортная «тяжесть» (стоимость пути)

    # Роль в корзине процветания (§5.8). in_food_basket → вклад в F_i, иначе в M_i.
    in_food_basket: bool = False
    basket_weight: float = 0.0    # w_g (еда) или v_g (рынок); нормируется по узлу

    def with_overrides(self, **changes: float | str | bool) -> "GoodParams":
        """Копия с переопределёнными полями (для загрузки из TOML)."""
        return replace(self, **changes)


# Стартовые догадки §7 по прайс-классам. Корзинные веса и per_capita здесь не
# задаются (зависят от конкретного мира) — берутся из описания мира.
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
    """Стартовые параметры §7 для прайс-класса (копия, можно мутировать)."""
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
    """Собрать товар из стартовых параметров прайс-класса с переопределениями."""
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
    """Справочник товаров мира. Порядок обхода — по отсортированным id (§9)."""

    goods: dict[str, GoodParams] = field(default_factory=dict)

    def ids(self) -> list[str]:
        """Детерминированный порядок товаров (§9: фиксированная сортировка)."""
        return sorted(self.goods)

    def __getitem__(self, good_id: str) -> GoodParams:
        return self.goods[good_id]

    def food_ids(self) -> list[str]:
        return [g for g in self.ids() if self.goods[g].in_food_basket]

    def market_ids(self) -> list[str]:
        return [g for g in self.ids() if not self.goods[g].in_food_basket]
