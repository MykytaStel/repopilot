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
            let path = text
                .trim_start_matches("pub ")
                .trim_start_matches("use ")
                .trim();
            let root = path
                .split("::")
                .next()
                .unwrap_or(path)
                .trim_end_matches(';');
            if !path.starts_with("crate::")
                && !path.starts_with("self::")
                && !path.starts_with("super::")
                && !matches!(root, "std" | "core" | "alloc")
                && !dependencies.is_local_package(root)
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
