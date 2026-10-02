import { test } from "node:test";
import assert from "node:assert/strict";
import { createOrder, refund } from "../src/orders.js";

const order = () => createOrder("o1", [{ price: 1000, quantity: 3 }]);

test("a partial refund", () => {
  assert.equal(refund(order(), 1000).status, "partially_refunded");
});

test("a full refund", () => {
  assert.equal(refund(order(), 3000).status, "refunded");
});

test("an over-refund is rejected", () => {
  assert.throws(() => refund(order(), 3001), RangeError);
});
