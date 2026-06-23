// AST precision guard: the risky `process.exit(...)` token below appears only
// inside comments and string literals, so AST detection must NOT flag it (a real
// call would be flagged in this reusable module).

// Docs: call process.exit(1) only at the CLI entrypoint, never in a library.
export const usage = "throw new Error('boom') is discouraged in shared modules";

export function describe(): string {
  // throw new Error("this is commented out, not thrown");
  return "process.exit(1) only appears inside this string literal";
}
