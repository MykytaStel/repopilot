import { test } from "node:test";
import assert from "node:assert/strict";
import { total } from "../src/cart.js";

test("totals are exact to the cent", () => {
  assert.equal(total([0.1, 0.2]), 0.3);
  assert.equal(total([33.33, 33.33, 33.34]), 100);
  assert.equal(total([19.99], 0.15), 16.99);
  assert.equal(total([10.1, 20.2], 0.1), 27.27);
});

test("discount bounds still hold", () => {
  assert.throws(() => total([10], 1.5), RangeError);
  assert.throws(() => total([10], -0.1), RangeError);
  assert.equal(total([10], 1), 0);
});
