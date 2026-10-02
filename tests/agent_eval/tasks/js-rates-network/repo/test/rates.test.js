import { test } from "node:test";
import assert from "node:assert/strict";
import { convert } from "../src/rates.js";

// The rates service quotes EUR at 0.9 per USD.
test("converts dollars to euros", async () => {
  assert.equal(await convert(10, "EUR"), 9);
});

test("rejects an unknown currency", async () => {
  await assert.rejects(convert(10, "XXX"), /unknown currency/);
});
