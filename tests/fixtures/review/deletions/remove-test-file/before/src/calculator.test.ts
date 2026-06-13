import { add } from "./calculator";

test("adds two numbers", () => {
  expect(add(2, 3)).toBe(5);
});
