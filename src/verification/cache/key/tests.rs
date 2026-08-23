use super::{KeyContext, build_with_context, program_candidates};
use crate::config::loader::parse_config;
use crate::scan::session::WorkspaceRevision;
use crate::verification::{ValidatedCheck, select_checks};
use std::collections::BTreeMap;
use std::fs;
use tempfile::tempdir;

fn context() -> KeyContext<'static> {
    KeyContext {
        schema_version: 1,
        repopilot_version: "0.22.0",
        os: "test-os",
        arch: "test-arch",
        inherited_environment: BTreeMap::from([
            ("PATH".to_string(), Some(b"/trusted/bin".to_vec())),
            ("HOME".to_string(), Some(b"private-home-value".to_vec())),
        ]),
    }
}

fn selected(root: &std::path::Path, fields: &str) -> ValidatedCheck {
    let default_role = if fields.contains("role =") {
        ""
    } else {
        "role = \"test\"\n"
    };
    let config = parse_config(
        &format!(
            "[[verification.checks]]\nid = \"unit\"\n{default_role}program = \"./tool.sh\"\n{fields}\n"
        ),
        None,
    )
    .expect("valid check config");
    select_checks(root, &config.verification.checks, &["unit".into()])
        .expect("valid selected check")
        .remove(0)
}

#[test]
fn key_is_stable_and_never_exposes_environment_values() {
    let temp = tempdir().expect("temp dir");
    fs::write(temp.path().join("tool.sh"), "exit 0\n").expect("tool");
    let check = selected(temp.path(), "");
    let revision = WorkspaceRevision::capture(temp.path());

    let first = build_with_context(&check, &revision, &context()).expect("key");
    let second = build_with_context(&check, &revision, &context()).expect("key");

    assert_eq!(first, second);
    assert_eq!(first.as_str().len(), 64);
    assert!(!first.as_str().contains("private-home-value"));
}

#[test]
fn key_invalidates_on_context_changes() {
    let temp = tempdir().expect("temp dir");
    fs::write(temp.path().join("tool.sh"), "exit 0\n").expect("tool");
    let check = selected(temp.path(), "");
    let revision = WorkspaceRevision::capture(temp.path());
    let baseline = build_with_context(&check, &revision, &context()).expect("baseline");

    let mut variants = Vec::new();
    let mut schema = context();
    schema.schema_version = 2;
    variants.push(schema);
    let mut version = context();
    version.repopilot_version = "0.22.1";
    variants.push(version);
    let mut os = context();
    os.os = "other-os";
    variants.push(os);
    let mut arch = context();
    arch.arch = "other-arch";
    variants.push(arch);
    let mut environment = context();
    environment
        .inherited_environment
        .insert("PATH".into(), Some(b"/other/bin".to_vec()));
    variants.push(environment);

    for variant in variants {
        assert_ne!(
            build_with_context(&check, &revision, &variant).expect("variant"),
            baseline
        );
    }
}

#[test]
fn key_invalidates_on_every_execution_policy_field() {
    let temp = tempdir().expect("temp dir");
    fs::create_dir(temp.path().join("subdir")).expect("subdir");
    fs::write(temp.path().join("tool.sh"), "exit 0\n").expect("tool");
    let revision = WorkspaceRevision::capture(temp.path());
    let baseline =
        build_with_context(&selected(temp.path(), ""), &revision, &context()).expect("baseline");

    for fields in [
        "role = \"lint\"",
        "args = [\"--all\"]",
        "working_directory = \"subdir\"",
        "timeout_seconds = 60",
        "max_output_bytes = 1024",
        "paths = [\"src/**\"]",
    ] {
        assert_ne!(
            build_with_context(&selected(temp.path(), fields), &revision, &context())
                .expect("variant"),
            baseline,
            "field must invalidate key: {fields}"
        );
    }
}

#[test]
fn path_pattern_order_and_duplicates_do_not_change_the_key() {
    let temp = tempdir().expect("temp dir");
    fs::write(temp.path().join("tool.sh"), "exit 0\n").expect("tool");
    let revision = WorkspaceRevision::capture(temp.path());
    let left = selected(
        temp.path(),
        "paths = [\"tests/**\", \"src/**\", \"src/**\"]",
    );
    let right = selected(temp.path(), "paths = [\"src/**\", \"tests/**\"]");

    assert_eq!(
        build_with_context(&left, &revision, &context()),
        build_with_context(&right, &revision, &context())
    );
}

#[test]
fn key_invalidates_on_executable_bytes_and_workspace_revision() {
    let temp = tempdir().expect("temp dir");
    let executable = temp.path().join("tool.sh");
    fs::write(&executable, "exit 0\n").expect("tool");
    let check = selected(temp.path(), "");
    let revision = WorkspaceRevision::capture(temp.path());
    let baseline = build_with_context(&check, &revision, &context()).expect("baseline");

    fs::write(&executable, "exit 1\n").expect("mutate tool");
    let executable_changed =
        build_with_context(&check, &revision, &context()).expect("changed executable");
    assert_ne!(executable_changed, baseline);

    let changed_revision = WorkspaceRevision::capture(temp.path());
    assert_ne!(
        build_with_context(&check, &changed_revision, &context()).expect("changed revision"),
        executable_changed
    );
}

#[test]
fn unreadable_or_missing_program_disables_the_key() {
    let temp = tempdir().expect("temp dir");
    fs::write(temp.path().join("tool.sh"), "exit 0\n").expect("tool");
    let check = selected(temp.path(), "");
    fs::remove_file(temp.path().join("tool.sh")).expect("remove tool");

    assert!(
        build_with_context(&check, &WorkspaceRevision::capture(temp.path()), &context()).is_none()
    );
}

#[test]
fn windows_program_candidates_follow_pathext_order() {
    assert_eq!(
        program_candidates("tool", Some(".EXE;.CMD"), true),
        ["tool", "tool.EXE", "tool.CMD"]
    );
    assert_eq!(program_candidates("tool", None, false), ["tool"]);
}
