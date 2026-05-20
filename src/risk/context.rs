use crate::audits::context::{FileRole, classify_file};
use crate::scan::facts::FileFacts;
use crate::scan::path_classification::is_low_signal_audit_path;

use super::model::{RiskSignal, push_adjustment};

pub(super) fn add_file_context_signals(
    file: &FileFacts,
    score: &mut i16,
    signals: &mut Vec<RiskSignal>,
) {
    let context = classify_file(file);
    if context.is_production_code() {
        push_adjustment(
            score,
            signals,
            "role.production",
            "visibility",
            10,
            "production code findings are more likely to affect runtime behavior",
        );
    }

    if context.has_role(FileRole::Test) || context.has_role(FileRole::RustTest) {
        push_adjustment(
            score,
            signals,
            "role.test",
            "visibility",
            -20,
            "test code often accepts patterns that would be risky in production",
        );
    }

    if context.has_role(FileRole::Generated) {
        push_adjustment(
            score,
            signals,
            "role.generated",
            "visibility",
            -30,
            "generated files should usually be fixed at the generator source",
        );
    }

    if context.has_role(FileRole::Config) {
        push_adjustment(
            score,
            signals,
            "role.config",
            "visibility",
            -18,
            "configuration files often use declarative patterns that differ from application code",
        );
    }

    if is_low_signal_audit_path(&file.path)
        && !context.has_role(FileRole::Test)
        && !context.has_role(FileRole::Generated)
    {
        push_adjustment(
            score,
            signals,
            "role.low-signal-path",
            "visibility",
            -15,
            "fixtures, examples, and benchmark paths are lower-priority by default",
        );
    }

    if context.has_role(FileRole::AppEntrypoint) {
        push_adjustment(
            score,
            signals,
            "role.entrypoint",
            "visibility",
            10,
            "entrypoints can affect application startup and process behavior",
        );
    }

    if context.has_role(FileRole::FrameworkController)
        || context.has_role(FileRole::DotNetController)
    {
        push_adjustment(
            score,
            signals,
            "role.controller",
            "visibility",
            12,
            "controllers and routers sit on user-facing request boundaries",
        );
    }

    if context.has_role(FileRole::FrameworkService) || context.has_role(FileRole::DotNetService) {
        push_adjustment(
            score,
            signals,
            "role.service",
            "visibility",
            10,
            "service-layer findings can affect reusable production behavior",
        );
    }

    if context.has_role(FileRole::Domain) {
        push_adjustment(
            score,
            signals,
            "role.domain",
            "visibility",
            8,
            "domain code usually carries core business behavior",
        );
    }

    if context.has_role(FileRole::ReactComponent) {
        push_adjustment(
            score,
            signals,
            "role.react-component",
            "visibility",
            4,
            "React component findings can affect user-visible behavior",
        );
    }
}
