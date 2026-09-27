const fs = require("node:fs");
const path = require("node:path");

const workload = {
  base: "b2ec92e731f1035461e20871086f88aa04e315ce",
  head: "c6656898cd706b5e4759e6727c50f2fa2c6ef2ca",
  expectedFiles: 123,
  iterations: 5,
  budgetMs: 2_100,
};
const volatileReportFields = new Set([
  "cache_telemetry",
  "context_graph_cache",
  "review_timings",
  "scan_duration_us",
  "scan_timings",
]);

function stableJson(value) {
  if (Array.isArray(value)) return value.map(stableJson);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, stableJson(value[key])]),
    );
  }
  return value;
}

function semanticReport(value) {
  if (Array.isArray(value)) return value.map(semanticReport);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .filter(([key]) => !volatileReportFields.has(key))
        .map(([key, child]) => [key, semanticReport(child)]),
    );
  }
  return value;
}

function tieredDifference(left, right) {
  const identity = (signal) =>
    [signal.signal_id, signal.kind, signal.path, signal.line].join("\0");
  for (const group of ["definitely", "maybe", "noise"]) {
    const after = new Map(right[group].map((signal) => [identity(signal), signal]));
    for (const signal of left[group]) {
      const matching = after.get(identity(signal));
      if (!matching) return `${group}: missing ${identity(signal)}`;
      const changed = Object.keys(signal).filter(
        (key) => JSON.stringify(signal[key]) !== JSON.stringify(matching[key]),
      );
      if (changed.length) {
        const values = changed
          .map(
            (key) =>
              `${key} ${JSON.stringify(signal[key])} -> ${JSON.stringify(matching[key])}`,
          )
          .join(", ");
        return `${group}: ${identity(signal)} changed ${values}`;
      }
    }
  }
  return "tiered signal ordering changed";
}

function review(workspace, output, expectedPaths, binary, runAt) {
  const started = process.hrtime.bigint();
  runAt(workspace, binary, [
    "review",
    ".",
    "--base",
    workload.base,
    "--format",
    "json",
    "--output",
    output,
    "--no-progress",
  ]);
  const elapsedMs = Number(process.hrtime.bigint() - started) / 1_000_000;
  return validateReport(output, expectedPaths, elapsedMs);
}

function validateReport(output, expectedPaths, elapsedMs) {
  const report = JSON.parse(fs.readFileSync(output, "utf8"));
  if (
    !Array.isArray(report.changed_files) ||
    report.changed_files.length !== workload.expectedFiles
  ) {
    throw new Error(
      `large-diff review reported ${report.changed_files?.length ?? "no"} changed files, expected ${workload.expectedFiles}`,
    );
  }
  const reportedPaths = report.changed_files.map((file) => file.path).sort();
  if (JSON.stringify(reportedPaths) !== JSON.stringify([...expectedPaths].sort())) {
    throw new Error("large-diff review paths did not match the pinned Git diff");
  }
  if (report.change_proof?.coverage?.requested_files !== workload.expectedFiles) {
    throw new Error(
      `large-diff review requested ${report.change_proof?.coverage?.requested_files} files, expected ${workload.expectedFiles}`,
    );
  }
  const reviewSignalsUs = report.review_timings?.review_signals_us;
  if (!Number.isFinite(reviewSignalsUs)) {
    throw new Error("large-diff review did not report review_signals_us");
  }
  return semanticResult(report, elapsedMs, reviewSignalsUs);
}

function semanticResult(report, elapsedMs, reviewSignalsUs) {
  const semantic = stableJson(semanticReport(report));
  return { elapsedMs, reviewSignalsUs, semantic, signature: JSON.stringify(semantic) };
}

function prepareWorktree(repository, root, runAt, setWorktree) {
  const workspace = path.join(root, "large-diff-history");
  runAt(repository, "git", ["worktree", "add", "--detach", "--quiet", workspace, workload.head]);
  setWorktree(workspace);
  const expectedPaths = runAt(workspace, "git", [
    "diff",
    "--name-only",
    "-z",
    workload.base,
    workload.head,
  ])
    .split("\0")
    .filter(Boolean);
  if (expectedPaths.length !== workload.expectedFiles) {
    throw new Error(
      `pinned large-diff workload changed: expected ${workload.expectedFiles} files, found ${expectedPaths.length}`,
    );
  }
  return { workspace, expectedPaths };
}

function sampleReviews(workspace, results, expectedPaths, binary, runAt) {
  const warmup = review(
    workspace,
    path.join(results, "large-diff-warmup.json"),
    expectedPaths,
    binary,
    runAt,
  );
  const samples = [];
  for (let iteration = 0; iteration < workload.iterations; iteration += 1) {
    const sample = review(
      workspace,
      path.join(results, `large-diff-${iteration}.json`),
      expectedPaths,
      binary,
      runAt,
    );
    assertSemanticMatch(warmup, sample);
    samples.push(sample);
  }
  return samples;
}

function assertSemanticMatch(warmup, sample) {
  if (sample.signature === warmup.signature) return;
  const changed = Object.keys(warmup.semantic).filter(
    (key) => JSON.stringify(warmup.semantic[key]) !== JSON.stringify(sample.semantic[key]),
  );
  const details = changed.includes("tiered_signals")
    ? `; ${tieredDifference(warmup.semantic.tiered_signals, sample.semantic.tiered_signals)}`
    : "";
  throw new Error(
    `large-diff review output changed between warm-up and measured runs: ${changed.join(", ")}${details}`,
  );
}

function benchmarkReport(samples) {
  const median = (values) => {
    const sorted = [...values].sort((left, right) => left - right);
    return sorted[Math.floor(sorted.length / 2)];
  };
  return {
    changed_files: workload.expectedFiles,
    base: workload.base,
    head: workload.head,
    wall_median_ms: Number(median(samples.map((sample) => sample.elapsedMs)).toFixed(2)),
    review_signals_median_ms: Number(
      (median(samples.map((sample) => sample.reviewSignalsUs)) / 1000).toFixed(2),
    ),
    required_max_median_ms: workload.budgetMs,
  };
}

function createLargeDiffBenchmark({ binary, repository, root, results, runAt }) {
  let worktree;
  return {
    run() {
      const prepared = prepareWorktree(repository, root, runAt, (value) => {
        worktree = value;
      });
      const samples = sampleReviews(
        prepared.workspace,
        results,
        prepared.expectedPaths,
        binary,
        runAt,
      );
      return benchmarkReport(samples);
    },
    cleanup() {
      if (worktree) runAt(repository, "git", ["worktree", "remove", "--force", worktree]);
    },
  };
}

module.exports = { createLargeDiffBenchmark, semanticReport };
