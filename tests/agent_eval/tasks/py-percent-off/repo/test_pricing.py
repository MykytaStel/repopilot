import unittest

from pricing import with_tax


class WithTaxTest(unittest.TestCase):
    def test_adds_vat(self):
        self.assertEqual(with_tax(100, 0.2), 120.0)

    def test_rejects_a_negative_rate(self):
        with self.assertRaises(ValueError):
            with_tax(100, -0.1)


if __name__ == "__main__":
    unittest.main()
