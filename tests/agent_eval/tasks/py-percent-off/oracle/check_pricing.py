"""Hidden check for percent_off and the existing helper."""

import sys

sys.path.insert(0, ".")

from pricing import percent_off, with_tax  # noqa: E402

assert percent_off(200, 15) == 170.0
assert percent_off(19.99, 10) == 17.99
assert percent_off(50, 0) == 50
assert percent_off(50, 100) == 0
for bad in (-1, 101):
    try:
        percent_off(10, bad)
    except ValueError:
        pass
    else:
        raise AssertionError(f"percent_off accepted {bad}")
assert with_tax(100, 0.2) == 120.0
print("ok")
