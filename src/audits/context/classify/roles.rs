use super::frameworks::{
    is_dotnet_controller, is_dotnet_service, is_react_component_file, is_react_hook_file,
};
use super::helpers::{
    is_app_entrypoint, is_build_tooling_path, is_config_file, is_generated_file, is_js_or_ts,
    path_contains_component,
};
use super::signals::ContextSignals;
use crate::audits::context::model::{
    FileRole, FrameworkKind, LanguageKind, RoleEvidence, RoleEvidenceSource,
};
use std::path::Path;

// This is the single orchestration boundary for role classification. Keeping
// all classifier inputs explicit makes path/content/framework decisions visible
// and avoids hiding them inside mutable global state.
#[allow(clippy::too_many_arguments)]
pub fn classify_roles(
    roles: &mut Vec<FileRole>,
    evidence: &mut Vec<RoleEvidence>,
    path: &Path,
    content: &str,
    language: LanguageKind,
    frameworks: &[FrameworkKind],
    signals: &ContextSignals,
    is_test: bool,
) {
    classify_static_roles(roles, evidence, path, content, language, signals, is_test);

    if is_js_or_ts(language) {
        classify_js_ts_roles(roles, evidence, path, content, frameworks);
    }

    if language == LanguageKind::CSharp {
        classify_csharp_roles(roles, evidence, path, content, frameworks);
    }

    classify_entrypoint_and_path_roles(roles, evidence, path, content, language);
}

pub(super) fn push_role(
    roles: &mut Vec<FileRole>,
    evidence: &mut Vec<RoleEvidence>,
    role: FileRole,
    source: RoleEvidenceSource,
    reason: &'static str,
) {
    if roles.contains(&role) {
        return;
    }
    roles.push(role);
    evidence.push(RoleEvidence::new(role, source, reason));
}

fn classify_static_roles(
    roles: &mut Vec<FileRole>,
    evidence: &mut Vec<RoleEvidence>,
    path: &Path,
    content: &str,
    language: LanguageKind,
    signals: &ContextSignals,
    is_test: bool,
) {
    if is_config_file(path) {
        push_role(
            roles,
            evidence,
            FileRole::Config,
            RoleEvidenceSource::Path,
            "recognized configuration filename or path",
        );
    }

    if signals.is_infrastructure_context() {
        push_role(
            roles,
            evidence,
            FileRole::Infrastructure,
            RoleEvidenceSource::Signal,
            "infrastructure context signal matched",
        );
    }

    if is_generated_file(path, content) {
        push_role(
            roles,
            evidence,
            FileRole::Generated,
            RoleEvidenceSource::Mixed,
            "generated/vendor path or generated-content marker matched",
        );
    }

    if is_test {
        push_role(
            roles,
            evidence,
            FileRole::Test,
            RoleEvidenceSource::Path,
            "recognized test path or filename",
        );

        if language == LanguageKind::Rust {
            push_role(
                roles,
                evidence,
                FileRole::RustTest,
                RoleEvidenceSource::Path,
                "recognized Rust test path or filename",
            );
        }
    }

    if let Some(support) = crate::languages::conventions::conventions_for_kind(language)
        .test_support
        .filter(|support| (support.matches)(path))
    {
        push_role(
            roles,
            evidence,
            FileRole::TestSupport,
            RoleEvidenceSource::Path,
            support.reason,
        );
    }

    if is_build_tooling_path(path) {
        push_role(
            roles,
            evidence,
            FileRole::BuildTooling,
            RoleEvidenceSource::Path,
            "recognized build-logic or buildSrc path",
        );
    }
}

fn classify_js_ts_roles(
    roles: &mut Vec<FileRole>,
    evidence: &mut Vec<RoleEvidence>,
    path: &Path,
    content: &str,
    frameworks: &[FrameworkKind],
) {
    if is_react_hook_file(path, content) {
        push_role(
            roles,
            evidence,
            FileRole::ReactHook,
            RoleEvidenceSource::Mixed,
            "React hook filename or implementation matched",
        );
        push_role(
            roles,
            evidence,
            FileRole::FrameworkHook,
            RoleEvidenceSource::Mixed,
            "framework hook shape matched",
        );
    }

    if frameworks.contains(&FrameworkKind::React) && is_react_component_file(path, content) {
        push_role(
            roles,
            evidence,
            FileRole::ReactComponent,
            RoleEvidenceSource::Mixed,
            "React framework plus component shape matched",
        );
        push_role(
            roles,
            evidence,
            FileRole::FrameworkComponent,
            RoleEvidenceSource::Mixed,
            "framework component shape matched",
        );
    }
}

fn classify_csharp_roles(
    roles: &mut Vec<FileRole>,
    evidence: &mut Vec<RoleEvidence>,
    path: &Path,
    content: &str,
    frameworks: &[FrameworkKind],
) {
    let lower = content.to_lowercase();
    if frameworks.contains(&FrameworkKind::Unity) && lower.contains("monobehaviour") {
        push_role(
            roles,
            evidence,
            FileRole::UnityMonoBehaviour,
            RoleEvidenceSource::Mixed,
            "Unity framework plus MonoBehaviour base type matched",
        );
    }
    if is_dotnet_controller(path, &lower) {
        push_role(
            roles,
            evidence,
            FileRole::DotNetController,
            RoleEvidenceSource::Mixed,
            "ASP.NET controller path or type shape matched",
        );
        push_role(
            roles,
            evidence,
            FileRole::FrameworkController,
            RoleEvidenceSource::Mixed,
            "framework controller shape matched",
        );
    }
    if is_dotnet_service(path, &lower) {
        push_role(
            roles,
            evidence,
            FileRole::DotNetService,
            RoleEvidenceSource::Mixed,
            ".NET service path or type shape matched",
        );
        push_role(
            roles,
            evidence,
            FileRole::FrameworkService,
            RoleEvidenceSource::Mixed,
            "framework service shape matched",
        );
    }
}

fn classify_entrypoint_and_path_roles(
    roles: &mut Vec<FileRole>,
    evidence: &mut Vec<RoleEvidence>,
    path: &Path,
    content: &str,
    language: LanguageKind,
) {
    if is_app_entrypoint(path, content, language) {
        push_role(
            roles,
            evidence,
            FileRole::AppEntrypoint,
            RoleEvidenceSource::Mixed,
            "recognized application entrypoint path or entry function",
        );
    }

    if matches!(language, LanguageKind::Python | LanguageKind::Go)
        && path_contains_component(path, &["cmd", "bin", "scripts"])
    {
        push_role(
            roles,
            evidence,
            FileRole::Script,
            RoleEvidenceSource::Path,
            "command or script directory matched",
        );
    }

    if path_contains_component(
        path,
        &["domain", "domains", "model", "models", "entity", "entities"],
    ) {
        push_role(
            roles,
            evidence,
            FileRole::Domain,
            RoleEvidenceSource::Path,
            "domain/model/entity path component matched",
        );
    }

    if path_contains_component(path, &["script", "scripts", "bin", "tools", "guards"])
        || is_guard_file(path)
    {
        push_role(
            roles,
            evidence,
            FileRole::Script,
            RoleEvidenceSource::Path,
            "script/tool/guard path or filename matched",
        );
    }
}

fn is_guard_file(path: &Path) -> bool {
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(|name| name.to_lowercase().contains("guard"))
        .unwrap_or(false)
}
