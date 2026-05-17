use super::helpers::{is_js_or_ts, path_contains_component, push_unique};
use super::signals::ContextSignals;
use crate::audits::context::model::{FileRole, FrameworkKind, LanguageKind, ProgrammingParadigm};
use std::path::Path;

pub fn classify_paradigms(
    paradigms: &mut Vec<ProgrammingParadigm>,
    path: &Path,
    content: &str,
    language: LanguageKind,
    frameworks: &[FrameworkKind],
    roles: &[FileRole],
    signals: &ContextSignals,
) {
    // All patterns here are lowercase keywords in their respective languages — no allocation needed.
    classify_role_and_framework_paradigms(paradigms, frameworks, roles);

    if language == LanguageKind::Rust {
        classify_rust_paradigms(paradigms, path, content);
    }

    if is_js_or_ts(language) {
        classify_js_ts_paradigms(paradigms, content);
    }

    if matches!(language, LanguageKind::Python | LanguageKind::Go) {
        classify_procedural_script_paradigms(paradigms, path, content);
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

    if signals.is_functional_first_language() {
        push_unique(paradigms, ProgrammingParadigm::Functional);
    }

    if signals.is_declarative_context() {
        push_unique(paradigms, ProgrammingParadigm::Declarative);
    }

    if paradigms.len() > 1 {
        push_unique(paradigms, ProgrammingParadigm::Mixed);
    }

    if paradigms.is_empty() {
        push_unique(paradigms, ProgrammingParadigm::Unknown);
    }
}

fn classify_role_and_framework_paradigms(
    paradigms: &mut Vec<ProgrammingParadigm>,
    frameworks: &[FrameworkKind],
    roles: &[FileRole],
) {
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
}

fn classify_rust_paradigms(paradigms: &mut Vec<ProgrammingParadigm>, path: &Path, content: &str) {
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

fn classify_js_ts_paradigms(paradigms: &mut Vec<ProgrammingParadigm>, content: &str) {
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

fn classify_procedural_script_paradigms(
    paradigms: &mut Vec<ProgrammingParadigm>,
    path: &Path,
    content: &str,
) {
    if content.contains("def main(")
        || content.contains("func main(")
        || path_contains_component(path, &["cmd", "scripts"])
    {
        push_unique(paradigms, ProgrammingParadigm::Procedural);
    }
}
