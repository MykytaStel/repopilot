import { describe, expect, it } from "vitest";
import { slugify } from "./slug";

describe("slugify", () => {
  it("lowercases and joins words", () => {
    expect(slugify("Hello World")).toBe("hello-world");
    expect(slugify("  A  B ")).toBe("a-b");
  });
});
