use crate::review::signals::behavioral::{
    BehavioralKind, BehavioralSignal, BehavioralSignalSource, DependencyContext, truncate_str,
};
use tree_sitter::Node;

pub(super) fn match_rust(
    node: Node<'_>,
    content: &str,
    path_str: &str,
    line: usize,
    dependencies: &DependencyContext,
) -> Option<BehavioralSignal> {
    match node.kind() {
        "call_expression" => {
            let function = node.child_by_field_name("function")?;
            let func_text = function.utf8_text(content.as_bytes()).ok()?.trim();
            let text = node.utf8_text(content.as_bytes()).unwrap_or("");

            if func_text.contains("reqwest::") || func_text == "TcpStream::connect" {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::NetworkCallAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if func_text.contains("Command::new") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::SubprocessAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if func_text.contains("fs::write")
                || func_text.contains("File::create")
                || func_text.contains("OpenOptions::new")
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::FsWriteAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if func_text.contains("env::var") || func_text.contains("std::env::var") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::EnvVarIntroduced,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
        }
        "macro_invocation" => {
            let macro_name = node
                .child(0)
                .and_then(|n| n.utf8_text(content.as_bytes()).ok())
                .unwrap_or("")
                .trim();
            let text = node.utf8_text(content.as_bytes()).unwrap_or("");
            if macro_name == "query" || macro_name == "sql" || macro_name == "query_as" {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::RawSqlAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
        }
        "use_declaration" => {
            let text = node.utf8_text(content.as_bytes()).ok()?.trim();
            let path = strip_rust_use_prefix(text);
            let root = path
                .split("::")
                .next()
                .unwrap_or(path)
                .trim_end_matches(';');
            if !path.starts_with("crate::")
                && !path.starts_with("self::")
                && !path.starts_with("super::")
                && !matches!(root, "std" | "core" | "alloc")
                && !dependencies.is_known_package(root)
                && !module_declared_in_scope(node, content, root)
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::DependencyImportAdded,
                    path: path_str.to_string(),
                    line,
                    detail: format!("Imported Rust crate/module: {}", text.trim_end_matches(';')),
                    source: BehavioralSignalSource::Ast,
                });
            }
        }
        _ => {}
    }
    None
}

fn strip_rust_use_prefix(text: &str) -> &str {
    let mut rest = text.trim();
    if let Some(after_pub) = rest.strip_prefix("pub") {
        let trimmed = after_pub.trim_start();
        if let Some(after_paren) = trimmed.strip_prefix('(') {
            if let Some((_, after_vis)) = after_paren.split_once(')') {
                rest = after_vis.trim_start();
            }
        } else if after_pub.starts_with(char::is_whitespace) {
            rest = trimmed;
        }
    }
    rest.strip_prefix("use")
        .filter(|after| after.starts_with(char::is_whitespace))
        .map_or(rest, str::trim_start)
}

fn module_declared_in_scope(node: Node<'_>, content: &str, root: &str) -> bool {
    let Some(scope) = node.parent() else {
        return false;
    };
    (0..scope.named_child_count()).any(|index| {
        let Ok(index) = u32::try_from(index) else {
            return false;
        };
        let Some(child) = scope.named_child(index) else {
            return false;
        };
        if child.kind() != "mod_item" {
            return false;
        }
        child
            .child_by_field_name("name")
            .and_then(|name| name.utf8_text(content.as_bytes()).ok())
            .is_some_and(|name| name == root)
    })
}

#[cfg(test)]
mod tests {
    use super::strip_rust_use_prefix;
    use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
    use crate::review::signals::behavioral::{
        BehavioralKind, DependencyContext, detect_behavioral_added,
    };
    use crate::review::signals::content::ReviewSource;
    use std::path::PathBuf;

    #[test]
    fn strip_rust_use_prefix_handles_restricted_visibility() {
        assert_eq!(strip_rust_use_prefix("use foo::bar;"), "foo::bar;");
        assert_eq!(strip_rust_use_prefix("pub use foo::bar;"), "foo::bar;");
        assert_eq!(
            strip_rust_use_prefix("pub(crate) use python::resolve;"),
            "python::resolve;"
        );
        assert_eq!(
            strip_rust_use_prefix("pub(super) use tables::JS_FAMILY;"),
            "tables::JS_FAMILY;"
        );
        assert_eq!(
            strip_rust_use_prefix("pub(in crate::review) use crate::graph::Node;"),
            "crate::graph::Node;"
        );
    }

    #[test]
    fn restricted_pub_use_of_local_module_is_not_flagged_as_external_dependency() {
        let code =
            "mod python;\npub(crate) use python::resolve;\npub(super) use crate::graph::Node;\n";
        let file = ChangedFile {
            path: PathBuf::from("src/mod.rs"),
            status: ChangeStatus::Added,
            ranges: vec![ChangedRange { start: 1, end: 3 }],
            hunks: Vec::new(),
        };
        let source = ReviewSource::new(code.to_string(), Some("Rust".to_string()));
        let signals = detect_behavioral_added(&file, &source, &DependencyContext::default());
        assert!(
            !signals
                .iter()
                .any(|signal| signal.kind == BehavioralKind::DependencyImportAdded),
            "internal pub(crate)/pub(super) use must not be flagged: {signals:?}"
        );
    }

    #[test]
    fn restricted_pub_use_of_external_crate_is_still_flagged() {
        let code = "pub(crate) use reqwest::Client;\n";
        let file = ChangedFile {
            path: PathBuf::from("src/client.rs"),
            status: ChangeStatus::Added,
            ranges: vec![ChangedRange { start: 1, end: 1 }],
            hunks: Vec::new(),
        };
        let source = ReviewSource::new(code.to_string(), Some("Rust".to_string()));
        let signals = detect_behavioral_added(&file, &source, &DependencyContext::default());
        assert!(
            signals
                .iter()
                .any(|signal| signal.kind == BehavioralKind::DependencyImportAdded),
            "external crate re-export must still be flagged: {signals:?}"
        );
    }
}
