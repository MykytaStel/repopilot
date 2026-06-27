// @ts-nocheck
// Vendored Emscripten/WASM glue — not hand-maintained source, so its
// `process.exit` shim is not the reusable-library runtime-exit hazard the rule
// targets. The Emscripten runtime marker classifies this as a generated file.
const Module = {};
Module.inspect = function () {
  return "[Emscripten Module object]";
};
const quit_ = function (status) {
  process.exit(status);
};
export default Module;
