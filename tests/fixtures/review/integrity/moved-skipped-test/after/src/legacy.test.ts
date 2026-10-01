import { expect, it } from "vitest";
import { parse } from "./parser";

it("parses numbers", () => {
  expect(parse("1")).toBe(1);
});
