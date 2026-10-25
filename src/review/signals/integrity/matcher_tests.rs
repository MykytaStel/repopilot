use super::tests::{detect, kinds};

#[test]
fn js_object_matcher_weakening_is_reported() {
    let before = r#"
describe("cart", () => {
  it("returns an invoice", () => {
    expect(invoice()).toEqual({ id: 1, total: 20 });
  });
});
"#;
    let after = before.replace("toEqual({ id: 1, total: 20 })", "toMatchObject({ id: 1 })");
    let signals = detect("src/cart.test.ts", "TypeScript", Some(before), Some(&after));

    assert_eq!(signals.len(), 1, "{signals:?}");
    assert_eq!(
        kinds(&signals),
        vec![super::IntegrityKind::AssertionsWeakened]
    );
    assert_eq!(
        signals[0].detail,
        "\"cart > returns an invoice\": toEqual → toMatchObject"
    );
    assert_eq!(signals[0].line, 3);
}

#[test]
fn to_equal_object_containing_is_reported_as_weakened() {
    let before = r#"
it("returns an invoice", () => {
  expect(invoice()).toEqual({ id: 1, total: 20 });
});
"#;
    let after = before.replace(
        "toEqual({ id: 1, total: 20 })",
        "toEqual(expect.objectContaining({ id: 1 }))",
    );
    let signals = detect("src/cart.test.ts", "TypeScript", Some(before), Some(&after));

    assert_eq!(signals.len(), 1, "{signals:?}");
    assert_eq!(
        kinds(&signals),
        vec![super::IntegrityKind::AssertionsWeakened]
    );
    assert_eq!(
        signals[0].detail,
        "\"returns an invoice\": toEqual → toEqual(expect.objectContaining)"
    );
    assert_eq!(signals[0].line, 2);
}

#[test]
fn editing_or_adding_object_containing_matchers_is_not_reported() {
    let before = r#"
it("returns an invoice", () => {
  expect(invoice()).toEqual(expect.objectContaining({ id: 1 }));
});
"#;
    let edited = before.replace("id: 1", "id: 2");
    assert!(
        detect(
            "src/cart.test.ts",
            "TypeScript",
            Some(before),
            Some(&edited)
        )
        .is_empty()
    );

    let added = format!(
        "{before}\nit(\"returns a partial invoice\", () => {{\n  expect(invoice()).toEqual(expect.objectContaining({{ id: 1 }}));\n}});\n"
    );
    assert!(detect("src/cart.test.ts", "TypeScript", Some(before), Some(&added),).is_empty());
}

#[test]
fn matcher_changes_in_same_file_helpers_are_reported() {
    let before = r#"
const expectInvoice = () => {
  expect(invoice()).toEqual({ id: 1, total: 20 });
};
it("returns an invoice", () => {
  expectInvoice();
});
"#;
    let after = before.replace("toEqual({ id: 1, total: 20 })", "toMatchObject({ id: 1 })");
    let signals = detect("src/cart.test.ts", "TypeScript", Some(before), Some(&after));

    assert_eq!(signals.len(), 1, "{signals:?}");
    assert_eq!(
        signals[0].detail,
        "\"returns an invoice\": toEqual → toMatchObject"
    );
}

#[test]
fn object_containing_changes_inside_same_file_helpers_are_reported() {
    let before = r#"
const expectInvoice = () => {
  expect(invoice()).toEqual({ id: 1, total: 20 });
};
it("returns an invoice", () => {
  expectInvoice();
});
"#;
    let after = before.replace(
        "toEqual({ id: 1, total: 20 })",
        "toEqual(expect.objectContaining({ id: 1 }))",
    );
    let signals = detect("src/cart.test.ts", "TypeScript", Some(before), Some(&after));

    assert_eq!(signals.len(), 1, "{signals:?}");
    assert_eq!(
        signals[0].detail,
        "\"returns an invoice\": toEqual → toEqual(expect.objectContaining)"
    );
}

#[test]
fn adding_or_strengthening_partial_matchers_is_not_reported() {
    let before = r#"
it("returns an invoice", () => {
  expect(invoice()).toMatchObject({ id: 1 });
});
"#;
    let strengthened = before.replace("toMatchObject({ id: 1 })", "toEqual({ id: 1 })");
    assert!(
        detect(
            "src/cart.test.ts",
            "TypeScript",
            Some(before),
            Some(&strengthened),
        )
        .is_empty()
    );

    let added = format!(
        "{before}\nit(\"returns a partial invoice\", () => {{\n  expect(invoice()).toMatchObject({{ id: 1 }});\n}});\n"
    );
    assert!(detect("src/cart.test.ts", "TypeScript", Some(before), Some(&added),).is_empty());
}

#[test]
fn changing_expected_values_without_loosening_matcher_is_not_reported() {
    let before = r#"
it("returns an invoice", () => {
  expect(invoice()).toEqual({ id: 1, total: 20 });
});
"#;
    let after = before.replace("total: 20", "total: 24");
    let signals = detect("src/cart.test.ts", "TypeScript", Some(before), Some(&after));
    assert!(signals.is_empty(), "{signals:?}");
}

#[test]
fn non_expect_and_constant_only_matcher_calls_are_not_reported() {
    let before = r#"
it("uses a matcher helper", () => {
  matcher.toEqual({ id: 1 });
  expect(true).toEqual({ ok: true });
});
"#;
    let after = before
        .replace("matcher.toEqual", "matcher.toMatchObject")
        .replace("expect(true).toEqual", "expect(true).toMatchObject");
    let signals = detect(
        "src/matcher.test.ts",
        "TypeScript",
        Some(before),
        Some(&after),
    );
    assert!(signals.is_empty(), "{signals:?}");
}

#[test]
fn primitive_matcher_changes_are_outside_the_object_transition() {
    let before = r#"
it("returns a count", () => {
  expect(count()).toEqual(3);
});
"#;
    let after = before.replace("toEqual(3)", "toMatchObject(3)");
    let signals = detect(
        "src/count.test.ts",
        "TypeScript",
        Some(before),
        Some(&after),
    );
    assert!(signals.is_empty(), "{signals:?}");
}
