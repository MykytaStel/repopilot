import unittest
from decimal import Decimal

from money import invoice_total


class InvoiceTotalTest(unittest.TestCase):
    def test_sums_lines(self):
        self.assertEqual(invoice_total(["10.00", "2.50"]), Decimal("12.50"))

    def test_rounds_the_total(self):
        # 3 x 0.375 = 1.125
        self.assertEqual(invoice_total(["0.375", "0.375", "0.375"]), Decimal("1.13"))


if __name__ == "__main__":
    unittest.main()
