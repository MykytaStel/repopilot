// A recording stand-in for sharp: each pipeline step is kept in
// globalThis.sharpCalls, and toBuffer() returns a non-empty buffer.
export default function sharp(input) {
  const calls = [["input", input]];
  (globalThis.sharpCalls ??= []).push(calls);
  const pipeline = {
    resize: (options) => (calls.push(["resize", options]), pipeline),
    webp: (options) => (calls.push(["webp", options]), pipeline),
    toBuffer: async () => Buffer.from("webp"),
  };
  return pipeline;
}
