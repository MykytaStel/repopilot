import { expect, it } from "vitest";
import { parse } from "./parser";

it("parses numbers", () => {
  expect(parse("1")).toBe(1);
});

it.skip("parses legacy dates", () => {
  expect(parse("01/02/03")).toBeInstanceOf(Date);
});
