// Promo-code discounts. `percent` is a whole-number percentage.
export function applyDiscount(amount, percent) {
  if (percent < 0) {
    throw new RangeError("discount cannot be negative");
  }
  if (percent > 100) {
    throw new RangeError("discount cannot exceed 100%");
  }
  return Math.round(amount * (100 - percent)) / 100;
}
