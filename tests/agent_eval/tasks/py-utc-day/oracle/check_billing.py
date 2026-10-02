"""Hidden check: the export uses the UTC day in any local time zone."""

import sys

sys.path.insert(0, ".")

from billing import day_label, invoice_name  # noqa: E402

assert day_label(1767222000) == "2025-12-31", day_label(1767222000)
assert day_label(1767225600) == "2026-01-01", day_label(1767225600)
assert invoice_name("acme", 1767222000) == "acme-2025-12-31.pdf"
print("ok")
