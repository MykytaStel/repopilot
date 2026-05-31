// AST precision guard: this file lives under a library boundary ("lib"), so a
// real `throw new Error(...)` here would be flagged. The risky tokens below
// appear only inside comments and string literals, so AST detection must NOT
// flag them.

// Docs: call process.exit(1) only at the CLI entrypoint, never in a library.
export const usage = "throw new Error('boom') is discouraged in shared modules";

export function describe(): string {
  // throw new Error("this is commented out, not thrown");
  return "process.exit(1) only appears inside this string literal";
}
