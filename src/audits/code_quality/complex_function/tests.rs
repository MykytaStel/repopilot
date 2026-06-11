use super::ComplexFunctionAudit;
use crate::audits::traits::FileAudit;
use crate::findings::types::Finding;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use std::path::PathBuf;

fn ts_file(path: &str, content: &str) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: Some("TypeScript".to_string()),
        non_empty_lines: content.lines().filter(|l| !l.trim().is_empty()).count(),
        branch_count: 0,
        imports: Vec::new(),
        content: Some(content.to_string()),
        has_inline_tests: false,
    }
}

fn run(file: &FileFacts) -> Vec<Finding> {
    ComplexFunctionAudit.audit(file, &ScanConfig::default())
}

#[test]
fn flags_a_deeply_nested_handler() {
    // Nesting cost: 1+2+3+4+5+6 = 21, over the default threshold of 15.
    let content = "\
export function handle(items: number[]): void {
  if (a) {
    for (const x of items) {
      if (b) {
        while (c) {
          if (d) {
            if (e) {
              doThing(x);
            }
          }
        }
      }
    }
  }
}
";
    let file = ts_file("src/handler.ts", content);
    let findings = run(&file);

    assert_eq!(findings.len(), 1, "deeply nested handler should be flagged");
    let finding = &findings[0];
    assert_eq!(finding.rule_id, "code-quality.complex-function");
    // Evidence spans the whole function and the snippet states the score.
    assert_eq!(finding.evidence[0].line_start, 1);
    assert!(finding.evidence[0].line_end.unwrap() > 1);
    assert!(
        finding.evidence[0].snippet.contains("score="),
        "snippet should carry the score: {}",
        finding.evidence[0].snippet
    );
}

#[test]
fn does_not_flag_a_wide_but_flat_dispatcher() {
    // A switch costs 1 regardless of arm count — the case complex-file gets wrong.
    let content = "\
export function dispatch(kind: string): string {
  switch (kind) {
    case \"a\": return doA();
    case \"b\": return doB();
    case \"c\": return doC();
    case \"d\": return doD();
    case \"e\": return doE();
    case \"f\": return doF();
    case \"g\": return doG();
    case \"h\": return doH();
    case \"i\": return doI();
    case \"j\": return doJ();
    case \"k\": return doK();
    case \"l\": return doL();
    default: return doDefault();
  }
}
";
    let file = ts_file("src/dispatch.ts", content);
    assert!(
        run(&file).is_empty(),
        "a flat dispatcher must score low and not be flagged"
    );
}

#[test]
fn ignores_non_production_paths() {
    let content = "\
export function handle(): void {
  if (a) { for (const x of y) { if (b) { while (c) { if (d) { if (e) { go(); } } } } } }
}
";
    let file = ts_file("src/handler.test.ts", content);
    assert!(
        run(&file).is_empty(),
        "test files are out of scope for this rule"
    );
}

#[test]
fn nested_closures_are_scored_independently_not_folded_in() {
    // The outer function's own control flow is shallow; the inner callbacks are
    // separate functions, so nothing crosses the threshold.
    let content = "\
export function setup(list: number[]): void {
  list.forEach((x) => {
    if (x > 0) {
      doA(x);
    }
  });
  list.map((y) => (y > 1 ? doB(y) : doC(y)));
}
";
    let file = ts_file("src/setup.ts", content);
    assert!(
        run(&file).is_empty(),
        "shallow outer + shallow closures should not be flagged"
    );
}
