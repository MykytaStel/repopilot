from orders import place_order


def test_place_order_charges_and_reserves_stock(store):
    order = place_order(store, sku="A1", qty=2)
    assert order.charged == 20
    assert store.stock("A1") == 8


def test_place_order_rejects_empty_cart(store):
    order = place_order(store, sku="A1", qty=0)
    assert order is None
