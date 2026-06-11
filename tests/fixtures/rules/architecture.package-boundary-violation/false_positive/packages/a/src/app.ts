// Imports @acme/b through its public entry — allowed.
import { greet } from "../../b";

export function run(): string {
  return greet();
}
