import { expect, it } from "vitest";
import { parse } from "./parser";

it("parses ISO dates", () => {
  expect(parse("2026-01-02")).toBeInstanceOf(Date);
});
