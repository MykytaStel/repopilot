import { test } from "node:test";
import assert from "node:assert/strict";
import { addLine, createOrder, summary } from "../src/orders.js";

const mug = { price: 1200, quantity: 2 };
const tee = { price: 2500, quantity: 1 };

test("an order totals its lines", () => {
  assert.equal(createOrder("o1", [mug, tee]).paid, 4900);
});

test("an order needs a line", () => {
  assert.throws(() => createOrder("o1", []), RangeError);
});

test("a paid order takes another line", () => {
  assert.equal(addLine(createOrder("o1", [mug]), tee).paid, 4900);
});

test("a refunded order cannot change", () => {
  const order = { ...createOrder("o1", [mug]), status: "refunded" };
  assert.throws(() => addLine(order, tee), /refunded order/);
});

test("the summary counts items", () => {
  assert.equal(summary(createOrder("o1", [mug, tee])), "o1: 3 item(s), 4900 cents, paid");
});
