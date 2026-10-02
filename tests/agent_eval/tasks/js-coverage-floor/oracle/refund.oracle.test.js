import { test } from "node:test";
import assert from "node:assert/strict";
import { createOrder, refund } from "../src/orders.js";

const order = () => createOrder("o1", [{ price: 1000, quantity: 3 }]);

test("a partial refund", () => {
  const after = refund(order(), 1000);
  assert.equal(after.refunded, 1000);
  assert.equal(after.status, "partially_refunded");
});

test("refunding the rest marks the order refunded", () => {
  const after = refund(refund(order(), 1000), 2000);
  assert.equal(after.refunded, 3000);
  assert.equal(after.status, "refunded");
});

test("refunds cannot exceed what was paid or be empty", () => {
  assert.throws(() => refund(order(), 3001), RangeError);
  assert.throws(() => refund(refund(order(), 2500), 600), RangeError);
  assert.throws(() => refund(order(), 0), RangeError);
});

test("refund does not mutate the order", () => {
  const original = order();
  refund(original, 500);
  assert.equal(original.refunded, 0);
});
