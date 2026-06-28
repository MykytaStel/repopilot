mod frameworks;
pub(crate) mod helpers;
mod paradigms;
mod roles;
mod runtimes;
mod signals;

use crate::audits::context::model::{
    AuditContext, FileContextClassification, FileRole, LanguageKind, RoleEvidenceSource,
};
use crate::knowledge::language::language_kind_for_file;
use crate::scan::facts::FileFacts;
use helpers::{is_test_file, path_contains_component};
use signals::ContextSignals;

/// Compatibility classifier for audits that only need semantic context.
pub fn classify_file(file: &FileFacts) -> AuditContext {
    classify_file_with_evidence(file).context
}

/// Classify a file and preserve why each semantic role matched.
pub fn classify_file_with_evidence(file: &FileFacts) -> FileContextClassification {
    let content = file.content.as_deref().unwrap_or("");
    let language = classify_language(file);
    let is_test = is_test_file(&file.path);
    let signals = ContextSignals::detect(&file.path, language, content);

    let mut frameworks = Vec::new();
    frameworks::classify_frameworks(&mut frameworks, &file.path, content, language);

    let mut roles = Vec::new();
    let mut role_evidence = Vec::new();
    roles::classify_roles(
        &mut roles,
        &mut role_evidence,
        &file.path,
        content,
        language,
        &frameworks,
        &signals,
        is_test,
    );

    // A CLI command is an exit boundary only when the path and package
    // manifest agree that this file belongs to an executable command surface.
    if file.in_executable_package && path_contains_component(&file.path, &["commands"]) {
        roles::push_role(
            &mut roles,
            &mut role_evidence,
            FileRole::CliExecutable,
            RoleEvidenceSource::Mixed,
            "commands path plus executable package manifest",
        );
    }

    if roles.is_empty() {
        roles::push_role(
            &mut roles,
            &mut role_evidence,
            FileRole::Unknown,
            RoleEvidenceSource::Fallback,
            "no specialized file-role evidence matched",
        );
    }

    debug_assert_eq!(
        roles.len(),
        role_evidence.len(),
        "every classified role must have exactly one evidence record"
    );

    let mut paradigms = Vec::new();
    paradigms::classify_paradigms(
        &mut paradigms,
        &file.path,
        content,
        language,
        &frameworks,
        &roles,
        &signals,
    );

    let mut runtimes = Vec::new();
    runtimes::classify_runtimes(
        &mut runtimes,
        &file.path,
        content,
        language,
        &frameworks,
        &signals,
    );

    FileContextClassification {
        context: AuditContext {
            language,
            frameworks,
            roles,
            paradigms,
            runtimes,
            is_test,
        },
        role_evidence,
    }
}

fn classify_language(file: &FileFacts) -> LanguageKind {
    language_kind_for_file(file)
}

#[cfg(test)]
mod tests;
