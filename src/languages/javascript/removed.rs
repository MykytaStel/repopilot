//! Removed-behavior tables for the JS dialect family: which calls are test
//! cases, error handling, and auth checks when counting what a change removed.

use crate::review::signals::tables::RemovedTables;

pub(super) static JS_FAMILY_REMOVED: RemovedTables = RemovedTables {
    extensions: &["js", "mjs", "cjs", "ts", "mts", "cts", "tsx", "jsx"],
    is_test_case: |node, content| {
        node.kind() == "call_expression"
            && node
                .child_by_field_name("function")
                .and_then(|callee| callee.utf8_text(content.as_bytes()).ok())
                .is_some_and(is_test_callee)
    },
    is_error_handling: |node, _| node.kind() == "try_statement",
    auth_call_kinds: &["call_expression"],
};

/// `test`, `it`, and `describe`, including their modified forms (`it.skip`,
/// `test.only`, `describe.each(...)`) and the `x`/`f` aliases. A skipped or
/// focused test is still a test case in the file; the skip or focus itself is
/// an integrity signal.
fn is_test_callee(callee: &str) -> bool {
    let root = callee.trim().split(['.', '(']).next().unwrap_or_default();
    matches!(
        root,
        "test" | "it" | "describe" | "xit" | "xtest" | "xdescribe" | "fit" | "fdescribe"
    )
}

#[cfg(test)]
mod tests {
    use super::is_test_callee;

    #[test]
    fn modified_and_aliased_test_calls_are_test_cases() {
        for callee in [
            "it",
            "test",
            "describe",
            "it.skip",
            "test.only",
            "describe.skip",
            "it.each([1, 2])",
            "test.concurrent",
            "xit",
            "fdescribe",
        ] {
            assert!(is_test_callee(callee), "{callee}");
        }
    }

    #[test]
    fn other_calls_are_not_test_cases() {
        for callee in [
            "expect",
            "beforeEach",
            "testing.render",
            "items.it",
            "describeUser",
        ] {
            assert!(!is_test_callee(callee), "{callee}");
        }
    }
}
