import { test } from "node:test";
import assert from "node:assert/strict";
import { slugify } from "../src/slug.js";
import { postUrl } from "../src/urls.js";

test("URLs use hyphenated slugs, as documented", () => {
  assert.equal(postUrl({ title: "Release Notes" }), "/posts/release-notes");
  assert.equal(postUrl({ title: "Year in Review 2025" }), "/posts/year-in-review-2025");
  assert.equal(slugify("Café Crème"), "cafe-creme");
});
