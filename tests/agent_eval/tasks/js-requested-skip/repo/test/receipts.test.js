import { test } from "node:test";
import assert from "node:assert/strict";
import { formatReceipt, sendReceipt } from "../src/receipts.js";

const order = { id: "o7", total: 42, lines: [{ name: "Mug", quantity: 2 }] };

test("formats the receipt", () => {
  assert.equal(formatReceipt(order), "Receipt o7\nMug x2\nTotal: 42 EUR");
});

test("sends the receipt email", async () => {
  assert.equal(await sendReceipt(order, "buyer@example.com"), true);
});
