// Reaches past @acme/b's public API into its internals — a boundary violation.
import { secret } from "../../b/src/internal";

export function run(): string {
  return secret();
}
