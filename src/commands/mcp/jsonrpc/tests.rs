use super::{IncomingMessage, RequestParseError, parse_message};
use serde_json::{Value, json};

#[test]
fn parse_response_message_accepts_result_and_error() {
    let result = parse_message(
        r#"{"jsonrpc":"2.0","id":"repopilot/elicitation/4","result":{"action":"accept","content":{"approve":true}}}"#,
    )
    .expect("response result parses");
    let IncomingMessage::Response(result) = result else {
        panic!("server response was parsed as a client request");
    };
    assert_eq!(result.id, "repopilot/elicitation/4");
    assert_eq!(
        result.result,
        Some(json!({"action":"accept","content":{"approve":true}}))
    );
    assert_eq!(result.error, None);

    let error = parse_message(
        r#"{"jsonrpc":"2.0","id":17,"error":{"code":-32603,"message":"client error"}}"#,
    )
    .expect("response error parses");
    let IncomingMessage::Response(error) = error else {
        panic!("server error response was parsed as a client request");
    };
    assert_eq!(error.id, 17);
    assert_eq!(error.result, None);
    assert_eq!(
        error.error,
        Some(super::ResponseError {
            code: -32603,
            message: "client error".to_string()
        })
    );
}

#[test]
fn response_requires_exactly_one_payload_and_a_non_null_id() {
    for input in [
        json!({ "jsonrpc": "2.0", "id": 1 }),
        json!({ "jsonrpc": "2.0", "id": 1, "result": {}, "error": { "code": 1, "message": "bad" } }),
        json!({ "jsonrpc": "2.0", "id": Value::Null, "result": {} }),
        json!({ "jsonrpc": "1.0", "id": 1, "result": {} }),
    ] {
        assert_eq!(
            parse_message(&input.to_string()).err(),
            Some(RequestParseError::InvalidRequest),
            "invalid response should be rejected: {input}"
        );
    }
}
