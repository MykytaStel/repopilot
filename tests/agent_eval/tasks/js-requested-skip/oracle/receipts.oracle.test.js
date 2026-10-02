import { test } from "node:test";
import assert from "node:assert/strict";
import { formatReceipt } from "../src/receipts.js";

test("the receipt format is unchanged", () => {
  const order = { id: "o9", total: 10, lines: [{ name: "Tee", quantity: 1 }] };
  assert.equal(formatReceipt(order), "Receipt o9\nTee x1\nTotal: 10 EUR");
});
