import { describe, expect, it } from "vitest";
import { slugify } from "./slug";

describe("slugify", () => {
  it("lowercases and joins words", () => {
    const cases = [["Hello World", "hello-world"]];
    expect(slugify(cases[0][0])).toEqual(cases[0][1]);
    expect(slugify("  A  B ")).toEqual("a-b");
  });
});
