import { describe, expect, it } from "vitest";
import { total } from "./cart";

describe("cart total", () => {
  it("sums line items", () => {
    expect(total([{ price: 2, qty: 3 }])).toBe(6);
  });

  it("applies the discount", () => {
    expect(total([{ price: 10, qty: 1 }], 0.1)).toBe(9);
  });
});
