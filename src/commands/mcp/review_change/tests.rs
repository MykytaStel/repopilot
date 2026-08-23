use super::verification_ids;
use serde_json::json;

#[test]
fn missing_and_empty_verification_selectors_are_equivalent() {
    assert_eq!(
        verification_ids(&json!({})).expect("missing selector"),
        verification_ids(&json!({ "verify": [] })).expect("empty selector")
    );
}

#[test]
fn verification_selectors_must_be_strings() {
    let error = verification_ids(&json!({ "verify": ["unit", 7] }))
        .expect_err("non-string selector must fail");

    assert!(error.to_string().contains("array of strings"));
}
