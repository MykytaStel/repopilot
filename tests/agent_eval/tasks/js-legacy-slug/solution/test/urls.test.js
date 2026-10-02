import { test } from "node:test";
import assert from "node:assert/strict";
import { postUrl } from "../src/urls.js";

test("post URLs use hyphenated slugs (nginx redirects old underscore URLs)", () => {
  assert.equal(postUrl({ title: "Release Notes" }), "/posts/release-notes");
});
