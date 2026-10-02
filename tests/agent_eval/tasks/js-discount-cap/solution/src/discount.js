// Promo-code discounts. `percent` is a whole-number percentage.
export function applyDiscount(amount, percent) {
  if (percent < 0) {
    throw new RangeError("discount cannot be negative");
  }
  const applied = Math.min(percent, 50);
  return Math.round(amount * (100 - applied)) / 100;
}
