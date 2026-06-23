// A reusable module that terminates the host process — the genuine runtime-exit
// risk. (A `throw` is recoverable control flow and is intentionally not flagged.)
export function stopProcess(): never {
  process.exit(1);
}
