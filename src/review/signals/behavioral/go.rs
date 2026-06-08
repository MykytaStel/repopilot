use crate::review::signals::behavioral::{
    BehavioralKind, BehavioralSignal, BehavioralSignalSource, DependencyContext,
    extract_string_literal, truncate_str,
};
use tree_sitter::Node;

pub(super) fn match_go(
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

            if func_text.starts_with("http.") || func_text == "net.Dial" {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::NetworkCallAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if func_text.starts_with("exec.Command") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::SubprocessAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if func_text.starts_with("os.WriteFile")
                || func_text.starts_with("os.Create")
                || func_text.starts_with("os.OpenFile")
                || func_text.starts_with("ioutil.WriteFile")
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::FsWriteAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if func_text == "os.Getenv" || func_text == "os.LookupEnv" {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::EnvVarIntroduced,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if func_text.ends_with(".Query")
                || func_text.ends_with(".QueryRow")
                || func_text.ends_with(".Exec")
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::RawSqlAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
        }
        "import_spec" => {
            if let Some(path_node) = node.child_by_field_name("path") {
                let path_text = path_node.utf8_text(content.as_bytes()).ok()?.trim();
                if let Some(val) = extract_string_literal(path_text)
                    && val.contains('.')
                    && !dependencies.is_local_package(val)
                {
                    return Some(BehavioralSignal {
                        kind: BehavioralKind::DependencyImportAdded,
                        path: path_str.to_string(),
                        line,
                        detail: format!("Imported package '{val}'"),
                        source: BehavioralSignalSource::Ast,
                    });
                }
            }
        }
        _ => {}
    }
    None
}
