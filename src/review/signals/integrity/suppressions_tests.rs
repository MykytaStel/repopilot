use super::IntegrityKind;
use super::detect_integrity;
use super::tests::{detect, kinds, markers};

#[test]
fn ts_suppressions_are_reported_in_non_test_files() {
    let before = "export function load(id: string) {\n  return fetchUser(id);\n}\n";
    let after = "export function load(id: string) {\n  // @ts-ignore: upstream types are wrong\n  return fetchUser(id as any);\n}\n";
    let signals = detect("src/load.ts", "TypeScript", Some(before), Some(after));
    assert_eq!(kinds(&signals), vec![IntegrityKind::SuppressionAdded]);
    assert_eq!(signals[0].detail, "`@ts-ignore` added (0 → 1 in this file)");
    assert_eq!(signals[0].line, 2);
}

#[test]
fn eslint_rules_are_part_of_the_label() {
    let before = "const a = 1;\n";
    let after = "// eslint-disable-next-line no-console\nconsole.log(a);\nconst a = 1;\n";
    let signals = detect("src/log.js", "JavaScript", Some(before), Some(after));
    assert_eq!(
        signals[0].detail,
        "`eslint-disable-next-line no-console` added (0 → 1 in this file)"
    );
}

#[test]
fn python_go_and_rust_suppressions_are_recognized() {
    let py = detect(
        "app/models.py",
        "Python",
        Some("x = load()\n"),
        Some("x = load()  # type: ignore[attr-defined]\ny = 1  # noqa: E501\n"),
    );
    let details: Vec<&str> = py.iter().map(|signal| signal.detail.as_str()).collect();
    assert!(
        details.contains(&"`type: ignore[attr-defined]` added (0 → 1 in this file)"),
        "{details:?}"
    );
    assert!(
        details.contains(&"`noqa: E501` added (0 → 1 in this file)"),
        "{details:?}"
    );

    let go = detect(
        "pkg/io.go",
        "Go",
        Some("package io\n\nfunc Close() { f.Close() }\n"),
        Some("package io\n\nfunc Close() { f.Close() //nolint:errcheck\n}\n"),
    );
    assert_eq!(go[0].detail, "`nolint:errcheck` added (0 → 1 in this file)");

    let rust = detect(
        "src/lib.rs",
        "Rust",
        Some("pub fn parse() -> u8 { 1 }\n"),
        Some("#[allow(clippy::unwrap_used)]\npub fn parse() -> u8 { 1 }\n"),
    );
    assert_eq!(
        rust[0].detail,
        "`#[allow(clippy::unwrap_used)]` added (0 → 1 in this file)"
    );
}

#[test]
fn suppression_text_in_strings_and_unchanged_suppressions_are_quiet() {
    let before = "// @ts-expect-error legacy\nconst a: number = legacy();\n";
    let after = "// @ts-expect-error legacy\nconst a: number = legacy();\nconst note = \"@ts-ignore is banned\";\n";
    assert!(detect("src/a.ts", "TypeScript", Some(before), Some(after)).is_empty());
}

#[test]
fn a_suppression_moved_with_its_code_is_not_new() {
    let with = "// eslint-disable-next-line no-console\nconsole.log(1);\n";
    let files = [
        markers("src/a.js", "JavaScript", Some(with), Some("")).unwrap(),
        markers("src/b.js", "JavaScript", Some(""), Some(with)).unwrap(),
    ];
    assert!(detect_integrity(&files).is_empty());
}
