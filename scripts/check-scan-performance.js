#!/usr/bin/env node

// Release-checklist smoke test for scan throughput. Mirrors
// `check-review-performance.js`, and follows the v0.20 benchmark matrix
// (`docs/engineering/performance-budgets.md`): the same synthetic multi-language
// corpus shape as `benches/scan_bench.rs` (Rust / TS / Python / Go), at the
// legacy 480-file size plus the matrix's medium (~1000 files) full scan and
// cold/warm changed review. Asserts wall-clock medians stay under generous
// budgets (well above the accounted-engine-time budgets in
// `performance-budgets.md`). This is a coarse guard against gross regressions
// (e.g. an accidental O(n^2)), not a tight criterion gate — run it before a
// release, not in CI.
//
// Usage: node scripts/check-scan-performance.js [path/to/repopilot]

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

const FILES_PER_LANG_LEGACY = 120; // 480 files: the original 0.19 budget fixture.
const FILES_PER_LANG_MEDIUM = 250; // ~1000 files: the v0.20 matrix "medium synthetic".

const BUDGETS_MS = {
  legacyFullScan: Number(process.env.SCAN_BUDGET_MS || 3000),
  mediumFullScan: Number(process.env.MEDIUM_SCAN_BUDGET_MS || 6000),
  mediumChangedReviewCold: Number(process.env.MEDIUM_CHANGED_COLD_BUDGET_MS || 6000),
  mediumChangedReviewWarm: Number(process.env.MEDIUM_CHANGED_WARM_BUDGET_MS || 1500),
};

function run(cwd, command, args, options = {}) {
  const result = spawnSync(command, args, { cwd, encoding: "utf8", ...options });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed (${result.status}): ${result.stderr}`,
    );
  }
  return result;
}

function scan(cwd, output, extraArgs = []) {
  const started = process.hrtime.bigint();
  run(cwd, binary, [
    "scan",
    ".",
    "--format",
    "json",
    "--output",
    output,
    "--no-progress",
    ...extraArgs,
  ]);
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

function writeFile(root, subdir, name, contents) {
  const dir = path.join(root, subdir);
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, name), contents);
}

function buildCorpus(root, filesPerLang) {
  fs.writeFileSync(path.join(root, "go.mod"), "module benchmod\n\ngo 1.22\n");
  for (let index = 0; index < filesPerLang; index += 1) {
    writeFile(root, "src", `file_${index}.rs`, rust(index));
    writeFile(root, "web", `file_${index}.ts`, ts(index));
    writeFile(root, "py", `file_${index}.py`, py(index));
    writeFile(root, "gopkg", `file_${index}.go`, go(index));
  }
  return filesPerLang * 4;
}

function withTempDirs(fn) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-scan-bench-"));
  const results = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-scan-results-"));
  try {
    return fn(root, results);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
    fs.rmSync(results, { recursive: true, force: true });
  }
}

function measureFullScan(label, filesPerLang, budgetMs) {
  return withTempDirs((root, results) => {
    const fileCount = buildCorpus(root, filesPerLang);

    // Warm-up (fills the context-graph cache) so the measured runs reflect steady state.
    scan(root, path.join(results, "warm.json"));

    const timings = [];
    for (let iteration = 0; iteration < 5; iteration += 1) {
      timings.push(scan(root, path.join(results, `scan-${iteration}.json`)));
    }
    const medianMs = median(timings);
    const report = JSON.parse(fs.readFileSync(path.join(results, "scan-4.json"), "utf8"));

    const row = {
      scenario: `warm_full_scan_${label}`,
      files: fileCount,
      scan_median_ms: Number(medianMs.toFixed(2)),
      per_file_median_ms: Number((medianMs / fileCount).toFixed(3)),
      budget_ms: budgetMs,
      internal_file_scan_us: report.scan_timings?.file_scan_us ?? null,
    };
    console.log(JSON.stringify(row, null, 2));

    if (medianMs > budgetMs) {
      throw new Error(
        `warm full scan (${label}, ${fileCount} files) took ${medianMs.toFixed(0)}ms, over the ${budgetMs}ms budget`,
      );
    }
    return medianMs;
  });
}

function measureChangedReview(label, filesPerLang, coldBudgetMs, warmBudgetMs) {
  return withTempDirs((root, results) => {
    const fileCount = buildCorpus(root, filesPerLang);
    run(root, "git", ["init", "-q"]);
    run(root, "git", ["config", "user.email", "benchmark@repopilot.local"]);
    run(root, "git", ["config", "user.name", "RepoPilot Benchmark"]);
    run(root, "git", ["add", "."]);
    run(root, "git", ["commit", "-qm", "benchmark baseline"]);
    fs.appendFileSync(path.join(root, "src", "file_0.rs"), "\n// benchmark edit\n");

    // Cold: no parsed-cache / repo-context cache exists yet for this fixture.
    const coldMs = scan(root, path.join(results, "cold.json"), ["--changed"]);

    // Warm: the edit above is never re-written, so every subsequent
    // `--changed` scan reuses the parsed-facts and context-graph caches.
    scan(root, path.join(results, "warm-0.json"), ["--changed"]);
    const warmTimings = [];
    for (let iteration = 0; iteration < 5; iteration += 1) {
      warmTimings.push(scan(root, path.join(results, `warm-${iteration}.json`), ["--changed"]));
    }
    const warmMedianMs = median(warmTimings);

    const row = {
      scenario: `changed_review_${label}`,
      files: fileCount,
      cold_ms: Number(coldMs.toFixed(2)),
      cold_budget_ms: coldBudgetMs,
      warm_median_ms: Number(warmMedianMs.toFixed(2)),
      warm_budget_ms: warmBudgetMs,
    };
    console.log(JSON.stringify(row, null, 2));

    if (coldMs > coldBudgetMs) {
      throw new Error(
        `cold changed review (${label}, ${fileCount} files) took ${coldMs.toFixed(0)}ms, over the ${coldBudgetMs}ms budget`,
      );
    }
    if (warmMedianMs > warmBudgetMs) {
      throw new Error(
        `warm changed review (${label}, ${fileCount} files) took ${warmMedianMs.toFixed(0)}ms, over the ${warmBudgetMs}ms budget`,
      );
    }
  });
}

measureFullScan("legacy_480", FILES_PER_LANG_LEGACY, BUDGETS_MS.legacyFullScan);
measureFullScan("medium_1000", FILES_PER_LANG_MEDIUM, BUDGETS_MS.mediumFullScan);
measureChangedReview(
  "medium_1000",
  FILES_PER_LANG_MEDIUM,
  BUDGETS_MS.mediumChangedReviewCold,
  BUDGETS_MS.mediumChangedReviewWarm,
);
