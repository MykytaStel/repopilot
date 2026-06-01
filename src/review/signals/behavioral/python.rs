use crate::review::signals::behavioral::{BehavioralKind, BehavioralSignal, truncate_str};
use tree_sitter::Node;

pub(super) fn match_python(
    node: Node<'_>,
    content: &str,
    path_str: &str,
    line: usize,
) -> Option<BehavioralSignal> {
    match node.kind() {
        "call" => {
            let callee = node.child_by_field_name("function")?;
            let callee_text = callee.utf8_text(content.as_bytes()).ok()?.trim();
            let text = node.utf8_text(content.as_bytes()).unwrap_or("");

            if callee_text.starts_with("requests.")
                || callee_text.starts_with("urllib.")
                || callee_text.starts_with("httpx.")
                || callee_text == "urlopen"
                || callee_text == "socket.connect"
                || callee_text.starts_with("aiohttp.")
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::NetworkCallAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                });
            }
            if callee_text.starts_with("subprocess.")
                || callee_text == "os.system"
                || callee_text == "os.popen"
                || callee_text == "sh.run"
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::SubprocessAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                });
            }
            if callee_text.starts_with("pathlib.") && callee_text.contains("write") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::FsWriteAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                });
            }
            if callee_text == "open" {
                if let Some(args) = node.child_by_field_name("arguments") {
                    let args_text = args.utf8_text(content.as_bytes()).unwrap_or("");
                    if args_text.contains("'w'")
                        || args_text.contains("\"w\"")
                        || args_text.contains("'wb'")
                        || args_text.contains("\"wb\"")
                        || args_text.contains("'a'")
                        || args_text.contains("\"a\"")
                        || args_text.contains("'ab'")
                        || args_text.contains("\"ab\"")
                    {
                        return Some(BehavioralSignal {
                            kind: BehavioralKind::FsWriteAdded,
                            path: path_str.to_string(),
                            line,
                            detail: truncate_str(text, 60),
                        });
                    }
                }
            }
            if callee_text.ends_with(".execute") || callee_text.ends_with(".query") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::RawSqlAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                });
            }
        }
        "attribute" | "subscript" => {
            let text = node.utf8_text(content.as_bytes()).ok()?.trim();
            if text == "os.environ"
                || text.starts_with("os.environ[")
                || text.starts_with("os.environ.get")
                || text.starts_with("os.getenv")
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::EnvVarIntroduced,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                });
            }
        }
        "import_statement" | "import_from_statement" => {
            let text = node.utf8_text(content.as_bytes()).ok()?.trim();
            if !text.contains("import .") && !text.starts_with("from .") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::DependencyImportAdded,
                    path: path_str.to_string(),
                    line,
                    detail: format!("Imported module: {text}"),
                });
            }
        }
        _ => {}
    }
    None
}
