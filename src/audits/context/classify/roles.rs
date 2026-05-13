use crate::audits::context::model::{FileRole, FrameworkKind, LanguageKind};
use super::frameworks::{is_dotnet_controller, is_dotnet_service, is_react_component_file, is_react_hook_file};
use super::helpers::{
    is_app_entrypoint, is_config_file, is_generated_file, is_js_or_ts, path_contains_component,
    push_unique,
};
use std::path::Path;

pub fn classify_roles(
    roles: &mut Vec<FileRole>,
    path: &Path,
    content: &str,
    language: LanguageKind,
    frameworks: &[FrameworkKind],
    is_test: bool,
) {
    if is_config_file(path) {
        push_unique(roles, FileRole::Config);
    }

    if is_generated_file(path, content) {
        push_unique(roles, FileRole::Generated);
    }

    if is_test {
        push_unique(roles, FileRole::Test);

        if language == LanguageKind::Rust {
            push_unique(roles, FileRole::RustTest);
        }
    }

    if is_js_or_ts(language) {
        if is_react_hook_file(path, content) {
            push_unique(roles, FileRole::ReactHook);
            push_unique(roles, FileRole::FrameworkHook);
        }

        if frameworks.contains(&FrameworkKind::React) && is_react_component_file(path, content) {
            push_unique(roles, FileRole::ReactComponent);
            push_unique(roles, FileRole::FrameworkComponent);
        }
    }

    if language == LanguageKind::CSharp {
        // Compute lowercase only for C# where PascalCase identifiers need folding.
        let lower = content.to_lowercase();
        if frameworks.contains(&FrameworkKind::Unity) && lower.contains("monobehaviour") {
            push_unique(roles, FileRole::UnityMonoBehaviour);
        }
        if is_dotnet_controller(path, &lower) {
            push_unique(roles, FileRole::DotNetController);
            push_unique(roles, FileRole::FrameworkController);
        }
        if is_dotnet_service(path, &lower) {
            push_unique(roles, FileRole::DotNetService);
            push_unique(roles, FileRole::FrameworkService);
        }
    }

    if is_app_entrypoint(path, content, language) {
        push_unique(roles, FileRole::AppEntrypoint);
    }

    if matches!(language, LanguageKind::Python | LanguageKind::Go)
        && path_contains_component(path, &["cmd", "bin", "scripts"])
    {
        push_unique(roles, FileRole::Script);
    }

    if path_contains_component(
        path,
        &["domain", "domains", "model", "models", "entity", "entities"],
    ) {
        push_unique(roles, FileRole::Domain);
    }

    if path_contains_component(path, &["script", "scripts", "bin", "tools"]) {
        push_unique(roles, FileRole::Script);
    }
}
