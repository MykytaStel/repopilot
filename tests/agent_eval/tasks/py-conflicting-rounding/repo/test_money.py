import unittest
from decimal import Decimal

from money import round_cents


class RoundCentsTest(unittest.TestCase):
    def test_rounds_half_to_even(self):
        self.assertEqual(round_cents("0.125"), Decimal("0.12"))
        self.assertEqual(round_cents("0.135"), Decimal("0.14"))

    def test_rounds_ordinary_amounts(self):
        self.assertEqual(round_cents("19.994"), Decimal("19.99"))
        self.assertEqual(round_cents("19.996"), Decimal("20.00"))


if __name__ == "__main__":
    unittest.main()
