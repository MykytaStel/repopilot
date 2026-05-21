#[test]
fn direct_state_mutation_detected() {
    let dir = tempdir().unwrap();
    let mut facts = facts_for(dir.path());
    facts.files.push(jsx_file(
            &dir,
            "Comp.tsx",
            "class MyComp extends React.Component {\n  handleClick() {\n    this.state.count = 5;\n  }\n}\n",
        ));
    let findings = DirectStateMutationAudit.audit(&facts, &ScanConfig::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].rule_id,
        "framework.react-native.direct-state-mutation"
    );
    assert_eq!(findings[0].severity, Severity::High);
}


#[test]
fn state_equality_check_not_flagged() {
    let dir = tempdir().unwrap();
    let mut facts = facts_for(dir.path());
    facts.files.push(jsx_file(
        &dir,
        "Comp.tsx",
        "if (this.state.count === 5) { doSomething(); }\n",
    ));
    let findings = DirectStateMutationAudit.audit(&facts, &ScanConfig::default());
    assert!(findings.is_empty());
}

// ── Hermes disabled via gradle.properties ─────────────────────────────────


#[test]
fn hermes_disabled_in_gradle_properties_is_flagged() {
    let dir = tempdir().unwrap();
    let android = dir.path().join("android");
    std::fs::create_dir(&android).unwrap();
    write!(
        std::fs::File::create(android.join("gradle.properties")).unwrap(),
        "hermesEnabled=false\nnewArchEnabled=true\n"
    )
    .unwrap();

    let findings = HermesDisabledAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].rule_id,
        "framework.react-native.hermes-disabled"
    );
}


#[test]
fn hermes_enabled_in_gradle_properties_is_not_flagged() {
    let dir = tempdir().unwrap();
    let android = dir.path().join("android");
    std::fs::create_dir(&android).unwrap();
    writeln!(
        std::fs::File::create(android.join("gradle.properties")).unwrap(),
        "hermesEnabled=true"
    )
    .unwrap();

    let findings = HermesDisabledAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
    assert!(findings.is_empty());
}


#[test]
fn hermes_disabled_gradle_properties_with_inline_comment_is_flagged() {
    let dir = tempdir().unwrap();
    let android = dir.path().join("android");
    std::fs::create_dir(&android).unwrap();
    writeln!(
        std::fs::File::create(android.join("gradle.properties")).unwrap(),
        "hermesEnabled=false   # JSC is faster for our use case"
    )
    .unwrap();

    let findings = HermesDisabledAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
    assert_eq!(findings.len(), 1);
}
