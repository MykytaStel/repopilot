//! Behavioral "added X" change signals.
//!
//! Walk the post-change AST to detect added calls or directives (network calls,
//! subprocess execution, filesystem writes, environment variables, new dependency
//! imports, and raw SQL queries) whose start lines fall within the changed ranges.
//! Also handles path-based migration detection for newly added files.

mod csharp;
mod dependency;
mod go;
mod js;
mod jvm;
mod keywords;
mod python;
mod removed;
mod removed_ast;
mod rust;

pub use removed::detect_behavioral_removed;

use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::signals::content::ReviewSource;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;
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

#[derive(Debug, Default)]
pub struct DependencyContext {
    local_package_names: BTreeSet<String>,
    local_import_prefixes: BTreeSet<String>,
}

impl DependencyContext {
    /// Build the set of names/prefixes that count as *local* — the repo's own
    /// package(s) and, in a monorepo, its workspace members — so cross-package
    /// imports are not mistaken for newly added external dependencies.
    pub fn from_repo_root(root: &Path) -> Self {
        let mut context = Self::default();

        for name in dependency::cargo_local_names(root) {
            context.add_local_name(&name);
        }
        for name in dependency::npm_local_names(root) {
            context.add_local_name(&name);
        }
        for prefix in dependency::go_local_prefixes(root) {
            context.local_import_prefixes.insert(prefix);
        }

        context
    }

    fn add_local_name(&mut self, name: &str) {
        let normalized = name.trim().trim_start_matches('@');
        if normalized.is_empty() {
            return;
        }
        // Match the full package name (and its `-`→`_` spelling). The bare last
        // path segment is intentionally *not* inserted: a repo named `@acme/core`
        // must not swallow an unrelated bare `core` import.
        self.local_package_names.insert(normalized.to_string());
        self.local_package_names
            .insert(normalized.replace('-', "_"));
    }

    pub(super) fn is_local_package(&self, name: &str) -> bool {
        let normalized = name.trim().trim_start_matches('@');
        let root = normalized
            .split(['/', ':', '.'])
            .next()
            .unwrap_or(normalized);
        self.local_package_names.contains(normalized)
            || self.local_package_names.contains(root)
            || self
                .local_import_prefixes
                .iter()
                .any(|prefix| normalized == prefix || normalized.starts_with(&format!("{prefix}/")))
    }
}

/// Detects newly added behavioral signals in a post-change source.
pub fn detect_behavioral_added(
    file: &ChangedFile,
    post_source: &ReviewSource,
    dependencies: &DependencyContext,
) -> Vec<BehavioralSignal> {
    let mut signals = Vec::new();

    if crate::audits::context::classify::helpers::is_test_file(&file.path, false) {
        return signals;
    }

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
        dependencies,
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
    dependencies: &DependencyContext,
    signals: &mut Vec<BehavioralSignal>,
) {
    let line = node.start_position().row + 1;
    if file.contains_line(line)
        && let Some(signal) = match_node(node, content, ext, path_str, line, dependencies)
    {
        signals.push(signal);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(child, content, file, ext, path_str, dependencies, signals);
    }
}

fn match_node(
    node: Node<'_>,
    content: &str,
    ext: &str,
    path_str: &str,
    line: usize,
    dependencies: &DependencyContext,
) -> Option<BehavioralSignal> {
    let kind = node.kind();

    // Check Raw SQL (common across all languages)
    let is_string = kind.contains("string") || kind == "character_literal";
    if is_string && let Ok(text) = node.utf8_text(content.as_bytes()) {
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

    match ext {
        "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "tsx" | "jsx" => {
            js::match_js(node, content, path_str, line, dependencies)
        }
        "py" => python::match_python(node, content, path_str, line, dependencies),
        "go" => go::match_go(node, content, path_str, line, dependencies),
        "rs" => rust::match_rust(node, content, path_str, line, dependencies),
        "java" | "kt" | "kts" => jvm::match_jvm(node, content, path_str, line, dependencies),
        "cs" => csharp::match_csharp(node, content, path_str, line, dependencies),
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
