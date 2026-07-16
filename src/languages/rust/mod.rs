use super::conventions::{PathConventions, TestSupportConvention};
use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

mod imports;
mod review;

use super::ImportExtractor;

static RUST_IMPORTS: ImportExtractor = ImportExtractor {
    eager: imports::eager,
    deferred: None,
    spans: imports::spans,
};

pub(super) static RUST: LanguageFrontend = LanguageFrontend {
    id: "rust",
    label: "Rust",
    kind: LanguageKind::Rust,
    knowledge_ids: &["rust"],
    grammars: &[GrammarBinding {
        label: "Rust",
        grammar: ParseLanguage::Rust,
    }],
    imports: Some(&RUST_IMPORTS),
    taint: None,
    review: Some(&review::RUST_REVIEW),
    conventions: &RUST_CONVENTIONS,
    risk: None,
};

static RUST_CONVENTIONS: PathConventions = PathConventions {
    // Only the plural forms are Rust test-file names; the singular
    // `test_*`/`*_test` collide with production modules (`test_edges.rs`).
    test_file_name: |name| name == "tests.rs" || name.ends_with("_tests.rs"),
    test_prefix_marks_test: false,
    test_support: Some(&RUST_TEST_SUPPORT),
    entrypoint_content: Some(|content| content.contains("fn main(")),
};

/// `testutil.rs`-style helpers: production modules (compiled in normal
/// builds) whose panics are assertion plumbing. Explicit allowlist keeps the
/// collision-prone `test_*` prefix out.
static RUST_TEST_SUPPORT: TestSupportConvention = TestSupportConvention {
    matches: |path| {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_lowercase())
            .unwrap_or_default();
        matches!(
            file_name.as_str(),
            "testutil.rs"
                | "testutils.rs"
                | "test_util.rs"
                | "test_utils.rs"
                | "test_support.rs"
                | "test_helper.rs"
                | "test_helpers.rs"
        )
    },
    reason: "recognized Rust test-support helper filename",
};
