//! Drift guard for the published rules catalog.
//!
//! `docs/rules-reference.md` is generated from the rule registry
//! (`render_rules_reference_markdown`). This test re-renders it in memory and
//! compares against the committed file so the docs can never fall behind the
//! code that emits findings. It runs under the ordinary `cargo test --all`
//! gate, so CI and `verify-release.sh` catch drift without any extra scripts.
//!
//! Regenerate after a registry change with:
//!   `REPOPILOT_BLESS=1 cargo test --test rules_reference_doc`

use repopilot::rules::render_rules_reference_markdown;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn rules_reference_doc_matches_registry() {
    let rendered = render_rules_reference_markdown();
    let path = doc_path();

    if std::env::var_os("REPOPILOT_BLESS").is_some() {
        fs::write(&path, &rendered).expect("failed to write docs/rules-reference.md");
        return;
    }

    let committed = fs::read_to_string(&path)
        .expect("docs/rules-reference.md is missing; run `REPOPILOT_BLESS=1 cargo test --test rules_reference_doc`")
        .replace("\r\n", "\n");

    assert_eq!(
        rendered, committed,
        "docs/rules-reference.md is out of date with the rule registry; \
regenerate with `REPOPILOT_BLESS=1 cargo test --test rules_reference_doc`"
    );
}

fn doc_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("rules-reference.md")
}
