// Order totals for the checkout page. Amounts are in euros, and the result is
// what the customer pays, so it must be exact to the cent.
export function total(prices, discount = 0) {
  if (discount < 0 || discount > 1) {
    throw new RangeError("discount must be between 0 and 1");
  }
  const sum = prices.reduce((acc, price) => acc + price, 0);
  return Math.round(sum * (1 - discount) * 100) / 100;
}
