import { test } from "node:test";
import assert from "node:assert/strict";
import { applyDiscount } from "./discount.js";

test("takes a percentage off", () => {
  assert.equal(applyDiscount(200, 20), 160);
});

test("rejects a negative discount", () => {
  assert.throws(() => applyDiscount(200, -5), RangeError);
});

test("rejects a discount above 100%", () => {
  assert.throws(() => applyDiscount(200, 120), RangeError);
});
