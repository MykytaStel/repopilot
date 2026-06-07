//! Minimal JSON-RPC 2.0 types for the MCP stdio transport.
//!
//! MCP frames each message as one line of JSON on stdin/stdout. We deserialize
//! requests and serialize responses with `serde_json` (already a dependency),
//! keeping the transport free of an async runtime.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC error code for an unknown method or tool.
pub const METHOD_NOT_FOUND: i32 = -32601;

/// An incoming JSON-RPC request or notification. Notifications omit `id` and
/// receive no response.
#[derive(Debug, Deserialize)]
pub struct Request {
    #[allow(dead_code)]
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
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
