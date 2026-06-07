#!/usr/bin/env node

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
const root = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-review-bench-"));
const results = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-review-results-"));

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    ...options,
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed (${result.status}): ${result.stderr}`,
    );
  }
}

function review(scope, output) {
  const started = process.hrtime.bigint();
  run(binary, [
    "review",
    ".",
    "--scope",
    scope,
    "--profile",
    scope === "changed" ? "default" : "strict",
    "--format",
    "json",
    "--output",
    output,
    "--no-progress",
  ]);
  return Number(process.hrtime.bigint() - started) / 1_000_000;
}

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor(sorted.length / 2)];
}

try {
  fs.mkdirSync(path.join(root, "src"));
  fs.writeFileSync(
    path.join(root, "package.json"),
    JSON.stringify({ name: "review-benchmark", private: true }, null, 2),
  );
  for (let index = 0; index < 320; index += 1) {
    const previous =
      index === 0 ? "" : `import { value${index - 1} } from "./module${index - 1}";\n`;
    const body = Array.from(
      { length: 70 },
      (_, line) => `  const local${line} = input + ${line};`,
    ).join("\n");
    const markers = Array.from(
      { length: 4 },
      (_, marker) => `// TODO: benchmark marker ${marker}`,
    ).join("\n");
    const exits = Array.from(
      { length: 20 },
      (_, exit) => `  if (input === ${exit}) process.exit(${exit});`,
    ).join("\n");
    fs.writeFileSync(
      path.join(root, "src", `module${index}.ts`),
      `${previous}${markers}\nexport const value${index} = ${index};\nexport function work${index}(input: number) {\n${body}\n${exits}\n  return input;\n}\n`,
    );
  }

  run("git", ["init", "-q"]);
  run("git", ["config", "user.email", "benchmark@repopilot.local"]);
  run("git", ["config", "user.name", "RepoPilot Benchmark"]);
  run("git", ["add", "."]);
  run("git", ["commit", "-qm", "benchmark baseline"]);
  fs.appendFileSync(
    path.join(root, "src", "module0.ts"),
    "\nexport const changed = fetch('/health');\n",
  );

  review("changed", path.join(results, "warm-changed.json"));
  review("full", path.join(results, "warm-full.json"));

  const changed = [];
  const full = [];
  for (let iteration = 0; iteration < 5; iteration += 1) {
    changed.push(review("changed", path.join(results, `changed-${iteration}.json`)));
    full.push(review("full", path.join(results, `full-${iteration}.json`)));
  }

  const changedMedian = median(changed);
  const fullMedian = median(full);
  const ratio = changedMedian / fullMedian;
  const changedReport = JSON.parse(
    fs.readFileSync(path.join(results, "changed-4.json"), "utf8"),
  );
  const reportFinalizationUs =
    changedReport.scan_timings?.report_finalization_us ?? Number.MAX_SAFE_INTEGER;
  console.log(
    JSON.stringify(
      {
        changed_median_ms: Number(changedMedian.toFixed(2)),
        full_median_ms: Number(fullMedian.toFixed(2)),
        changed_to_full_ratio: Number(ratio.toFixed(3)),
        required_max_ratio: 0.6,
        report_finalization_us: reportFinalizationUs,
        report_finalization_budget_us: 100_000,
      },
      null,
      2,
    ),
  );
  if (ratio > 0.6) {
    throw new Error(
      `changed review must be at least 40% faster (ratio ${ratio.toFixed(3)})`,
    );
  }
  if (reportFinalizationUs > 100_000) {
    throw new Error(
      `report finalization exceeded 100 ms (${reportFinalizationUs} us)`,
    );
  }
} finally {
  fs.rmSync(root, { recursive: true, force: true });
  fs.rmSync(results, { recursive: true, force: true });
}
