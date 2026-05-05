use repopilot::scan::language::detect_language;
use std::path::Path;

#[test]
fn detects_common_language_extensions() {
    assert_eq!(detect_language(Path::new("src/main.rs")), Some("Rust"));
    assert_eq!(
        detect_language(Path::new("src/App.tsx")),
        Some("TypeScript React")
    );
    assert_eq!(
        detect_language(Path::new("scripts/build.py")),
        Some("Python")
    );
    assert_eq!(detect_language(Path::new("README.md")), Some("Markdown"));
}

#[test]
fn returns_none_for_unknown_extensions() {
    assert_eq!(detect_language(Path::new("file.unknown")), None);
}
