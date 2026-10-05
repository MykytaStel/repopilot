//! Entry-function probes match a top-level definition, not the words in a
//! string or a test: `parse("fn main() {}")` in RepoPilot's own parser tests
//! made `src/analysis/parse.rs` an application entrypoint.

use super::conventions::conventions_for_kind;
use crate::audits::context::model::LanguageKind;

fn probe(language: LanguageKind, content: &str) -> bool {
    conventions_for_kind(language)
        .entrypoint_content
        .is_some_and(|probe| probe(content))
}

#[test]
fn rust_main_is_a_top_level_function() {
    assert!(probe(
        LanguageKind::Rust,
        "use std::env;\n\nfn main() {\n}\n"
    ));
    assert!(probe(
        LanguageKind::Rust,
        "#[tokio::main]\nasync fn main() {\n}\n"
    ));
    assert!(probe(
        LanguageKind::Rust,
        "pub fn main() -> Result<(), Error> {\n}\n"
    ));
    assert!(!probe(
        LanguageKind::Rust,
        "#[test]\nfn parses() {\n    assert!(parse(\"fn main() {}\").is_some());\n}\n"
    ));
    // A multi-line string in a test module puts `fn main(` in column 0.
    assert!(!probe(
        LanguageKind::Rust,
        "pub fn parse() {}\n\n#[cfg(test)]\nmod tests {\n    const SOURCE: &str = \"use crate::facts;\nfn main() {}\";\n}\n"
    ));
}

#[test]
fn go_main_is_a_top_level_function() {
    assert!(probe(
        LanguageKind::Go,
        "package main\n\nfunc main() {\n}\n"
    ));
    assert!(!probe(LanguageKind::Go, "const src = `func main() {}`\n"));
}

#[test]
fn python_main_guard_is_a_top_level_statement() {
    assert!(probe(
        LanguageKind::Python,
        "def run():\n    pass\n\nif __name__ == \"__main__\":\n    run()\n"
    ));
    assert!(probe(
        LanguageKind::Python,
        "if __name__ == '__main__':\n    run()\n"
    ));
    assert!(!probe(
        LanguageKind::Python,
        "TEMPLATE = 'if __name__ == \"__main__\": main()'\n"
    ));
}
