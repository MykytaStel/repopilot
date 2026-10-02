import { test } from "node:test";
import assert from "node:assert/strict";
import { thumbnail, thumbnailName } from "../src/thumbnails.js";

// sharp is a native module; the test drives a stand-in with the same API.
function fakeSharp(calls) {
  return async () => ({
    default: () => {
      const pipeline = {
        resize: (options) => (calls.push(["resize", options]), pipeline),
        webp: (options) => (calls.push(["webp", options]), pipeline),
        toBuffer: async () => Buffer.from("webp"),
      };
      return pipeline;
    },
  });
}

test("names the thumbnail after the photo", () => {
  assert.equal(thumbnailName("mug.jpg"), "mug-320.webp");
});

test("produces a thumbnail", async () => {
  const calls = [];
  const output = await thumbnail(Buffer.from("photo"), fakeSharp(calls));
  assert.ok(output.length > 0);
  assert.deepEqual(calls.map(([step]) => step), ["resize", "webp"]);
});
