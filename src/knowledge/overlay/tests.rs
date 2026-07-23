use super::*;

#[test]
fn parses_a_valid_rule_entry_with_path_and_severity() {
    let content = r#"
        [[overlay]]
        rule = "architecture.large-file"
        path = "legacy/**"
        severity = "low"
        reason = "Legacy freeze until Q3 migration"
    "#;
    let validation = parse_overlay_content(content, PathBuf::from(".repopilot/overlay.toml"));

    assert!(validation.parse_error.is_none());
    assert_eq!(validation.entries.len(), 1);
    let entry = &validation.entries[0];
    assert_eq!(
        entry.target,
        OverlayTarget::Rule("architecture.large-file".to_string())
    );
    assert_eq!(entry.severity, Some(Severity::Low));
    assert_eq!(
        entry.reason.as_deref(),
        Some("Legacy freeze until Q3 migration")
    );
    assert!(entry.path_glob.is_some());
}

#[test]
fn rejects_entry_missing_both_rule_and_kind() {
    let content = r#"
        [[overlay]]
        path = "legacy/**"
        severity = "low"
    "#;
    let validation = parse_overlay_content(content, PathBuf::from(".repopilot/overlay.toml"));
    assert_eq!(validation.entries.len(), 0);
    assert_eq!(validation.invalid_entries_count, 1);
    assert!(
        validation.diagnostics[0]
            .message
            .contains("missing exactly one")
    );
}

#[test]
fn rejects_entry_with_both_rule_and_kind() {
    let content = r#"
        [[overlay]]
        rule = "architecture.large-file"
        kind = "behavioral"
    "#;
    let validation = parse_overlay_content(content, PathBuf::from(".repopilot/overlay.toml"));
    assert_eq!(validation.invalid_entries_count, 1);
    assert!(
        validation.diagnostics[0]
            .message
            .contains("exactly one is allowed")
    );
}

#[test]
fn rejects_unknown_rule_id() {
    let content = r#"
        [[overlay]]
        rule = "not-a-real-rule"
    "#;
    let validation = parse_overlay_content(content, PathBuf::from(".repopilot/overlay.toml"));
    assert_eq!(validation.invalid_entries_count, 1);
    assert_eq!(validation.diagnostics[0].code, "overlay.unknown-rule");
}

#[test]
fn rejects_severity_on_kind_entry() {
    let content = r#"
        [[overlay]]
        kind = "behavioral"
        severity = "low"
    "#;
    let validation = parse_overlay_content(content, PathBuf::from(".repopilot/overlay.toml"));
    assert_eq!(validation.invalid_entries_count, 1);
    assert!(validation.diagnostics[0].message.contains("no severity"));
}

#[test]
fn rejects_invalid_severity_label() {
    let content = r#"
        [[overlay]]
        rule = "architecture.large-file"
        severity = "catastrophic"
    "#;
    let validation = parse_overlay_content(content, PathBuf::from(".repopilot/overlay.toml"));
    assert_eq!(validation.invalid_entries_count, 1);
    assert_eq!(validation.diagnostics[0].code, "overlay.invalid-severity");
}

#[test]
fn rejects_unparseable_glob() {
    let content = r#"
        [[overlay]]
        rule = "architecture.large-file"
        path = "legacy/["
    "#;
    let validation = parse_overlay_content(content, PathBuf::from(".repopilot/overlay.toml"));
    assert_eq!(validation.invalid_entries_count, 1);
    assert_eq!(validation.diagnostics[0].code, "overlay.invalid-path-glob");
}

#[test]
fn rejects_invalid_expiry_date() {
    let content = r#"
        [[overlay]]
        rule = "architecture.large-file"
        expires = "not-a-date"
    "#;
    let validation = parse_overlay_content(content, PathBuf::from(".repopilot/overlay.toml"));
    assert_eq!(validation.invalid_entries_count, 1);
    assert_eq!(validation.diagnostics[0].code, "overlay.invalid-expiry");
}

#[test]
fn accepts_a_valid_kind_entry_without_severity() {
    let content = r#"
        [[overlay]]
        kind = "behavioral"
        path = "scripts/**"
        reason = "Ops scripts are expected to shell out"
    "#;
    let validation = parse_overlay_content(content, PathBuf::from(".repopilot/overlay.toml"));
    assert_eq!(validation.entries.len(), 1);
    assert_eq!(
        validation.entries[0].target,
        OverlayTarget::Kind("behavioral".to_string())
    );
    assert!(validation.entries[0].severity.is_none());
}
