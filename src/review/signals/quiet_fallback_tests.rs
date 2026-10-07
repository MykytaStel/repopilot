use super::behavioral::{
    BehavioralSignal, BehavioralSignalSource, DependencyContext, detect_behavioral_added,
};
use super::content::ReviewSource;
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use std::path::PathBuf;

fn signals(source: &str, path: &str, language: &str, changed_line: usize) -> Vec<BehavioralSignal> {
    let file = ChangedFile {
        path: PathBuf::from(path),
        status: ChangeStatus::Modified,
        ranges: vec![ChangedRange {
            start: changed_line,
            end: changed_line,
        }],
        hunks: Vec::new(),
    };
    let source = ReviewSource::new(source.to_string(), Some(language.to_string()));
    detect_behavioral_added(&file, &source, &DependencyContext::default())
        .into_iter()
        .filter(|signal| format!("{:?}", signal.kind) == "QuietFallbackIntroduced")
        .collect()
}

fn line_of(source: &str, needle: &str) -> usize {
    source
        .lines()
        .position(|line| line.contains(needle))
        .expect("source contains line")
        + 1
}

#[test]
fn javascript_image_transform_empty_catch_with_later_return_is_reported() {
    let source = r#"async function loadSharp() {
  return import("sharp").catch(() => null);
}
export async function thumbnail(input) {
  const sharp = await loadSharp();
  if (sharp) {
    try {
      return await sharp(input).resize({ width: 320 }).webp().toBuffer();
    } catch {
      // Processing errors fall through to the original bytes.
    }
  }
  return Buffer.from(input);
}"#;
    let signals = signals(
        source,
        "src/thumbnails.js",
        "JavaScript",
        line_of(source, "} catch {"),
    );

    assert_eq!(signals.len(), 1, "{signals:?}");
    assert_eq!(signals[0].line, line_of(source, "} catch {"));
    assert_eq!(signals[0].source, BehavioralSignalSource::Ast);
    assert!(
        signals[0].detail.contains("sharp(input)"),
        "{}",
        signals[0].detail
    );
    assert!(
        signals[0].detail.contains("return line"),
        "{}",
        signals[0].detail
    );
}

#[test]
fn typescript_comment_only_catch_is_reported() {
    let source = r#"export async function thumbnail(input: Buffer) {
  try {
    return await transform(input);
  } catch {
    // The body is still structurally empty.
  }
  return input;
}"#;
    let signals = signals(
        source,
        "src/thumbnail.ts",
        "TypeScript",
        line_of(source, "} catch {"),
    );

    assert_eq!(signals.len(), 1, "{signals:?}");
}

#[test]
fn optional_dynamic_import_catch_is_excluded() {
    let source = "async function load() { return import(\"sharp\").catch(() => null); }\n";
    assert!(signals(source, "src/native.js", "JavaScript", 1).is_empty());
}

#[test]
fn explicit_error_recovery_and_rethrow_is_excluded() {
    let source = r#"function read() {
  try {
    return parse();
  } catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}"#;
    assert!(
        signals(
            source,
            "src/read.js",
            "JavaScript",
            line_of(source, "} catch")
        )
        .is_empty()
    );
}

#[test]
fn empty_catch_without_later_return_is_excluded() {
    let source = "function report() {\n  try { log(); } catch {}\n}\n";
    assert!(signals(source, "src/report.js", "JavaScript", 2).is_empty());
}

#[test]
fn return_inside_nested_function_does_not_count_as_fallback() {
    let source = r#"function outer(input) {
  try { transform(input); } catch {}
  const nested = () => { return input; };
}"#;
    assert!(signals(source, "src/outer.js", "JavaScript", 2).is_empty());
}

#[test]
fn unchanged_catch_line_is_excluded() {
    let source =
        "function load(input) {\n  try { return parse(input); } catch {}\n  return input;\n}\n";
    assert!(signals(source, "src/load.js", "JavaScript", 1).is_empty());
}

#[test]
fn test_file_is_excluded() {
    let source = "async function check(input) {\n  try { return await transform(input); } catch {}\n  return input;\n}\n";
    assert!(signals(source, "tests/thumbnail.test.js", "JavaScript", 2).is_empty());
}

#[test]
fn parse_error_is_excluded() {
    let source = "function broken( { try { transform(); } catch {} return null; }\n";
    assert!(signals(source, "src/broken.js", "JavaScript", 1).is_empty());
}
