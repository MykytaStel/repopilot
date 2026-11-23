use super::run;

#[test]
fn js_exec_file_argument_vector_remains_safe_through_a_helper_boundary() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function run_tool(value: string) { execFile("grep", [value]); }
function handler() { run_tool(req.query.pattern); }
"#,
    );
    assert!(
        signals.is_empty(),
        "separate execFile arguments are not shell text"
    );
}

#[test]
fn js_exec_file_shell_command_remains_tainted_through_a_helper_boundary() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function run_shell(value: string) { execFile("sh", ["-c", value]); }
function handler() { run_shell(req.query.command); }
"#,
    );
    assert_eq!(signals.len(), 1, "explicit shell execution remains a sink");
}

#[test]
fn js_combined_shell_flags_remain_tainted_through_a_helper_boundary() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function run_shell(value: string) { execFile("/bin/bash", ["-lc", value]); }
function handler() { run_shell(req.query.command); }
"#,
    );
    assert_eq!(signals.len(), 1, "bash -lc executes a shell command");
}

#[test]
fn js_shell_command_flag_after_an_ordinary_option_remains_tainted() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function run_shell(value: string) { execFile("bash", ["-l", "-c", value]); }
function handler() { run_shell(req.query.command); }
"#,
    );
    assert_eq!(signals.len(), 1, "bash -l -c executes a shell command");
}

#[test]
fn js_powershell_encoded_command_remains_tainted() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function run_shell(value: string) { execFile("pwsh", ["-NoProfile", "-EncodedCommand", value]); }
function handler() { run_shell(req.query.command); }
"#,
    );
    assert_eq!(
        signals.len(),
        1,
        "encoded PowerShell command remains a sink"
    );
}

#[test]
fn js_exec_file_shell_option_keeps_the_argument_vector_tainted() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function run_tool(value: string) { execFile("grep", [value], { shell: true }); }
function handler() { run_tool(req.query.pattern); }
"#,
    );
    assert_eq!(
        signals.len(),
        1,
        "shell:true converts argv into shell input"
    );
}
