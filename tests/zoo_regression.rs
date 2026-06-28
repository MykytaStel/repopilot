//! Real-repo validation zoo regression gate.
//!
//! This is an OPT-IN integration test: it is skipped unless `REPOPILOT_ZOO=1`
//! is set, because it requires the curated repos to be cloned locally (network)
//! into `.zoo/` via `python3 scripts/zoo.py clone`. Normal `cargo test --all`
//! and CI runs treat it as a no-op so the default suite stays hermetic.
//!
//! When enabled, it re-scans every cloned repo and diffs the result against the
//! committed snapshots under `tests/zoo/snapshots/`. Any drift in default-visible
//! findings (counts or fingerprints) fails — the tripwire that catches when a
//! rule change silently shifts real-world output. Accept intended changes with
//! `python3 scripts/zoo.py scan --bless`.
//!
//! Run it:
//!     python3 scripts/zoo.py clone
//!     REPOPILOT_ZOO=1 cargo test --test zoo_regression
//!
//! Bless after an intended change:
//!     REPOPILOT_ZOO_BLESS=1 cargo test --test zoo_regression

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn zoo_snapshots_match_committed() {
    if env::var("REPOPILOT_ZOO").as_deref() != Ok("1")
        && env::var("REPOPILOT_ZOO_BLESS").as_deref() != Ok("1")
    {
        eprintln!(
            "skipping zoo regression (set REPOPILOT_ZOO=1 and run \
             `python3 scripts/zoo.py clone` first)"
        );
        return;
    }

    let root = repo_root();
    let bless = env::var("REPOPILOT_ZOO_BLESS").as_deref() == Ok("1");

    let mut cmd = Command::new("python3");
    cmd.arg("scripts/zoo.py").arg("scan").current_dir(&root);
    if bless {
        cmd.arg("--bless");
    }

    let output = cmd
        .output()
        .expect("failed to run scripts/zoo.py scan (is python3 on PATH?)");

    // Surface the triage output regardless of outcome.
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let failure_kind =
        if output.status.code() == Some(2) || stderr.contains("SCANNER PREPARATION FAILED") {
            "scanner build/resolution failure"
        } else if output.status.code() == Some(3) || stderr.contains("SCANNER PROVENANCE FAILED") {
            "scanner provenance/version failure"
        } else if stdout.contains("DRIFT vs committed snapshot") {
            "real snapshot drift"
        } else {
            "zoo scan failure"
        };

    assert!(
        output.status.success(),
        "zoo regression failed ({failure_kind}). Review the output above; \
         if the change is intended, re-bless with \
         `python3 scripts/zoo.py scan --bless`."
    );
}
