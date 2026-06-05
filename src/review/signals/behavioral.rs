//! Behavioral "added X" change signals.
//!
//! Walk the post-change AST to detect added calls or directives (network calls,
//! subprocess execution, filesystem writes, environment variables, new dependency
//! imports, and raw SQL queries) whose start lines fall within the changed ranges.
//! Also handles path-based migration detection for newly added files.

mod csharp;
mod go;
mod js;
mod jvm;
mod keywords;
mod python;
mod removed;
mod rust;

pub use removed::detect_behavioral_removed;

use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::signals::content::ReviewSource;
use serde::Serialize;
use tree_sitter::Node;

/// The category of behavioral change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BehavioralKind {
    NetworkCallAdded,
    SubprocessAdded,
    FsWriteAdded,
    EnvVarIntroduced,
    DependencyImportAdded,
    MigrationAdded,
    RawSqlAdded,
    ErrorHandlingRemoved,
    TestDeletedOrEmptied,
    AuthCheckRemoved,
}

/// How a behavioral signal was detected. Confidence — and therefore tiering —
/// keys off this structured source, never off the user-facing `detail` text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BehavioralSignalSource {
    /// Matched structurally against the parsed syntax tree.
    Ast,
    /// Matched by scanning raw diff lines because the file could not be parsed.
    CoarseFallback,
}

/// A behavioral signal detected in a changed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BehavioralSignal {
    pub kind: BehavioralKind,
    pub path: String,
    pub line: usize,
    pub detail: String,
    pub source: BehavioralSignalSource,
}

impl BehavioralSignal {
    /// Whether this signal came from the coarse (non-AST) fallback, which only
    /// runs when the file can't be parsed. Coarse signals are hints, not
    /// confident findings, and are demoted in the tiered view.
    pub fn is_coarse(&self) -> bool {
        self.source == BehavioralSignalSource::CoarseFallback
    }
}

/// Detects newly added behavioral signals in a post-change source.
pub fn detect_behavioral_added(
    file: &ChangedFile,
    post_source: &ReviewSource,
) -> Vec<BehavioralSignal> {
    let mut signals = Vec::new();

    // 1. Path-based MigrationAdded detection (only if status is Added)
    if file.status == ChangeStatus::Added && is_migration_path(&file.path_string()) {
        signals.push(BehavioralSignal {
            kind: BehavioralKind::MigrationAdded,
            path: file.path_string(),
            line: 1,
            detail: "New database migration script added".to_string(),
            source: BehavioralSignalSource::Ast,
        });
    }

    let parsed = post_source.parsed();
    let Some(tree) = parsed.tree() else {
        return signals;
    };

    let content = post_source.content();
    let path_str = file.path_string();
    let ext = file.path.extension().and_then(|e| e.to_str()).unwrap_or("");

    walk_node(
        tree.root_node(),
        content,
        file,
        ext,
        &path_str,
        &mut signals,
    );

    // Deduplicate signals of the same kind on the same line to avoid noise from nested AST nodes
    let mut unique = Vec::new();
    for sig in signals {
        if !unique.iter().any(|existing: &BehavioralSignal| {
            existing.kind == sig.kind && existing.line == sig.line
        }) {
            unique.push(sig);
        }
    }
    unique
}

fn is_migration_path(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    path_lower.contains("/migrations/")
        || path_lower.contains("/migrate/")
        || path_lower.contains("/sql/")
        || path_lower.contains("/db/upgrade")
        || path_lower.contains("/database/upgrade")
        || (path_lower.contains("/db/") && path_lower.contains("migration"))
}

fn walk_node(
    node: Node<'_>,
    content: &str,
    file: &ChangedFile,
    ext: &str,
    path_str: &str,
    signals: &mut Vec<BehavioralSignal>,
) {
    let line = node.start_position().row + 1;
    if file.contains_line(line) {
        if let Some(signal) = match_node(node, content, ext, path_str, line) {
            signals.push(signal);
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(child, content, file, ext, path_str, signals);
    }
}

fn match_node(
    node: Node<'_>,
    content: &str,
    ext: &str,
    path_str: &str,
    line: usize,
) -> Option<BehavioralSignal> {
    let kind = node.kind();

    // Check Raw SQL (common across all languages)
    let is_string = kind.contains("string") || kind == "character_literal";
    if is_string {
        if let Ok(text) = node.utf8_text(content.as_bytes()) {
            let unquoted = extract_string_literal(text).unwrap_or(text);
            if is_sql_query(unquoted) {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::RawSqlAdded,
                    path: path_str.to_string(),
                    line,
                    detail: format!("Raw SQL query: {}", truncate_str(unquoted, 60)),
                    source: BehavioralSignalSource::Ast,
                });
            }
        }
    }

    match ext {
        "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "tsx" | "jsx" => {
            js::match_js(node, content, path_str, line)
        }
        "py" => python::match_python(node, content, path_str, line),
        "go" => go::match_go(node, content, path_str, line),
        "rs" => rust::match_rust(node, content, path_str, line),
        "java" | "kt" | "kts" => jvm::match_jvm(node, content, path_str, line),
        "cs" => csharp::match_csharp(node, content, path_str, line),
        _ => None,
    }
}

pub(crate) fn extract_string_literal(s: &str) -> Option<&str> {
    if let Some(rest) = s.strip_prefix('"') {
        let end = rest.find('"')?;
        Some(&rest[..end])
    } else if let Some(rest) = s.strip_prefix('\'') {
        let end = rest.find('\'')?;
        Some(&rest[..end])
    } else if let Some(rest) = s.strip_prefix('`') {
        let end = rest.find('`')?;
        Some(&rest[..end])
    } else {
        None
    }
}

pub(crate) fn is_local_import(path: &str) -> bool {
    path.starts_with('.')
        || path.starts_with('/')
        || path.starts_with('~')
        || path.starts_with("@/")
}

fn is_sql_query(s: &str) -> bool {
    let s_upper = s.to_uppercase();
    (s_upper.contains("SELECT ") && s_upper.contains(" FROM "))
        || s_upper.contains("INSERT INTO ")
        || (s_upper.contains("UPDATE ") && s_upper.contains(" SET "))
        || s_upper.contains("DELETE FROM ")
        || s_upper.contains("CREATE TABLE ")
        || s_upper.contains("DROP TABLE ")
}

pub(crate) fn truncate_str(s: &str, max_len: usize) -> String {
    let s_trimmed = s.trim().replace('\n', " ");
    if s_trimmed.len() <= max_len {
        s_trimmed
    } else {
        format!("{}...", &s_trimmed[..max_len])
    }
}
