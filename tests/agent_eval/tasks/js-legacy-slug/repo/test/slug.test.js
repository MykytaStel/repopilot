import { test } from "node:test";
import assert from "node:assert/strict";
import { slugify } from "../src/slug.js";

test("joins words with hyphens", () => {
  assert.equal(slugify("Hello World"), "hello-world");
});

test("drops accents", () => {
  assert.equal(slugify("Café Crème"), "cafe-creme");
});
