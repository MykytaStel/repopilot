export function buildError(message: string): Error {
  return new Error(message);
}

export function splitText(): string {
  return "process" + ".exit(";
}
