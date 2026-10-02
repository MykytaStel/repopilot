"""Price helpers for the storefront. Prices are in euros."""


def with_tax(price: float, rate: float) -> float:
    """The price with VAT at `rate` (0.2 for 20%), rounded to cents."""
    if rate < 0:
        raise ValueError("tax rate cannot be negative")
    return round(price * (1 + rate), 2)
