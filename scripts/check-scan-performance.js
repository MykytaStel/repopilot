#!/usr/bin/env node

// Release-checklist smoke test for scan throughput. Mirrors
// `check-review-performance.js`, but for `repopilot scan`: it builds the same
// synthetic 480-file corpus as `benches/scan_bench.rs` (120 files × Rust / TS /
// Python / Go) and asserts the warm wall-clock median stays under a generous
// budget (~3× the expected time). This is a coarse guard against gross
// regressions (e.g. an accidental O(n²)), not a tight criterion gate — run it
// before a release, not in CI.
//
// Usage: node scripts/check-scan-performance.js [path/to/repopilot] [budget_ms]

const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const binary = path.resolve(
  process.argv[2] ||
    path.join(
      __dirname,
      "..",
      "target",
      "release",
      process.platform === "win32" ? "repopilot.exe" : "repopilot",
    ),
);
const budgetMs = Number(process.argv[3] || process.env.SCAN_BUDGET_MS || 3000);

const FILES_PER_LANG = 120;

const root = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-scan-bench-"));
const results = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-scan-results-"));

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: root, encoding: "utf8", ...options });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed (${result.status}): ${result.stderr}`,
    );
  }
  return result;
}

function scan(output) {
  const started = process.hrtime.bigint();
  run(binary, ["scan", ".", "--format", "json", "--output", output, "--no-progress"]);
  return Number(process.hrtime.bigint() - started) / 1_000_000;
}

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor(sorted.length / 2)];
}

// Mirrors the per-language templates in benches/scan_bench.rs: a nested-loop
// function importing the previous file, so the import graph and AST audits are
// exercised at the same realistic small-repo scale.
function rust(index) {
  return `use crate::file_${Math.max(0, index - 1)};\n\npub fn compute_${index}(input: i64) -> i64 {\n    let mut total = 0_i64;\n    for outer in 0..input {\n        if outer % 2 == 0 {\n            for inner in 0..outer {\n                total += outer * inner;\n            }\n        }\n    }\n    total\n}\n`;
}
function ts(index) {
  return `import { compute } from "./file_${Math.max(0, index - 1)}";\n\nexport function run_${index}(input: number): number {\n    let total = 0;\n    for (let outer = 0; outer < input; outer++) {\n        if (outer % 2 === 0) {\n            for (let inner = 0; inner < outer; inner++) {\n                total += outer * inner;\n            }\n        }\n    }\n    return total + compute;\n}\n`;
}
function py(index) {
  return `from .file_${Math.max(0, index - 1)} import compute\n\n\ndef run_${index}(value):\n    total = 0\n    for outer in range(value):\n        if outer % 2 == 0:\n            for inner in range(outer):\n                total += outer * inner\n    return total + compute\n`;
}
function go(index) {
  return `package gopkg\n\nimport "fmt"\n\nfunc Run${index}(value int) int {\n    total := 0\n    for outer := 0; outer < value; outer++ {\n        if outer%2 == 0 {\n            for inner := 0; inner < outer; inner++ {\n                total += outer * inner\n            }\n        }\n    }\n    fmt.Sprintln(total)\n    return total\n}\n`;
}

function writeFile(subdir, name, contents) {
  const dir = path.join(root, subdir);
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, name), contents);
}

try {
  fs.writeFileSync(path.join(root, "go.mod"), "module benchmod\n\ngo 1.22\n");
  for (let index = 0; index < FILES_PER_LANG; index += 1) {
    writeFile("src", `file_${index}.rs`, rust(index));
    writeFile("web", `file_${index}.ts`, ts(index));
    writeFile("py", `file_${index}.py`, py(index));
    writeFile("gopkg", `file_${index}.go`, go(index));
  }
  const fileCount = FILES_PER_LANG * 4;

  // Warm-up (fills the context-graph cache) so the measured runs reflect steady state.
  scan(path.join(results, "warm.json"));

  const timings = [];
  for (let iteration = 0; iteration < 5; iteration += 1) {
    timings.push(scan(path.join(results, `scan-${iteration}.json`)));
  }
  const medianMs = median(timings);

  const report = JSON.parse(fs.readFileSync(path.join(results, "scan-4.json"), "utf8"));
  const fileScanUs = report.scan_timings?.file_scan_us ?? null;

  console.log(
    JSON.stringify(
      {
        files: fileCount,
        scan_median_ms: Number(medianMs.toFixed(2)),
        per_file_median_ms: Number((medianMs / fileCount).toFixed(3)),
        budget_ms: budgetMs,
        internal_file_scan_us: fileScanUs,
      },
      null,
      2,
    ),
  );

  if (medianMs > budgetMs) {
    throw new Error(
      `scan of ${fileCount} files took ${medianMs.toFixed(0)}ms, over the ${budgetMs}ms budget`,
    );
  }
} finally {
  fs.rmSync(root, { recursive: true, force: true });
  fs.rmSync(results, { recursive: true, force: true });
}
