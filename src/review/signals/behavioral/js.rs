use crate::review::signals::behavioral::{
    BehavioralKind, BehavioralSignal, extract_string_literal, is_local_import, truncate_str,
};
use tree_sitter::Node;

pub(super) fn match_js(
    node: Node<'_>,
    content: &str,
    path_str: &str,
    line: usize,
) -> Option<BehavioralSignal> {
    match node.kind() {
        "call_expression" => {
            let callee = node.child_by_field_name("function")?;
            let callee_text = callee.utf8_text(content.as_bytes()).ok()?.trim();
            let text = node.utf8_text(content.as_bytes()).unwrap_or("");

            if callee_text == "fetch"
                || callee_text.starts_with("axios")
                || callee_text.starts_with("http.request")
                || callee_text.starts_with("https.request")
                || callee_text.starts_with("http.get")
                || callee_text.starts_with("https.get")
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::NetworkCallAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                });
            }
            if callee_text == "exec"
                || callee_text == "spawn"
                || callee_text == "fork"
                || callee_text.starts_with("child_process.")
                || callee_text.contains("execSync")
                || callee_text.contains("spawnSync")
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::SubprocessAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                });
            }
            if callee_text.contains("writeFile")
                || callee_text.contains("writeFileSync")
                || callee_text.contains("appendFile")
                || callee_text.contains("appendFileSync")
                || callee_text.contains("createWriteStream")
                || (callee_text.starts_with("fs.")
                    && (callee_text.contains("write") || callee_text.contains("append")))
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::FsWriteAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                });
            }
            if callee_text == "require" || callee_text == "import" {
                if let Some(args) = node.child_by_field_name("arguments") {
                    let arg_text = args.utf8_text(content.as_bytes()).ok()?;
                    if let Some(val) = extract_string_literal(arg_text.trim()) {
                        if !is_local_import(val) {
                            return Some(BehavioralSignal {
                                kind: BehavioralKind::DependencyImportAdded,
                                path: path_str.to_string(),
                                line,
                                detail: format!("Imported dependency '{val}'"),
                            });
                        }
                    }
                }
            }
            if callee_text.ends_with(".query")
                || callee_text == "query"
                || callee_text.ends_with(".execute")
                || callee_text == "execute"
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::RawSqlAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                });
            }
        }
        "new_expression" => {
            let constructor = node.child_by_field_name("constructor")?;
            let name = constructor.utf8_text(content.as_bytes()).ok()?.trim();
            if name == "WebSocket" {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::NetworkCallAdded,
                    path: path_str.to_string(),
                    line,
                    detail: "new WebSocket(...)".to_string(),
                });
            }
        }
        "member_expression" => {
            let text = node.utf8_text(content.as_bytes()).ok()?.trim();
            if text.starts_with("process.env") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::EnvVarIntroduced,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                });
            }
        }
        "import_statement" | "export_statement" => {
            if let Some(source) = node.child_by_field_name("source") {
                let text = source.utf8_text(content.as_bytes()).ok()?.trim();
                if let Some(val) = extract_string_literal(text) {
                    if !is_local_import(val) {
                        return Some(BehavioralSignal {
                            kind: BehavioralKind::DependencyImportAdded,
                            path: path_str.to_string(),
                            line,
                            detail: format!("Imported dependency '{val}'"),
                        });
                    }
                }
            }
        }
        _ => {}
    }
    None
}
