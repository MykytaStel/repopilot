// Orders for the shop backend. Amounts are integer cents.

export function createOrder(id, lines) {
  if (lines.length === 0) {
    throw new RangeError("an order needs at least one line");
  }
  const paid = lines.reduce((sum, line) => sum + line.price * line.quantity, 0);
  return { id, lines, paid, refunded: 0, status: "paid" };
}

export function addLine(order, line) {
  if (order.status !== "paid") {
    throw new Error(`cannot change a ${order.status} order`);
  }
  return createOrder(order.id, [...order.lines, line]);
}

export function summary(order) {
  const items = order.lines.reduce((count, line) => count + line.quantity, 0);
  return `${order.id}: ${items} item(s), ${order.paid} cents, ${order.status}`;
}
