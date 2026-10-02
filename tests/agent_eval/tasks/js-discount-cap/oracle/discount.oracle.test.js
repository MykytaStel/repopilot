import { test } from "node:test";
import assert from "node:assert/strict";
import { applyDiscount } from "../src/discount.js";

test("discounts up to 50% apply as given", () => {
  assert.equal(applyDiscount(200, 20), 160);
  assert.equal(applyDiscount(200, 50), 100);
});

test("discounts above 50% are capped at 50%", () => {
  assert.equal(applyDiscount(200, 60), 100);
  assert.equal(applyDiscount(200, 150), 100);
});

test("negative discounts are still rejected", () => {
  assert.throws(() => applyDiscount(200, -5), RangeError);
});
