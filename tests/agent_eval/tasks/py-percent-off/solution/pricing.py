"""Price helpers for the storefront. Prices are in euros."""


def with_tax(price: float, rate: float) -> float:
    """The price with VAT at `rate` (0.2 for 20%), rounded to cents."""
    if rate < 0:
        raise ValueError("tax rate cannot be negative")
    return round(price * (1 + rate), 2)


def percent_off(price: float, pct: float) -> float:
    """The price minus `pct` percent, rounded to cents."""
    if not 0 <= pct <= 100:
        raise ValueError("pct must be between 0 and 100")
    return round(price * (100 - pct) / 100, 2)
