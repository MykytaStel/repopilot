const test = require("node:test");
const assert = require("node:assert/strict");
const { semanticReport } = require("./review-performance-large-diff");

test("semantic report ignores timings and cache telemetry", () => {
  const first = {
    impact_paths: { affected: ["src/a.rs"] },
    review_timings: { review_signals_us: 100 },
    scan_timings: { file_scan_us: 500 },
    scan_duration_us: 510,
    cache_telemetry: { hits: 2, misses: 0 },
    context_graph_cache: { status: "hit", reason: "loaded", cache_path: ".cache" },
  };
  const second = {
    ...first,
    review_timings: { review_signals_us: 900 },
    scan_timings: { file_scan_us: 1200 },
    scan_duration_us: 1250,
    cache_telemetry: { hits: 0, misses: 2 },
    context_graph_cache: { status: "miss", reason: "absent", cache_path: ".cache" },
  };

  assert.deepEqual(semanticReport(first), semanticReport(second));
});

test("semantic report compares impact, diagnostics, verification, and readiness", () => {
  const first = {
    impact_paths: { affected: ["src/a.rs"] },
    diagnostics: [{ code: "D1", message: "one" }],
    verification: [{ status: "passed", command: "cargo test" }],
    merge_readiness: { status: "ready" },
    evidence: { status: "sufficient", sources: ["diff"] },
    ownership: { owners: ["team-a"] },
    blast_radius: ["src/a.rs"],
  };
  assert.deepEqual(semanticReport(first), semanticReport({ ...first }));

  const changes = [
    ["impact_paths", { affected: ["src/b.rs"] }],
    ["diagnostics", [{ code: "D2", message: "two" }]],
    ["verification", [{ status: "failed", command: "cargo test" }]],
    ["merge_readiness", { status: "blocked" }],
    ["evidence", { status: "insufficient", sources: [] }],
    ["ownership", { owners: ["team-b"] }],
    ["blast_radius", ["src/b.rs"]],
  ];
  for (const [field, value] of changes) {
    assert.notDeepEqual(
      semanticReport(first),
      semanticReport({ ...first, [field]: value }),
      `${field} changes must affect the semantic signature`,
    );
  }
});
