//! MCP and CLI projections of the same review agree on integrity signals:
//! `repopilot_review_change` returns the same verdict, reasons, and
//! integrity signals (id, kind, tier, path, line, detail) as
//! `review --format json`, so an agent sees exactly what CI sees.

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const BEFORE: &str = r#"describe("cart", () => {
  it("sums line items", () => {
    expect(total([2, 3])).toBe(5);
    expect(total([])).toBe(0);
  });
  it("applies the discount", () => {
    expect(total([10], 0.1)).toBe(9);
  });
  it("rejects negative quantities", () => {
    expect(() => total([-1])).toThrow();
  });
});
"#;

#[test]
fn mcp_review_returns_the_integrity_signals_the_cli_reports() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("src dir");
    fs::write(root.join("src/cart.test.ts"), BEFORE).expect("seed test");
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "test@example.com"],
        &["config", "user.name", "Test"],
        &["add", "."],
        &["commit", "-qm", "initial"],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }
    let weakened = BEFORE
        .replace("it(\"applies", "it.only(\"applies")
        .replace("it(\"rejects", "it.skip(\"rejects")
        .replace("    expect(total([])).toBe(0);\n", "");
    fs::write(root.join("src/cart.test.ts"), weakened).expect("weaken");

    let cli = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["review", ".", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("run review");
    let cli: Value = serde_json::from_slice(&cli.stdout).expect("cli json");
    let mcp = mcp_review(root);

    let cli_signals = integrity_signals(&cli);
    assert_eq!(
        cli_signals
            .iter()
            .map(|signal| signal["kind"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec![
            "integrity.test-focused",
            "integrity.assertions-removed",
            "integrity.test-skipped"
        ]
    );
    assert_eq!(integrity_signals(&mcp), cli_signals);
    assert_eq!(
        mcp["change_proof"]["verdict"],
        cli["change_proof"]["verdict"]
    );
    assert_eq!(cli["change_proof"]["verdict"], "REVIEW");
    assert_eq!(reason_codes(&mcp), reason_codes(&cli));
}

/// Integrity signals across tiers, reduced to the fields both projections share.
fn integrity_signals(report: &Value) -> Vec<Value> {
    ["definitely", "maybe", "noise"]
        .iter()
        .flat_map(|tier| {
            report["tiered_signals"][tier]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter(|signal| signal["family"] == "integrity")
        .map(|signal| {
            let mut kept = serde_json::Map::new();
            for field in [
                "signal_id",
                "kind",
                "tier",
                "path",
                "line",
                "detail",
                "headline",
            ] {
                kept.insert(field.to_string(), signal[field].clone());
            }
            Value::Object(kept)
        })
        .collect()
}

fn reason_codes(report: &Value) -> Vec<Value> {
    report["change_proof"]["reasons"]
        .as_array()
        .map(|reasons| {
            reasons
                .iter()
                .map(|reason| reason["code"].clone())
                .collect()
        })
        .unwrap_or_default()
}

fn mcp_review(root: &Path) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .arg("mcp")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        for request in [
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"repopilot_review_change","arguments":{"path":"."}}}"#,
        ] {
            writeln!(stdin, "{request}").expect("write request");
        }
    }
    let output = child.wait_with_output().expect("wait for mcp");
    let response = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|message| message["id"] == 2)
        .expect("tool response");
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    serde_json::from_str(text).expect("review report json")
}
