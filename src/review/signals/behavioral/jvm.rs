use crate::review::signals::behavioral::{
    BehavioralKind, BehavioralSignal, BehavioralSignalSource, DependencyContext, truncate_str,
};
use tree_sitter::Node;

pub(super) fn match_jvm(
    node: Node<'_>,
    content: &str,
    path_str: &str,
    line: usize,
    dependencies: &DependencyContext,
) -> Option<BehavioralSignal> {
    let text = node.utf8_text(content.as_bytes()).unwrap_or("").trim();
    match node.kind() {
        "method_invocation" | "call_expression" | "object_creation_expression" => {
            if text.contains("HttpClient")
                || text.contains("OkHttpClient")
                || text.contains("HttpURLConnection")
                || text.contains("openConnection")
                || text.contains("Retrofit")
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::NetworkCallAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if text.contains("ProcessBuilder") || text.contains(".exec(") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::SubprocessAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if text.contains("FileWriter")
                || text.contains("FileOutputStream")
                || text.contains("Files.write")
                || text.contains("PrintWriter")
            {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::FsWriteAdded,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if text.contains("System.getenv") {
                return Some(BehavioralSignal {
                    kind: BehavioralKind::EnvVarIntroduced,
                    path: path_str.to_string(),
                    line,
                    detail: truncate_str(text, 60),
                    source: BehavioralSignalSource::Ast,
                });
            }
            if text.contains("executeQuery")
                || text.contains("executeUpdate")
                || text.contains("execute(")
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
        "import_declaration" => {
            let imported = text
                .trim_start_matches("import ")
                .trim_start_matches("static ")
                .trim_end_matches(';');
            if imported.starts_with("java.")
                || imported.starts_with("javax.")
                || imported.starts_with("kotlin.")
                || dependencies.is_local_package(imported)
            {
                return None;
            }
            return Some(BehavioralSignal {
                kind: BehavioralKind::DependencyImportAdded,
                path: path_str.to_string(),
                line,
                detail: format!("Imported package: {}", text.trim_end_matches(';')),
                source: BehavioralSignalSource::Ast,
            });
        }
        _ => {}
    }
    None
}
