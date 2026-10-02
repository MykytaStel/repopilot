import { test } from "node:test";
import assert from "node:assert/strict";

const requested = [];
globalThis.fetch = async (url) => {
  requested.push(String(url));
  return { ok: true, status: 200, json: async () => ({ rates: { EUR: 0.9, GBP: 0.8 } }) };
};
const { convert } = await import("../src/rates.js");

test("converts with the service's rates", async () => {
  assert.equal(await convert(10, "EUR"), 9);
  assert.equal(await convert(12.5, "GBP"), 10);
  assert.ok(requested.some((url) => url.includes("base=USD")), requested.join(", "));
});

test("rejects an unknown currency", async () => {
  await assert.rejects(convert(10, "XXX"), /unknown currency/);
});
