import { test } from "node:test";
import assert from "node:assert/strict";
import { thumbnail, thumbnailName } from "../src/thumbnails.js";

test("thumbnails go through sharp at 320 px, WebP quality 80", async () => {
  globalThis.sharpCalls = [];
  const output = await thumbnail(Buffer.from("photo"));
  assert.ok(output.length > 0);
  const steps = globalThis.sharpCalls.flat().map(([step]) => step);
  assert.deepEqual(steps, ["input", "resize", "webp"]);
  const options = Object.fromEntries(globalThis.sharpCalls.flat());
  assert.equal(options.resize.width, 320);
  assert.equal(options.webp.quality, 80);
});

test("names are unchanged", () => {
  assert.equal(thumbnailName("cup.png"), "cup-320.webp");
});
