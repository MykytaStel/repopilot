use std::fs;
use std::path::Path;

#[test]
fn action_exposes_typed_receipt_input_and_output() {
    let action_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("action.yml");
    let action = fs::read_to_string(action_path).expect("read action.yml");

    assert!(action.contains("receipt:"));
    assert!(action.contains("receipt-file:"));
    assert!(action.contains("RECEIPT=\"${{ inputs.receipt }}\""));
    assert!(action.contains("Receipt output is only supported by 'scan'"));
    assert!(action.contains("RUN_ARGS+=(--receipt \"$RECEIPT\")"));
    assert!(action.contains("receipt_file=$RECEIPT"));
}
