use super::IntegrityKind;
use super::tests::{detect, kinds};

const PY_INLINE: &str = r#"
def test_typecast():
    result = run_device(source)
    assert result.shape == golden.shape
    assert passed(golden, result)
"#;

const PY_EXTRACTED: &str = r#"
def test_typecast():
    _run_typecast(source)


def test_typecast_rounding():
    _run_typecast(boundaries, max_ulp=0)


def _run_typecast(values, max_ulp=None):
    result = run_device(values)
    assert result.shape == golden.shape
    assert passed(golden, result, max_ulp=max_ulp)
"#;

#[test]
fn assertions_moved_into_a_same_file_helper_still_count() {
    let signals = detect(
        "tests/test_typecast.py",
        "Python",
        Some(PY_INLINE),
        Some(PY_EXTRACTED),
    );
    assert!(signals.is_empty(), "{signals:?}");
}

#[test]
fn an_assertion_dropped_from_a_helper_is_reported_on_the_test() {
    let after = PY_EXTRACTED.replace("    assert result.shape == golden.shape\n", "");
    let signals = detect(
        "tests/test_typecast.py",
        "Python",
        Some(PY_EXTRACTED),
        Some(&after),
    );
    assert_eq!(
        kinds(&signals),
        vec![
            IntegrityKind::AssertionsRemoved,
            IntegrityKind::AssertionsRemoved
        ]
    );
    assert_eq!(signals[0].detail, "\"test_typecast\": assertions 2 → 1");
}

#[test]
fn js_helper_called_per_case_counts_once_per_call() {
    let before = r#"
it("sums", () => {
  expect(total([1, 2])).toBe(3);
  expect(total([])).toBe(0);
});
"#;
    let after = r#"
const expectTotal = (items, sum) => {
  expect(total(items)).toBe(sum);
};
it("sums", () => {
  expectTotal([1, 2], 3);
  expectTotal([], 0);
});
"#;
    let signals = detect("src/total.test.ts", "TypeScript", Some(before), Some(after));
    assert!(signals.is_empty(), "{signals:?}");
}

const GO_SEQUENTIAL: &str = r#"
package helmet

func Test_HSTSHeaders(t *testing.T) {
	require.Equal(t, "max-age=60", header(t, Config{HSTSMaxAge: 60}, "https"))
	require.Equal(t, "", header(t, Config{HSTSMaxAge: 60}, "http"))
}
"#;

const GO_TABLE: &str = r#"
package helmet

func Test_HSTSHeaders(t *testing.T) {
	tests := []struct {
		name, scheme, expected string
	}{
		{"tls", "https", "max-age=60"},
		{"plain", "http", ""},
		{"zero", "https", ""},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			require.Equal(t, tt.expected, header(t, Config{HSTSMaxAge: 60}, tt.scheme))
		})
	}
}
"#;

#[test]
fn a_test_turned_table_driven_is_not_compared_by_statement_count() {
    let signals = detect(
        "middleware/helmet/helmet_test.go",
        "Go",
        Some(GO_SEQUENTIAL),
        Some(GO_TABLE),
    );
    assert!(signals.is_empty(), "{signals:?}");
}

#[test]
fn a_table_driven_test_that_loses_an_assertion_is_reported() {
    let before = GO_TABLE.replace(
        "\t\t\trequire.Equal(t, tt.expected,",
        "\t\t\trequire.NotEmpty(t, tt.name)\n\t\t\trequire.Equal(t, tt.expected,",
    );
    let signals = detect(
        "middleware/helmet/helmet_test.go",
        "Go",
        Some(&before),
        Some(GO_TABLE),
    );
    assert_eq!(kinds(&signals), vec![IntegrityKind::AssertionsRemoved]);
    assert_eq!(signals[0].detail, "\"Test_HSTSHeaders\": assertions 2 → 1");
}

#[test]
fn recursive_helpers_terminate() {
    let before = r#"
def test_walk():
    _walk(tree)
    assert tree.visited


def _walk(node):
    for child in node.children:
        _walk(child)
"#;
    let after = before.replace("    assert tree.visited\n", "");
    let signals = detect("tests/test_walk.py", "Python", Some(before), Some(&after));
    assert_eq!(kinds(&signals), vec![IntegrityKind::AssertionsRemoved]);
}
