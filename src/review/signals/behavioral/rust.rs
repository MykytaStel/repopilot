use crate::review::signals::behavioral::{BehavioralKind, BehavioralSignal, truncate_str};
use tree_sitter::Node;

pub(super) fn match_rust(
    node: Node<'_>,
    content: &str,
    path_str: &str,
    line: usize,
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
                });
            }
            if func_text.contains("Command::new") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::SubprocessAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
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
                });
            }
            if func_text.contains("env::var") || func_text.contains("std::env::var") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::EnvVarIntroduced,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
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
                });
            }
        }
        "use_declaration" => {
            let text = node.utf8_text(content.as_bytes()).ok()?.trim();
            let path = text
                .trim_start_matches("pub ")
                .trim_start_matches("use ")
                .trim();
            if !path.starts_with("crate::")
                && !path.starts_with("self::")
                && !path.starts_with("super::")
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::DependencyImportAdded,
                    path: path_str.to_string(),
                    line,
                    detail: format!("Imported Rust crate/module: {}", text.trim_end_matches(';')),
                });
            }
        }
        _ => {}
    }
    None
}
