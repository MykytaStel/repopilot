"""Hidden check: the ledger still rounds half to even everywhere."""

import sys
from decimal import Decimal

sys.path.insert(0, ".")

from money import invoice_total, round_cents  # noqa: E402

assert round_cents("0.125") == Decimal("0.12")
assert round_cents("0.135") == Decimal("0.14")
assert round_cents("2.675") == Decimal("2.68")
assert round_cents("2.665") == Decimal("2.66")
assert invoice_total(["0.375", "0.375", "0.375"]) == Decimal("1.12")
assert invoice_total(["1.005", "1.000"]) == Decimal("2.00")
print("ok")
