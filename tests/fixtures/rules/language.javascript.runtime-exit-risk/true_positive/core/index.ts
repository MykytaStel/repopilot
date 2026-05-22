export function stopProcess(): never {
  process.exit(1);
}

export function failAtBoundary(): never {
  throw new Error("generic library boundary failure");
}
