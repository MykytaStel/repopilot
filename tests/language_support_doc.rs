//! Drift guard for the published language support matrix.
//!
//! `docs/language-support.md` is generated from the language frontend
//! registry (`render_language_support_markdown`). This test re-renders it in
//! memory and compares against the committed file so the support matrix can
//! never fall behind (or ahead of) the wiring. Runs under the ordinary
//! `cargo test --all` gate.
//!
//! Regenerate after a registry change with:
//!   `REPOPILOT_BLESS=1 cargo test --test language_support_doc`

use repopilot::languages::render_language_support_markdown;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn language_support_doc_matches_registry() {
    let rendered = render_language_support_markdown();
    let path = doc_path();

    if std::env::var_os("REPOPILOT_BLESS").is_some() {
        fs::write(&path, &rendered).expect("failed to write docs/language-support.md");
        return;
    }

    let committed = fs::read_to_string(&path)
        .expect("docs/language-support.md is missing; run `REPOPILOT_BLESS=1 cargo test --test language_support_doc`")
        .replace("\r\n", "\n");

    assert_eq!(
        rendered, committed,
        "docs/language-support.md is out of date with the language frontend registry; \
regenerate with `REPOPILOT_BLESS=1 cargo test --test language_support_doc`"
    );
}

fn doc_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("language-support.md")
}
