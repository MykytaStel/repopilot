//! Rust facade detection for the excessive-fan-out rule. A `lib.rs`/`mod.rs`
//! whose body is only `mod`/`use`/`pub use`/`extern crate` declarations is an
//! intentional re-export barrel, not change-amplifying coupling, so it is
//! exempt from the fan-out finding.

use crate::graph::FileMetrics;
use crate::scan::facts::ScanFacts;
use std::path::Path;

pub(super) fn is_pure_rust_facade(metric: &FileMetrics, facts: &ScanFacts, root: &Path) -> bool {
    let Some(file) = facts
        .files
        .iter()
        .find(|file| file.path == metric.path || root.join(&file.path) == metric.path)
    else {
        return false;
    };

    if file.language.as_deref() != Some("Rust") || !is_rust_facade_filename(&file.path) {
        return false;
    }

    let content = file
        .content
        .clone()
        .or_else(|| std::fs::read_to_string(root.join(&file.path)).ok())
        .or_else(|| std::fs::read_to_string(&file.path).ok());
    let Some(content) = content else {
        return false;
    };

    let mut saw_facade_declaration = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("#[")
            || trimmed.starts_with("//!")
            || trimmed.starts_with("///")
        {
            continue;
        }

        if is_rust_facade_declaration(trimmed) {
            saw_facade_declaration = true;
            continue;
        }

        return false;
    }

    saw_facade_declaration
}

fn is_rust_facade_filename(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "lib.rs" | "mod.rs"))
}

fn is_rust_facade_declaration(line: &str) -> bool {
    let line = line.strip_suffix(';').unwrap_or(line).trim();
    let line = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub(super) "))
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line);

    line.starts_with("mod ")
        || line.starts_with("use ")
        || line.starts_with("extern crate ")
        || line.starts_with("pub use ")
}
