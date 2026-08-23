//! Minimal JSON-RPC 2.0 types for the MCP stdio transport.
//!
//! MCP frames each message as one line of JSON on stdin/stdout. We deserialize
//! requests and serialize responses with `serde_json` (already a dependency),
//! keeping the transport free of an async runtime.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC error code for an unknown method or tool.
pub const METHOD_NOT_FOUND: i32 = -32601;

/// JSON-RPC error codes for invalid transport input.
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestParseError {
    Parse,
    InvalidRequest,
}

/// An incoming JSON-RPC request or notification. Notifications omit `id` and
/// receive no response.
#[derive(Debug, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

pub fn parse_request(line: &str) -> Result<Request, RequestParseError> {
    let value = serde_json::from_str::<Value>(line).map_err(|_| RequestParseError::Parse)?;
    let explicit_id = value.get("id").cloned();
    let valid_params = value
        .get("params")
        .is_none_or(|params| params.is_array() || params.is_object());
    let mut request =
        serde_json::from_value::<Request>(value).map_err(|_| RequestParseError::InvalidRequest)?;
    request.id = explicit_id;
    let valid_id = request
        .id
        .as_ref()
        .is_none_or(|id| id.is_null() || id.is_string() || id.is_number());
    if request.jsonrpc != "2.0" || !valid_id || !valid_params {
        return Err(RequestParseError::InvalidRequest);
    }
    Ok(request)
}

/// An outgoing JSON-RPC response. Exactly one of `result`/`error` is set.
#[derive(Debug, Serialize)]
pub struct Response {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
}

impl Response {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(ResponseError {
                code,
                message: message.into(),
            }),
        }
    }
}
