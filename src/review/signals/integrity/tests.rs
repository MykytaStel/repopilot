use super::{
    FileEvidence, IntegrityKind, IntegritySignal, collect_file_evidence, detect_integrity,
};
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use crate::review::signals::content::ReviewSource;
use std::path::PathBuf;

pub(super) fn changed(path: &str, status: ChangeStatus) -> ChangedFile {
    ChangedFile {
        path: PathBuf::from(path),
        status,
        ranges: vec![ChangedRange {
            start: 1,
            end: 10_000,
        }],
        hunks: Vec::new(),
    }
}

pub(super) fn source(label: &str, content: &str) -> ReviewSource {
    ReviewSource::new(content.to_string(), Some(label.to_string()))
}

pub(super) fn markers(
    path: &str,
    label: &str,
    pre: Option<&str>,
    post: Option<&str>,
) -> Option<FileEvidence> {
    let status = match (pre, post) {
        (None, Some(_)) => ChangeStatus::Added,
        (Some(_), None) => ChangeStatus::Deleted,
        _ => ChangeStatus::Modified,
    };
    let pre = pre.map(|content| source(label, content));
    let post = post.map(|content| source(label, content));
    collect_file_evidence(&changed(path, status), pre.as_ref(), post.as_ref())
}

pub(super) fn detect(
    path: &str,
    label: &str,
    pre: Option<&str>,
    post: Option<&str>,
) -> Vec<IntegritySignal> {
    detect_integrity(&[markers(path, label, pre, post).expect("recognizer applies")])
}

pub(super) fn kinds(signals: &[IntegritySignal]) -> Vec<IntegrityKind> {
    signals.iter().map(|signal| signal.kind).collect()
}

const JS_BEFORE: &str = r#"
describe("header", () => {
  it("renders title", () => { expect(render()).toContain("Title"); });
  it("renders logo", () => { expect(render()).toContain("logo"); });
});
"#;

#[test]
fn js_focused_test_is_definitely_sensitive() {
    let after = JS_BEFORE.replace("it(\"renders title\"", "it.only(\"renders title\"");
    let signals = detect(
        "src/header.test.ts",
        "TypeScript",
        Some(JS_BEFORE),
        Some(&after),
    );
    assert_eq!(kinds(&signals), vec![IntegrityKind::TestFocused]);
    assert_eq!(signals[0].line, 3);
    assert!(
        signals[0]
            .detail
            .contains("`it.only` added on \"renders title\"")
    );
}

#[test]
fn js_skip_forms_are_reported_with_their_test_names() {
    for (from, to, marker) in [
        ("it(\"renders logo\"", "it.skip(\"renders logo\"", "it.skip"),
        ("it(\"renders logo\"", "xit(\"renders logo\"", "xit"),
        (
            "it(\"renders logo\"",
            "test.todo(\"renders logo\"",
            "test.todo",
        ),
        (
            "describe(\"header\"",
            "describe.skip(\"header\"",
            "describe.skip",
        ),
    ] {
        let after = JS_BEFORE.replace(from, to);
        let signals = detect(
            "src/header.test.js",
            "JavaScript",
            Some(JS_BEFORE),
            Some(&after),
        );
        assert_eq!(
            kinds(&signals),
            vec![IntegrityKind::TestSkipped],
            "{marker}"
        );
        assert!(
            signals[0]
                .detail
                .starts_with(&format!("`{marker}` added on")),
            "{}",
            signals[0].detail
        );
    }
}

#[test]
fn js_runtime_skip_names_the_enclosing_test() {
    let after = JS_BEFORE.replace(
        "{ expect(render()).toContain(\"logo\"); }",
        "{ this.skip(); expect(render()).toContain(\"logo\"); }",
    );
    let signals = detect(
        "test/header.js",
        "JavaScript",
        Some(JS_BEFORE),
        Some(&after),
    );
    assert_eq!(kinds(&signals), vec![IntegrityKind::TestSkipped]);
    assert!(
        signals[0]
            .detail
            .contains("`this.skip` added on \"renders logo\"")
    );
}

#[test]
fn js_markers_in_strings_comments_or_other_calls_are_not_reported() {
    let after = JS_BEFORE.replace(
        "});\n",
        "  // it.only(\"later\") once the API lands\n  const note = \"describe.skip is banned\";\n  list.skip(2);\n});\n",
    );
    let signals = detect(
        "src/header.test.ts",
        "TypeScript",
        Some(JS_BEFORE),
        Some(&after),
    );
    assert!(signals.is_empty(), "{signals:?}");
}

#[test]
fn js_unchanged_or_removed_markers_are_not_reported() {
    let skipped = JS_BEFORE.replace("it(\"renders logo\"", "it.skip(\"renders logo\"");
    let edited = skipped.replace("\"logo\"); }", "\"logo\"); expect(1).toBe(1); }");
    assert!(
        detect(
            "src/header.test.ts",
            "TypeScript",
            Some(&skipped),
            Some(&edited)
        )
        .is_empty()
    );
    assert!(
        detect(
            "src/header.test.ts",
            "TypeScript",
            Some(&skipped),
            Some(JS_BEFORE)
        )
        .is_empty()
    );
}

#[test]
fn js_new_files_report_focus_but_not_skip() {
    let focused = JS_BEFORE.replace("it(\"renders logo\"", "it.only(\"renders logo\"");
    let skipped = JS_BEFORE.replace("it(\"renders logo\"", "it.skip(\"renders logo\"");
    assert_eq!(
        kinds(&detect(
            "src/new.test.ts",
            "TypeScript",
            None,
            Some(&focused)
        )),
        vec![IntegrityKind::TestFocused]
    );
    assert!(detect("src/new.test.ts", "TypeScript", None, Some(&skipped)).is_empty());
}

#[test]
fn a_skipped_test_moved_between_files_is_not_reported() {
    let skipped = JS_BEFORE.replace("it(\"renders logo\"", "it.skip(\"renders logo\"");
    let files = [
        markers(
            "src/a.test.ts",
            "TypeScript",
            Some(&skipped),
            Some(JS_BEFORE),
        )
        .unwrap(),
        markers(
            "src/b.test.ts",
            "TypeScript",
            Some(JS_BEFORE),
            Some(&skipped),
        )
        .unwrap(),
    ];
    assert!(detect_integrity(&files).is_empty());
}

#[test]
fn a_second_identical_skip_is_counted() {
    let once = JS_BEFORE.replace("it(\"renders logo\"", "it.skip(\"renders logo\"");
    let twice = format!("{once}\nit.skip(\"renders logo\", () => {{}});\n");
    let signals = detect(
        "src/header.test.ts",
        "TypeScript",
        Some(&once),
        Some(&twice),
    );
    assert_eq!(kinds(&signals), vec![IntegrityKind::TestSkipped]);
}

#[test]
fn fixture_and_testdata_inputs_are_not_tests() {
    let focused = JS_BEFORE.replace("it(\"renders logo\"", "it.only(\"renders logo\"");
    for path in [
        "tests/fixtures/review/after/src/cart.test.ts",
        "src/__fixtures__/cart.test.ts",
        "pkg/testdata/cart.test.ts",
    ] {
        assert!(
            markers(path, "TypeScript", Some(JS_BEFORE), Some(&focused)).is_none(),
            "{path}"
        );
    }
}

#[test]
fn non_test_files_are_ignored_outside_rust() {
    let after = JS_BEFORE.replace("it(\"renders logo\"", "it.skip(\"renders logo\"");
    assert!(markers("src/header.ts", "TypeScript", Some(JS_BEFORE), Some(&after)).is_none());
}

const PY_BEFORE: &str = r#"
import pytest

def test_login(client):
    assert client.login("a", "b")

class TestLogout(unittest.TestCase):
    def test_logout(self):
        self.assertTrue(logout())
"#;

#[test]
fn python_skip_decorators_and_calls_are_reported() {
    let decorated = PY_BEFORE.replace(
        "def test_login",
        "@pytest.mark.skip(reason=\"flaky on CI\")\ndef test_login",
    );
    let signals = detect(
        "tests/test_auth.py",
        "Python",
        Some(PY_BEFORE),
        Some(&decorated),
    );
    assert_eq!(kinds(&signals), vec![IntegrityKind::TestSkipped]);
    assert!(
        signals[0]
            .detail
            .contains("`@pytest.mark.skip` added on \"test_login\" (reason: \"flaky on CI\")"),
        "{}",
        signals[0].detail
    );

    let xfail = PY_BEFORE.replace("def test_login", "@pytest.mark.xfail\ndef test_login");
    assert_eq!(
        kinds(&detect(
            "tests/test_auth.py",
            "Python",
            Some(PY_BEFORE),
            Some(&xfail)
        )),
        vec![IntegrityKind::TestSkipped]
    );

    let runtime = PY_BEFORE.replace(
        "        self.assertTrue(logout())",
        "        self.skipTest(\"needs redis\")\n        self.assertTrue(logout())",
    );
    let signals = detect(
        "tests/test_auth.py",
        "Python",
        Some(PY_BEFORE),
        Some(&runtime),
    );
    assert_eq!(kinds(&signals), vec![IntegrityKind::TestSkipped]);
    assert!(
        signals[0]
            .detail
            .contains("`self.skipTest` added on \"test_logout\"")
    );

    let module = PY_BEFORE.replace(
        "import pytest\n",
        "import pytest\npytestmark = pytest.mark.skip\n",
    );
    assert_eq!(
        kinds(&detect(
            "tests/test_auth.py",
            "Python",
            Some(PY_BEFORE),
            Some(&module)
        )),
        vec![IntegrityKind::TestSkipped]
    );
}

#[test]
fn python_optional_dependency_guards_and_strings_are_not_reported() {
    let after = PY_BEFORE.replace(
        "import pytest\n",
        "import pytest\nredis = pytest.importorskip(\"redis\")\nNOTE = \"@pytest.mark.skip is banned\"\n",
    );
    assert!(
        detect(
            "tests/test_auth.py",
            "Python",
            Some(PY_BEFORE),
            Some(&after)
        )
        .is_empty()
    );
}

const GO_BEFORE: &str = r#"package auth

import "testing"

func TestLogin(t *testing.T) {
	if !login("a", "b") {
		t.Fatal("login failed")
	}
}
"#;

#[test]
fn go_skip_on_a_testing_handle_is_reported() {
    let after = GO_BEFORE.replace("\tif !login", "\tt.Skip(\"flaky until #42\")\n\tif !login");
    let signals = detect("auth/login_test.go", "Go", Some(GO_BEFORE), Some(&after));
    assert_eq!(kinds(&signals), vec![IntegrityKind::TestSkipped]);
    assert!(
        signals[0]
            .detail
            .contains("`t.Skip` added on \"TestLogin\" (reason: \"flaky until #42\")")
    );
}

#[test]
fn go_skip_on_a_non_testing_value_is_not_reported() {
    let after = GO_BEFORE.replace("\tif !login", "\treader.Skip(4)\n\tif !login");
    assert!(detect("auth/login_test.go", "Go", Some(GO_BEFORE), Some(&after)).is_empty());
}

const RUST_BEFORE: &str = r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds() {
        assert_eq!(add(1, 2), 3);
    }
}
"#;

#[test]
fn rust_ignore_in_an_inline_test_module_is_reported() {
    let after = RUST_BEFORE.replace(
        "    #[test]\n    fn adds",
        "    #[test]\n    #[ignore = \"slow\"]\n    fn adds",
    );
    let signals = detect("src/math.rs", "Rust", Some(RUST_BEFORE), Some(&after));
    assert_eq!(kinds(&signals), vec![IntegrityKind::TestSkipped]);
    assert!(
        signals[0]
            .detail
            .contains("`#[ignore]` added on \"adds\" (reason: \"slow\")"),
        "{}",
        signals[0].detail
    );
}

#[test]
fn rust_ignore_text_in_comments_and_strings_is_not_reported() {
    let after = RUST_BEFORE.replace(
        "        assert_eq!(add(1, 2), 3);",
        "        // #[ignore] this once CI is slow\n        let _ = \"#[ignore]\";\n        assert_eq!(add(1, 2), 3);",
    );
    assert!(detect("src/math.rs", "Rust", Some(RUST_BEFORE), Some(&after)).is_empty());
}
