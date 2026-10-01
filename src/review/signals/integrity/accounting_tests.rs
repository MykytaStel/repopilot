use super::IntegrityKind;
use super::detect_integrity;
use super::tests::{detect, kinds, markers};

const JS_BEFORE: &str = r#"
describe("cart", () => {
  it("sums line items", () => {
    expect(total([item(2, 3)])).toBe(6);
    expect(total([])).toBe(0);
  });
  it("applies the discount", () => {
    expect(total([item(10, 1)], 0.1)).toBe(9);
  });
  it("rejects negative quantities", () => {
    expect(() => total([item(1, -1)])).toThrow();
  });
});
"#;

#[test]
fn js_removed_test_is_reported_with_its_qualified_name() {
    let after = JS_BEFORE.replace(
        "  it(\"rejects negative quantities\", () => {\n    expect(() => total([item(1, -1)])).toThrow();\n  });\n",
        "",
    );
    let signals = detect(
        "src/cart.test.ts",
        "TypeScript",
        Some(JS_BEFORE),
        Some(&after),
    );
    assert_eq!(kinds(&signals), vec![IntegrityKind::TestRemoved]);
    assert_eq!(
        signals[0].detail,
        "1 test removed: \"cart > rejects negative quantities\""
    );
}

#[test]
fn js_substituted_test_names_the_new_one() {
    let after = JS_BEFORE.replace(
        "it(\"rejects negative quantities\", () => {\n    expect(() => total([item(1, -1)])).toThrow();",
        "it(\"handles quantities\", () => {\n    expect(total([item(1, 1)])).toBe(1);",
    );
    let signals = detect(
        "src/cart.test.ts",
        "TypeScript",
        Some(JS_BEFORE),
        Some(&after),
    );
    assert_eq!(kinds(&signals), vec![IntegrityKind::TestRemoved]);
    assert!(
        signals[0]
            .detail
            .contains("1 new test in this file: \"cart > handles quantities\""),
        "{}",
        signals[0].detail
    );
}

#[test]
fn js_dropped_and_trivialized_assertions_are_reported() {
    let dropped = JS_BEFORE.replace("    expect(total([])).toBe(0);\n", "");
    let signals = detect(
        "src/cart.test.ts",
        "TypeScript",
        Some(JS_BEFORE),
        Some(&dropped),
    );
    assert_eq!(kinds(&signals), vec![IntegrityKind::AssertionsRemoved]);
    assert_eq!(
        signals[0].detail,
        "\"cart > sums line items\": assertions 2 → 1"
    );

    let trivial = JS_BEFORE.replace(
        "expect(total([item(10, 1)], 0.1)).toBe(9);",
        "expect(true).toBe(true);",
    );
    let signals = detect(
        "src/cart.test.ts",
        "TypeScript",
        Some(JS_BEFORE),
        Some(&trivial),
    );
    assert_eq!(kinds(&signals), vec![IntegrityKind::AssertionsRemoved]);
    assert!(
        signals[0]
            .detail
            .contains("assertions 1 → 0; the test can no longer fail")
    );
}

#[test]
fn js_rewritten_but_equally_checked_tests_are_quiet() {
    let after = JS_BEFORE
        .replace(
            "expect(total([])).toBe(0);",
            "expect(total([])).toEqual(0);",
        )
        .replace("item(10, 1)", "item(20, 1)")
        .replace("toBe(9)", "toBe(18)");
    assert!(
        detect(
            "src/cart.test.ts",
            "TypeScript",
            Some(JS_BEFORE),
            Some(&after)
        )
        .is_empty()
    );
}

#[test]
fn a_test_moved_to_another_file_is_not_removed() {
    let moved_block = "  it(\"rejects negative quantities\", () => {\n    expect(() => total([item(1, -1)])).toThrow();\n  });\n";
    let without = JS_BEFORE.replace(moved_block, "");
    let target_before = "describe(\"cart\", () => {\n});\n";
    let target_after = format!("describe(\"cart\", () => {{\n{moved_block}}});\n");
    let files = [
        markers(
            "src/cart.test.ts",
            "TypeScript",
            Some(JS_BEFORE),
            Some(&without),
        )
        .unwrap(),
        markers(
            "src/cart-errors.test.ts",
            "TypeScript",
            Some(target_before),
            Some(&target_after),
        )
        .unwrap(),
    ];
    assert!(detect_integrity(&files).is_empty());
}

#[test]
fn emptying_a_test_file_is_left_to_the_behavioral_signal() {
    let emptied = "import { total } from \"./cart\";\n";
    assert!(
        detect(
            "src/cart.test.ts",
            "TypeScript",
            Some(JS_BEFORE),
            Some(emptied)
        )
        .is_empty()
    );
}

const PY_BEFORE: &str = r#"
import pytest

class TestInvoice:
    def test_total(self):
        assert invoice_total(100, tax=0.2) == 120
        assert invoice_total(0, tax=0.2) == 0

    def test_rejects_negative(self):
        with pytest.raises(ValueError):
            invoice_total(-1)
"#;

#[test]
fn python_removed_and_weakened_tests_are_reported() {
    let after = PY_BEFORE
        .replace("        assert invoice_total(0, tax=0.2) == 0\n", "        assert True\n")
        .replace(
            "    def test_rejects_negative(self):\n        with pytest.raises(ValueError):\n            invoice_total(-1)\n",
            "",
        );
    let signals = detect(
        "tests/test_invoice.py",
        "Python",
        Some(PY_BEFORE),
        Some(&after),
    );
    let mut found = kinds(&signals);
    found.sort_by_key(|kind| format!("{kind:?}"));
    assert_eq!(
        found,
        vec![IntegrityKind::AssertionsRemoved, IntegrityKind::TestRemoved]
    );
    assert!(
        signals
            .iter()
            .any(|signal| signal.detail == "\"TestInvoice.test_total\": assertions 2 → 1")
    );
    assert!(
        signals
            .iter()
            .any(|signal| signal.detail == "1 test removed: \"TestInvoice.test_rejects_negative\"")
    );
}

const GO_BEFORE: &str = r#"package billing

import "testing"

func TestTotal(t *testing.T) {
	if got := Total(2, 3); got != 6 {
		t.Errorf("Total = %d, want 6", got)
	}
	if got := Total(0, 3); got != 0 {
		t.Fatalf("Total = %d, want 0", got)
	}
}
"#;

#[test]
fn go_removed_failure_report_is_an_assertion_drop() {
    let after = GO_BEFORE.replace(
        "\tif got := Total(0, 3); got != 0 {\n\t\tt.Fatalf(\"Total = %d, want 0\", got)\n\t}\n",
        "\t_ = Total(0, 3)\n",
    );
    let signals = detect("billing/total_test.go", "Go", Some(GO_BEFORE), Some(&after));
    assert_eq!(kinds(&signals), vec![IntegrityKind::AssertionsRemoved]);
    assert_eq!(signals[0].detail, "\"TestTotal\": assertions 2 → 1");
}

const RUST_BEFORE: &str = r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds() {
        assert_eq!(add(1, 2), 3);
        assert_eq!(add(-1, 1), 0);
    }

    #[test]
    fn overflow_wraps() {
        assert_eq!(add(i32::MAX, 0), i32::MAX);
    }
}
"#;

#[test]
fn rust_inline_tests_are_accounted_by_module_path() {
    let after = RUST_BEFORE
        .replace("        assert_eq!(add(-1, 1), 0);\n", "        assert!(true);\n")
        .replace(
            "\n    #[test]\n    fn overflow_wraps() {\n        assert_eq!(add(i32::MAX, 0), i32::MAX);\n    }\n",
            "\n",
        );
    let signals = detect("src/math.rs", "Rust", Some(RUST_BEFORE), Some(&after));
    assert!(
        signals
            .iter()
            .any(|signal| signal.kind == IntegrityKind::AssertionsRemoved
                && signal.detail == "\"tests::adds\": assertions 2 → 1")
    );
    assert!(
        signals
            .iter()
            .any(|signal| signal.kind == IntegrityKind::TestRemoved
                && signal.detail == "1 test removed: \"tests::overflow_wraps\"")
    );
}

#[test]
fn a_renamed_test_with_the_same_checks_is_not_removed() {
    let after = JS_BEFORE.replace(
        "it(\"rejects negative quantities\"",
        "it(\"throws on negative quantities\"",
    );
    assert!(
        detect(
            "src/cart.test.ts",
            "TypeScript",
            Some(JS_BEFORE),
            Some(&after)
        )
        .is_empty()
    );
}

#[test]
fn a_renamed_test_that_lost_assertions_is_reported_as_such() {
    let after = JS_BEFORE
        .replace("it(\"sums line items\"", "it(\"adds up line items\"")
        .replace("    expect(total([])).toBe(0);\n", "");
    let signals = detect(
        "src/cart.test.ts",
        "TypeScript",
        Some(JS_BEFORE),
        Some(&after),
    );
    assert_eq!(kinds(&signals), vec![IntegrityKind::AssertionsRemoved]);
    assert_eq!(
        signals[0].detail,
        "\"cart > sums line items\" renamed to \"cart > adds up line items\": assertions 2 → 1"
    );
}
