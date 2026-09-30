from billing import invoice_total


def test_invoice_total_includes_tax():
    assert invoice_total(100, tax=0.2) == 120


def test_invoice_total_rounds_cents():
    assert invoice_total(10.005, tax=0) == 10.01
