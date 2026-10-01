const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

// A 20-file change: 10 test files that skip, rename, and thin out tests, and
// the 10 sources they cover. Budgets: RP24-023 (median under 1 s) and the
// Phase C limit that integrity analysis stays within 10% of the review.
const workload = {
  sources: 10,
  testsPerFile: 40,
  iterations: 7,
  budgetMs: 1_000,
  maxIntegrityShare: 0.1,
};

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor(sorted.length / 2)];
}

function testFile(index, testsPerFile) {
  const cases = Array.from({ length: testsPerFile }, (_, test) =>
    [
      `  it("handles case ${test}", () => {`,
      `    const result = price${index}(${test}, ${test % 7});`,
      `    expect(result).toBeGreaterThanOrEqual(0);`,
      `    expectRounded(result, ${test});`,
      `    expect(price${index}(${test}, 0)).toBe(${test});`,
      "  });",
    ].join("\n"),
  ).join("\n");
  return [
    `import { price${index} } from "./price${index}";`,
    "",
    "function expectRounded(value: number, base: number) {",
    "  expect(Math.round(value)).toBe(value);",
    "  expect(value).toBeLessThanOrEqual(base * 2 + 7);",
    "}",
    "",
    `describe("price${index}", () => {`,
    cases,
    "});",
    "",
  ].join("\n");
}

function sourceFile(index) {
  return `export function price${index}(base: number, extra: number): number {\n  return base + extra;\n}\n`;
}

/** Skips one test, renames one, and drops an assertion from a third. */
function weaken(text) {
  return text
    .replace('it("handles case 3"', 'it.skip("handles case 3"')
    .replace('it("handles case 5"', 'it("handles case five"')
    .replace(/(it\("handles case 8"[^\n]*\n[^\n]*\n)[^\n]*\n/, "$1");
}

function createIntegrityBenchmark({ binary, runAt }) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-integrity-bench-"));
  const run = (command, args) => runAt(root, command, args);

  function setup() {
    fs.mkdirSync(path.join(root, "src"));
    fs.writeFileSync(
      path.join(root, "package.json"),
      JSON.stringify({ name: "integrity-benchmark", private: true, scripts: { test: "vitest run" } }, null, 2),
    );
    for (let index = 0; index < workload.sources; index += 1) {
      fs.writeFileSync(path.join(root, "src", `price${index}.ts`), sourceFile(index));
      fs.writeFileSync(
        path.join(root, "src", `price${index}.test.ts`),
        testFile(index, workload.testsPerFile),
      );
    }
    run("git", ["init", "-q"]);
    run("git", ["config", "user.email", "benchmark@repopilot.local"]);
    run("git", ["config", "user.name", "RepoPilot Benchmark"]);
    run("git", ["add", "."]);
    run("git", ["commit", "-qm", "integrity benchmark baseline"]);
    for (let index = 0; index < workload.sources; index += 1) {
      const test = path.join(root, "src", `price${index}.test.ts`);
      fs.writeFileSync(test, weaken(fs.readFileSync(test, "utf8")));
      fs.appendFileSync(path.join(root, "src", `price${index}.ts`), `export const revision${index} = 2;\n`);
    }
  }

  function review(output) {
    const started = process.hrtime.bigint();
    run(binary, ["review", ".", "--format", "json", "--output", output, "--no-progress"]);
    const wallMs = Number(process.hrtime.bigint() - started) / 1_000_000;
    const report = JSON.parse(fs.readFileSync(output, "utf8"));
    return { wallMs, report };
  }

  return {
    run() {
      setup();
      const output = path.join(root, "review.json");
      const warm = review(output).report;
      const signals = ["definitely", "maybe", "noise"]
        .flatMap((tier) => warm.tiered_signals?.[tier] ?? [])
        .filter((signal) => signal.family === "integrity").length;
      if ((warm.changed_files ?? []).length !== workload.sources * 2 || signals === 0) {
        throw new Error(
          `integrity benchmark expected ${workload.sources * 2} changed files with integrity signals, got ${(warm.changed_files ?? []).length} files and ${signals} signals`,
        );
      }
      const walls = [];
      const integrity = [];
      for (let iteration = 0; iteration < workload.iterations; iteration += 1) {
        const { wallMs, report } = review(output);
        walls.push(wallMs);
        integrity.push(report.review_timings.integrity_us / 1_000);
      }
      const wallMedian = median(walls);
      const integrityMedian = median(integrity);
      return {
        changed_files: workload.sources * 2,
        integrity_signals: signals,
        wall_median_ms: Number(wallMedian.toFixed(2)),
        integrity_median_ms: Number(integrityMedian.toFixed(3)),
        integrity_share: Number((integrityMedian / wallMedian).toFixed(4)),
        required_max_median_ms: workload.budgetMs,
        required_max_integrity_share: workload.maxIntegrityShare,
      };
    },
    cleanup() {
      fs.rmSync(root, { recursive: true, force: true });
    },
  };
}

module.exports = { createIntegrityBenchmark, weaken };
