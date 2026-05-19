use std::fs;
use std::path::Path;

#[test]
fn action_exposes_typed_receipt_input_and_output() {
    let action_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("action.yml");
    let action = fs::read_to_string(action_path).expect("read action.yml");

    assert!(action.contains("receipt:"));
    assert!(action.contains("receipt-file:"));
    assert!(action.contains("INPUT_RECEIPT: ${{ inputs.receipt }}"));
    assert!(action.contains("scripts/repopilot-action.sh"));

    let helper_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action.sh");
    let helper = fs::read_to_string(helper_path).expect("read action helper");
    assert!(helper.contains("Receipt output is only supported by 'scan'"));
    assert!(helper.contains("RUN_ARGS+=(--receipt \"$RECEIPT\")"));
    assert!(helper.contains("receipt_file=$RECEIPT"));
}

#[test]
fn action_exposes_priority_and_rule_parity_inputs() {
    let action_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("action.yml");
    let action = fs::read_to_string(action_path).expect("read action.yml");

    for input in ["fail-on-priority:", "min-priority:", "rule:", "timing:"] {
        assert!(action.contains(input), "missing action input {input}");
    }

    assert!(action.contains("INPUT_FAIL_ON_PRIORITY: ${{ inputs.fail-on-priority }}"));
    assert!(action.contains("INPUT_MIN_PRIORITY: ${{ inputs.min-priority }}"));
    assert!(action.contains("INPUT_RULE: ${{ inputs.rule }}"));
    assert!(action.contains("INPUT_TIMING: ${{ inputs.timing }}"));

    let helper_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action.sh");
    let helper = fs::read_to_string(helper_path).expect("read action helper");
    assert!(helper.contains("RUN_ARGS+=(--fail-on-priority \"$FAIL_ON_PRIORITY\")"));
    assert!(helper.contains("RUN_ARGS+=(--min-priority \"$MIN_PRIORITY\")"));
    assert!(helper.contains("RUN_ARGS+=(--rule \"$RULE\")"));
    assert!(helper.contains("RUN_ARGS+=(--timing)"));
}

#[test]
fn action_supports_doctor_as_markdown_auto_command() {
    let action_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("action.yml");
    let action = fs::read_to_string(action_path).expect("read action.yml");

    assert!(action.contains("scan | review | compare | doctor | ai-context"));
    assert!(action.contains("review/compare/doctor"));

    let helper_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action.sh");
    let helper = fs::read_to_string(helper_path).expect("read action helper");
    assert!(helper.contains("doctor) RUN_ARGS=(doctor \"$PATH_INPUT\") ;;"));
    assert!(helper.contains("SARIF output and upload are only supported by 'scan'"));
    assert!(!helper.contains("ai-context|vibe"));
}
