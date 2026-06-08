use crate::review::signals::behavioral::{
    BehavioralKind, BehavioralSignal, BehavioralSignalSource, DependencyContext, truncate_str,
};
use tree_sitter::Node;

pub(super) fn match_csharp(
    node: Node<'_>,
    content: &str,
    path_str: &str,
    line: usize,
    dependencies: &DependencyContext,
) -> Option<BehavioralSignal> {
    let text = node.utf8_text(content.as_bytes()).unwrap_or("").trim();
    match node.kind() {
        "invocation_expression" | "object_creation_expression" => {
            if text.contains("HttpClient") || text.contains("HttpWebRequest") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::NetworkCallAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if text.contains("Process.Start") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::SubprocessAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if text.contains("File.WriteAll")
                || text.contains("File.OpenWrite")
                || text.contains("StreamWriter")
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::FsWriteAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if text.contains("Environment.GetEnvironmentVariable") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::EnvVarIntroduced,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
        }
        "using_directive" => {
            let imported = text
                .trim_start_matches("using ")
                .trim_end_matches(';')
                .trim();
            if imported.starts_with("System") || dependencies.is_local_package(imported) {
                return None;
            }
            return Some(BehavioralSignal {
                kind: BehavioralKind::DependencyImportAdded,
                path: path_str.to_string(),
                line,
                detail: format!("Imported using namespace: {}", text.trim_end_matches(';')),
                source: BehavioralSignalSource::Ast,
            });
        }
        _ => {}
    }
    None
}
