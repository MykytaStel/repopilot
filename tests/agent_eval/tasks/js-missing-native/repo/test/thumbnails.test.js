import { test } from "node:test";
import assert from "node:assert/strict";
import { thumbnail, thumbnailName } from "../src/thumbnails.js";

test("names the thumbnail after the photo", () => {
  assert.equal(thumbnailName("mug.jpg"), "mug-320.webp");
});

test("produces a thumbnail", async () => {
  const output = await thumbnail(Buffer.from("photo"));
  assert.ok(output.length > 0);
});
