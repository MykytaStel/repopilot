def process_order(cursor, order_id, items):
    cursor.execute(f"SELECT * FROM orders WHERE id = {order_id}")
    total = 0
    for item in items:
        if item["qty"] > 0:
            total += item["qty"] * item["price"]
    return total
