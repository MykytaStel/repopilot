use super::*;

#[test]
fn python_changed_imported_helper_is_counted_in_unchanged_test() {
    let before = "def verify(value):\n    assert value\n    assert value is not None\n";
    let after = "def verify(value):\n    assert value\n";
    let caller =
        "from checks import verify as assert_valid\n\ndef test_request():\n    assert_valid(1)\n";
    let signals = changed_helper_signals(
        "checks.py",
        "Python",
        before,
        after,
        "test_checks.py",
        caller,
    );

    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].kind, IntegrityKind::AssertionsRemoved);
    assert!(signals[0].path.ends_with("test_checks.py"));
    assert!(signals[0].detail.contains("checks.py::verify"));
}

#[test]
fn python_reordered_helper_assertions_are_a_safe_refactor() {
    let signals = changed_helper_signals(
        "checks.py",
        "Python",
        "def verify(value):\n    assert value\n    assert value is not None\n",
        "def verify(value):\n    assert value is not None\n    assert value\n",
        "test_checks.py",
        "from checks import verify as assert_valid\n\ndef test_request():\n    assert_valid(1)\n",
    );
    assert!(
        signals.is_empty(),
        "reordering checks keeps their semantics"
    );
}

#[test]
fn v028_python_competing_module_and_package_targets_do_not_get_attributed() {
    let repo = tempdir().expect("temporary repo");
    let root = repo.path();
    write(
        root,
        "checks/__init__.py",
        "def verify(value):\n    assert value\n    assert value is not None\n",
    );
    write(
        root,
        "checks.py",
        "def verify(value):\n    assert value\n    assert value is not None\n",
    );
    write(
        root,
        "test_checks.py",
        "from checks import verify\n\ndef test_request():\n    verify(1)\n",
    );
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot test"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "before"]);
    write(
        root,
        "checks.py",
        "def verify(value):\n    assert value is not None\n",
    );

    let file = ChangedFile {
        path: PathBuf::from("checks.py"),
        status: ChangeStatus::Modified,
        ranges: vec![ChangedRange { start: 1, end: 20 }],
        hunks: Vec::new(),
    };
    let before = ReviewSource::new(
        "def verify(value):\n    assert value\n    assert value is not None\n".to_string(),
        Some("Python".to_string()),
    );
    let after = ReviewSource::new(
        "def verify(value):\n    assert value is not None\n".to_string(),
        Some("Python".to_string()),
    );
    let changed = [ChangedIntegritySource {
        file: &file,
        pre: Some(&before),
        post: Some(&after),
    }];
    let evidence = collect_file_evidence(&file, Some(&before), Some(&after))
        .into_iter()
        .collect::<Vec<_>>();

    assert!(detect(root, DiffTarget::WorkingTree, &changed, &evidence).is_empty());
}

#[test]
fn v028_python_ignores_imports_from_other_scopes_and_shadowed_aliases() {
    for caller in [
        "from other import verify as check\n\ndef unrelated():\n    from checks import verify as check\n    return check\n\ndef test_request():\n    check(1)\n",
        "from checks import verify as check\n\ndef test_request(check):\n    check(1)\n",
        "from checks import verify as check\nfrom other import verify as check\n\ndef test_request():\n    check(1)\n",
    ] {
        let signals = changed_helper_signals(
            "checks.py",
            "Python",
            "def verify(value):\n    assert value\n    assert value is not None\n",
            "def verify(value):\n    assert value\n",
            "test_checks.py",
            caller,
        );
        assert!(signals.is_empty(), "{caller}: {signals:?}");
    }
}

#[test]
fn v028_python_reports_only_called_helper_with_declaration_line() {
    let signals = changed_helper_signals(
        "checks.py",
        "Python",
        "\ndef verify(value):\n    assert value\n    assert value is not None\n\ndef other(value):\n    assert value\n    assert value is not None\n",
        "\ndef verify(value):\n    assert value\n\ndef other(value):\n    assert value\n",
        "test_checks.py",
        "from checks import verify as check, other\n\ndef test_request():\n    check(1)\n",
    );
    assert_eq!(signals.len(), 1);
    assert!(signals[0].detail.contains("checks.py::verify (line 2)"));
    assert!(!signals[0].detail.contains("other"));
}
