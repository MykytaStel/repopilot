import { test } from "node:test";
import assert from "node:assert/strict";
import { total } from "./cart.js";

test("adds line items", () => {
  assert.equal(total([0.1, 0.2]), 0.3);
});

test("applies a percentage discount", () => {
  assert.equal(total([19.99], 0.15), 16.99);
});

test("rejects a discount above 100%", () => {
  assert.throws(() => total([10], 1.5), RangeError);
});
