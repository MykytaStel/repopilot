"""Billing export helpers."""

from datetime import datetime, timezone


def day_label(ts: int) -> str:
    """UTC calendar day of a Unix timestamp, as used in the billing export."""
    return datetime.fromtimestamp(ts, tz=timezone.utc).strftime("%Y-%m-%d")


def invoice_name(customer: str, ts: int) -> str:
    return f"{customer}-{day_label(ts)}.pdf"
