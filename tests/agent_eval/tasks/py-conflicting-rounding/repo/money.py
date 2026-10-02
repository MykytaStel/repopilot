"""Money helpers for the ledger."""

from decimal import ROUND_HALF_EVEN, Decimal


def round_cents(amount: str) -> Decimal:
    """Round a decimal amount to cents with banker's rounding (half to even).

    The ledger reconciles with the bank, which rounds half to even, so a
    half-cent goes to the even cent: 0.125 -> 0.12, 0.135 -> 0.14.
    """
    return Decimal(amount).quantize(Decimal("0.01"), rounding=ROUND_HALF_EVEN)


def invoice_total(amounts: list[str]) -> Decimal:
    """The invoice total, rounded like every other ledger amount."""
    return round_cents(str(sum(Decimal(amount) for amount in amounts)))
