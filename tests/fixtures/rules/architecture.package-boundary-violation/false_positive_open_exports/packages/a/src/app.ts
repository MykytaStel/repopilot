// Deep-imports @acme/b/src/internal, but @acme/b publishes `"exports": { "./*" }`,
// so every subpath is sanctioned public API — not a boundary violation.
import { secret } from "../../b/src/internal";

export function run(): string {
  return secret();
}
