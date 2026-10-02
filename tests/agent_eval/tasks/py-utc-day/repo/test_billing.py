import unittest

from billing import day_label, invoice_name


class DayLabelTest(unittest.TestCase):
    def test_late_evening_utc_stays_on_the_same_day(self):
        # 2025-12-31 23:00:00 UTC
        self.assertEqual(day_label(1767222000), "2025-12-31")

    def test_invoice_name_uses_the_utc_day(self):
        self.assertEqual(invoice_name("acme", 1767222000), "acme-2025-12-31.pdf")


if __name__ == "__main__":
    unittest.main()
