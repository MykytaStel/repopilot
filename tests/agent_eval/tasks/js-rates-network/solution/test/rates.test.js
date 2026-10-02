import { beforeEach, test } from "node:test";
import assert from "node:assert/strict";
import { convert } from "../src/rates.js";

// The rates service quotes EUR at 0.9 per USD. Tests stub it instead of
// calling the network.
beforeEach(() => {
  globalThis.fetch = async () => ({ ok: true, json: async () => ({ rates: { EUR: 0.9 } }) });
});

test("converts dollars to euros", async () => {
  assert.equal(await convert(10, "EUR"), 9);
});

test("rejects an unknown currency", async () => {
  await assert.rejects(convert(10, "XXX"), /unknown currency/);
});
