use super::behavioral::{
    BehavioralKind, BehavioralSignal, BehavioralSignalSource, DependencyContext,
    detect_behavioral_added,
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
        .filter(|signal| signal.kind == BehavioralKind::QuietFallbackIntroduced)
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

#[test]
fn python_empty_except_with_call_and_later_return_is_reported() {
    let source = "def thumbnail(image):\n    try:\n        return native_transform(image)\n    except ImportError:\n        pass\n    return image\n";
    let found = signals(
        source,
        "src/thumbnail.py",
        "Python",
        line_of(source, "pass"),
    );

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, line_of(source, "pass"));
    assert!(found[0].detail.contains("native_transform"));
    assert!(found[0].detail.contains("return line 6"));
}

#[test]
fn python_optional_import_fallback_is_reported() {
    let source = "def decode(data):\n    try:\n        import native_image\n    except ImportError:\n        pass\n    return []\n";
    let found = signals(source, "src/decode.py", "Python", line_of(source, "pass"));

    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].detail.contains("import native_image"));
}

#[test]
fn python_deferred_lambda_and_generator_calls_are_not_caught_operations() {
    let lambda = "def load():\n    try:\n        fallback = lambda: decode()\n    except ImportError:\n        pass\n    return fallback\n";
    let generator = "def load(items):\n    try:\n        values = (decode(item) for item in items)\n    except ImportError:\n        pass\n    return values\n";

    assert!(signals(lambda, "src/load.py", "Python", 5).is_empty());
    assert!(signals(generator, "src/load.py", "Python", 5).is_empty());
}

#[test]
fn python_generator_outer_iterable_call_is_caught_inside_try() {
    let source = "def load():\n    try:\n        values = (item for item in make_items())\n    except ImportError:\n        pass\n    return values\n";
    let found = signals(source, "src/load.py", "Python", line_of(source, "pass"));

    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].detail.contains("make_items"));
}

#[test]
fn python_pass_only_handler_allows_comments() {
    let source = "def load():\n    try:\n        decode()\n    except ImportError:\n        # optional decoder is unavailable\n        pass\n    return None\n";
    let found = signals(source, "src/load.py", "Python", line_of(source, "pass"));

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, line_of(source, "pass"));
}

#[test]
fn python_comment_only_change_does_not_introduce_a_pass_fallback() {
    let source = "def load():\n    try:\n        decode()\n    except ImportError:\n        # optional decoder is unavailable\n        pass\n    return None\n";
    assert!(
        signals(
            source,
            "src/load.py",
            "Python",
            line_of(source, "optional decoder")
        )
        .is_empty()
    );
}

#[test]
fn python_changed_pass_followed_by_recovery_is_not_pass_only() {
    let source = "def load():\n    try:\n        decode()\n    except ImportError:\n        pass\n        log_recovery()\n    return None\n";
    assert!(signals(source, "src/load.py", "Python", line_of(source, "pass")).is_empty());
}

#[test]
fn python_changed_handler_body_is_reported_even_when_header_is_unchanged() {
    let source = "def thumbnail(image):\n    try:\n        return native_transform(image)\n    except ImportError:\n        pass\n    return image\n";
    let found = signals(
        source,
        "src/thumbnail.py",
        "Python",
        line_of(source, "pass"),
    );

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, line_of(source, "pass"));
}

#[test]
fn python_explicit_recovery_is_excluded() {
    let source = "def thumbnail(image):\n    try:\n        return native_transform(image)\n    except ImportError:\n        return image.copy()\n";
    assert!(signals(source, "src/thumbnail.py", "Python", 4).is_empty());
}

#[test]
fn python_reraise_and_handlers_without_a_later_return_are_excluded() {
    let reraising =
        "def load():\n    try:\n        return decode()\n    except DecodeError:\n        raise\n";
    let no_later_return =
        "def log():\n    try:\n        emit()\n    except RuntimeError:\n        pass\n";

    assert!(signals(reraising, "src/load.py", "Python", 4).is_empty());
    assert!(signals(no_later_return, "src/log.py", "Python", 4).is_empty());
}

#[test]
fn python_pass_only_handler_without_a_call_is_excluded() {
    let source = "def load():\n    try:\n        value = 1\n    except RuntimeError:\n        pass\n    return value\n";
    assert!(signals(source, "src/load.py", "Python", 4).is_empty());
}

#[test]
fn python_return_inside_a_nested_function_does_not_count_as_fallback() {
    let source = "def process():\n    try:\n        convert()\n    except ImportError:\n        pass\n    def fallback():\n        return []\n";
    assert!(signals(source, "src/process.py", "Python", 4).is_empty());
}

#[test]
fn python_test_file_unchanged_handler_and_parse_error_are_excluded() {
    let source = "def thumbnail(image):\n    try:\n        return native_transform(image)\n    except ImportError:\n        pass\n    return image\n";
    assert!(signals(source, "tests/test_thumbnail.py", "Python", 4).is_empty());
    assert!(signals(source, "src/thumbnail.py", "Python", 1).is_empty());
    assert!(signals(
        "def broken(:\n    try:\n        transform()\n    except Error:\n        pass\n    return None\n",
        "src/broken.py",
        "Python",
        4,
    )
    .is_empty());
}
