use super::{run, run_with_changed_lines};

#[test]
fn v028_helper_sink_uses_state_at_the_sink() {
    for (statements, expected) in [
        ("db.query(value); value = 'fixed';", 1),
        ("value = 'fixed'; db.query(value); value = input;", 0),
    ] {
        let code = format!(
            "function report(input, value) {{ {statements} }}\nfunction handler() {{ report(req.query.message, req.query.message); }}\n"
        );
        let signals = run_with_changed_lines("src/handler.ts", "TypeScript", &code, 2, 2);
        assert_eq!(signals.len(), expected, "{statements}: {signals:?}");
    }
}

#[test]
fn v028_changed_identity_does_not_claim_other_caller_sources() {
    let signals = run_with_changed_lines(
        "src/handler.ts",
        "TypeScript",
        concat!(
            "function identity(value) { return value; }\n",
            "function handler() {\n",
            "  const ignored = identity(req.query.ignored);\n",
            "  const existing = req.query.existing;\n",
            "  db.query(existing);\n",
            "}\n",
        ),
        3,
        3,
    );
    assert!(signals.is_empty(), "{signals:?}");
}

#[test]
fn v028_taint_does_not_chain_recognized_helpers() {
    for calls in [
        "const copied = identity(req.query.message); report(copied);",
        "report(identity(req.query.message));",
        "const copied = identity(req.query.message) + ' suffix'; report(copied);",
    ] {
        let code = format!(
            concat!(
                "function identity(value) {{ return value; }}\n",
                "function report(value) {{ db.query(value); }}\n",
                "function handler() {{ {} }}\n",
            ),
            calls
        );
        let signals = run("src/handler.ts", "TypeScript", &code);
        assert!(signals.is_empty(), "{calls}: {signals:?}");
    }
}

#[test]
fn v028_python_helper_arguments_follow_keyword_names() {
    for (arguments, expected) in [
        ("other=request.args.get('note'), message='fixed'", 0),
        ("other='fixed', message=request.args.get('note')", 1),
    ] {
        let code = format!(
            concat!(
                "def report(message, other):\n",
                "    cursor.execute(message)\n",
                "def handler():\n",
                "    report({})\n",
            ),
            arguments
        );
        let signals = run_with_changed_lines("src/handler.py", "Python", &code, 4, 4);
        assert_eq!(signals.len(), expected, "{arguments}: {signals:?}");
    }
}

#[test]
fn v028_python_interpolated_helper_returns_are_not_clean_literals() {
    let signals = run(
        "src/handler.py",
        "Python",
        concat!(
            "def format_message(message):\n",
            "    return f'message: {message}'\n",
            "def handler():\n",
            "    result = format_message(request.args.get('note'))\n",
            "    cursor.execute(result)\n",
        ),
    );
    assert_eq!(signals.len(), 1, "{signals:?}");
}
