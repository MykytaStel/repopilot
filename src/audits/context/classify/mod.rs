mod frameworks;
pub(crate) mod helpers;
mod paradigms;
mod roles;
mod runtimes;
mod signals;

use crate::audits::context::model::{AuditContext, FileRole, LanguageKind};
use crate::knowledge::language::language_kind_for_file;
use crate::scan::facts::FileFacts;
use helpers::{is_test_file, path_contains_component};
use signals::ContextSignals;

pub fn classify_file(file: &FileFacts) -> AuditContext {
    let content = file.content.as_deref().unwrap_or("");
    let language = classify_language(file);
    let is_test = is_test_file(&file.path);
    let signals = ContextSignals::detect(&file.path, language, content);

    let mut frameworks = Vec::new();
    frameworks::classify_frameworks(&mut frameworks, &file.path, content, language);

    let mut roles = Vec::new();
    roles::classify_roles(
        &mut roles,
        &file.path,
        content,
        language,
        &frameworks,
        &signals,
        is_test,
    );

    // A CLI command handler is a genuine exit boundary only when BOTH hold: the
    // file lives in a `commands/` directory AND its package declares an executable
    // (`in_executable_package`, set from the nearest package's manifest). This
    // pairs path evidence with manifest evidence, so a CQRS `domain/commands/` in
    // a non-CLI package is not exempted, and a reusable module (`lib/`, `utils/`)
    // inside a CLI package keeps its default-visible severity. `bin/`/`scripts/`
    // entrypoints are already handled by the `script`/`app-entrypoint` roles.
    if file.in_executable_package && path_contains_component(&file.path, &["commands"]) {
        roles.push(FileRole::CliExecutable);
    }

    if roles.is_empty() {
        roles.push(FileRole::Unknown);
    }

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

    AuditContext {
        language,
        frameworks,
        roles,
        paradigms,
        runtimes,
        is_test,
    }
}

fn classify_language(file: &FileFacts) -> LanguageKind {
    language_kind_for_file(file)
}

#[cfg(test)]
mod tests;
