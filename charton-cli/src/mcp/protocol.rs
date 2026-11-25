//! JSON-RPC 2.0, the framing MCP speaks over stdio.
//!
//! One JSON object per line in each direction. Nothing but protocol messages may
//! ever be written to stdout: it is the transport, so a stray `println!` corrupts
//! the stream. Diagnostics go to stderr.

use serde_json::{Value, json};

/// Our preferred revision. Newer clients may ask for another one they support;
/// only `tools/*` is used here, which has not changed across these revisions.
pub const PREFERRED_VERSION: &str = "2025-06-18";

/// Revisions this server can serve.
pub const SUPPORTED_VERSIONS: &[&str] = &["2024-11-05", "2025-03-26", "2025-06-18"];

/// Parse error.
pub const PARSE_ERROR: i32 = -32700;
/// The message is not a valid request.
pub const INVALID_REQUEST: i32 = -32600;
/// No such method.
pub const METHOD_NOT_FOUND: i32 = -32601;
/// The method's arguments are wrong.
pub const INVALID_PARAMS: i32 = -32602;

/// A message that arrived.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Message {
    /// Must be `"2.0"`.
    #[serde(default)]
    pub jsonrpc: Option<String>,
    /// Present on requests, absent on notifications.
    #[serde(default)]
    pub id: Option<Value>,
    /// The method name.
    #[serde(default)]
    pub method: Option<String>,
    /// Method arguments.
    #[serde(default)]
    pub params: Option<Value>,
}

/// A successful response.
#[must_use]
pub fn success(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// A failed response.
#[must_use]
pub fn failure(id: Value, code: i32, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message.into()},
    })
}

/// The revision to answer with: the client's if we serve it, ours otherwise.
#[must_use]
pub fn negotiate(requested: Option<&str>) -> &'static str {
    requested
        .and_then(|version| {
            SUPPORTED_VERSIONS
                .iter()
                .copied()
                .find(|known| *known == version)
        })
        .unwrap_or(PREFERRED_VERSION)
}
