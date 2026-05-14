use super::helpers::{is_js_or_ts, path_contains_component, push_unique};
use crate::audits::context::model::{FileRole, FrameworkKind, LanguageKind, ProgrammingParadigm};
use std::path::Path;

pub fn classify_paradigms(
    paradigms: &mut Vec<ProgrammingParadigm>,
    path: &Path,
    content: &str,
    language: LanguageKind,
    frameworks: &[FrameworkKind],
    roles: &[FileRole],
) {
    // All patterns here are lowercase keywords in their respective languages — no allocation needed.
    if roles.contains(&FileRole::ReactComponent) {
        push_unique(paradigms, ProgrammingParadigm::DeclarativeUi);
        push_unique(paradigms, ProgrammingParadigm::Functional);
    }

    if roles.contains(&FileRole::ReactHook) {
        push_unique(paradigms, ProgrammingParadigm::Functional);
        push_unique(paradigms, ProgrammingParadigm::Reactive);
    }

    if frameworks.contains(&FrameworkKind::Unity) {
        push_unique(paradigms, ProgrammingParadigm::ObjectOriented);
        push_unique(paradigms, ProgrammingParadigm::DataOriented);
    }

    if language == LanguageKind::CSharp
        && (content.contains("class ")
            || content.contains("interface ")
            || content.contains("record "))
    {
        push_unique(paradigms, ProgrammingParadigm::ObjectOriented);
    }

    if language == LanguageKind::Rust {
        if content.contains("impl ")
            || content.contains("trait ")
            || content.contains("struct ")
            || content.contains("enum ")
        {
            push_unique(paradigms, ProgrammingParadigm::ObjectOriented);
        }

        if content.contains(".map(")
            || content.contains(".filter(")
            || content.contains(".fold(")
            || content.contains(".and_then(")
            || content.contains(".unwrap_or_else(")
        {
            push_unique(paradigms, ProgrammingParadigm::Functional);
        }

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == "main.rs")
            .unwrap_or(false)
        {
            push_unique(paradigms, ProgrammingParadigm::Procedural);
        }
    }

    if is_js_or_ts(language) {
        if content.contains("function ")
            || content.contains("=>")
            || content.contains(".map(")
            || content.contains(".filter(")
            || content.contains(".reduce(")
        {
            push_unique(paradigms, ProgrammingParadigm::Functional);
        }

        if content.contains("class ") {
            push_unique(paradigms, ProgrammingParadigm::ObjectOriented);
        }
    }

    if matches!(language, LanguageKind::Python | LanguageKind::Go)
        && (content.contains("def main(")
            || content.contains("func main(")
            || path_contains_component(path, &["cmd", "scripts"]))
    {
        push_unique(paradigms, ProgrammingParadigm::Procedural);
    }

    if matches!(
        language,
        LanguageKind::Java | LanguageKind::Kotlin | LanguageKind::CSharp
    ) && (content.contains("class ")
        || content.contains("interface ")
        || content.contains("record "))
    {
        push_unique(paradigms, ProgrammingParadigm::ObjectOriented);
    }

    if paradigms.len() > 1 {
        push_unique(paradigms, ProgrammingParadigm::Mixed);
    }

    if paradigms.is_empty() {
        push_unique(paradigms, ProgrammingParadigm::Unknown);
    }
}
