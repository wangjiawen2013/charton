//! The stdio message loop.
//!
//! stdout carries protocol messages and nothing else; a stray line corrupts the
//! stream. Logs go to stderr. An unknown method or a malformed message is
//! answered with a JSON-RPC error and the loop continues.
use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::errors::CliError;
use crate::sandbox::Sandbox;

use super::protocol::{self, Message};
use super::tools::{self, Call, Content, Outcome};

/// Serve MCP over stdin/stdout until stdin closes.
///
/// # Errors
///
/// Fails when stdin cannot be read or stdout cannot be written.
pub fn serve(sandbox: Sandbox) -> Result<(), CliError> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let mut state = State::default();

    for line in stdin.lock().lines() {
        let line = line.map_err(|error| CliError::data(format!("cannot read stdin: {error}")))?;
        if line.trim().is_empty() {
            continue;
        }

        if let Some(response) = handle(&line, &sandbox, &mut state) {
            write_message(&mut stdout, &response)?;
        }

        if state.closed {
            break;
        }
    }

    Ok(())
}

#[derive(Default)]
struct State {
    /// Set by a `shutdown` request, honoured before the transport closes.
    closed: bool,
}

fn handle(line: &str, sandbox: &Sandbox, state: &mut State) -> Option<Value> {
    let message: Message = match serde_json::from_str(line) {
        Ok(message) => message,
        Err(error) => {
            return Some(protocol::failure(
                Value::Null,
                protocol::PARSE_ERROR,
                format!("cannot parse the message: {error}"),
            ));
        }
    };

    if message.jsonrpc.as_deref() != Some("2.0") {
        return Some(protocol::failure(
            message.id.unwrap_or(Value::Null),
            protocol::INVALID_REQUEST,
            "jsonrpc must be \"2.0\"",
        ));
    }

    let method = message.method.unwrap_or_default();
    let params = message.params.unwrap_or(Value::Null);

    // A notification has no id and gets no answer. Requests are served even
    // before `initialized`: the client knows its own business, and refusing here
    // only turns a protocol slip into a mystery.
    let id = message.id?;

    let outcome = match method.as_str() {
        "initialize" => Ok(initialize_result(&params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_list() })),
        "tools/call" => call_tool(&params, sandbox),
        "shutdown" => {
            state.closed = true;
            Ok(json!({}))
        }
        other => Err((
            protocol::METHOD_NOT_FOUND,
            format!("unknown method `{other}`"),
        )),
    };

    Some(match outcome {
        Ok(result) => protocol::success(id, result),
        Err((code, message)) => protocol::failure(id, code, message),
    })
}

fn initialize_result(params: &Value) -> Value {
    let requested = params.get("protocolVersion").and_then(Value::as_str);

    json!({
        "protocolVersion": protocol::negotiate(requested),
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "charton", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "Charton renders charts from a subset of Vega-Lite v5. \
                         Read get_schema before writing a spec, profile_data before \
                         naming columns, validate_spec before rendering, and render_spec \
                         to get the image. Everything is checked; nothing is guessed.",
    })
}

fn tool_list() -> Vec<Value> {
    tools::all()
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": (tool.schema)(),
            })
        })
        .collect()
}

/// Run one tool. `Err` is a protocol-level mistake; a tool's own failure comes
/// back inside the result, where the model can read it.
fn call_tool(params: &Value, sandbox: &Sandbox) -> Result<Value, (i32, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((protocol::INVALID_PARAMS, "`name` is missing".to_owned()))?;

    let tool = tools::find(name).ok_or((
        protocol::INVALID_PARAMS,
        format!(
            "unknown tool `{name}`; available: {}",
            tools::all()
                .iter()
                .map(|tool| tool.name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ))?;

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    Ok(outcome_to_value((tool.call)(Call {
        arguments: &arguments,
        sandbox,
    })))
}

fn outcome_to_value(outcome: Outcome) -> Value {
    let content: Vec<Value> = outcome
        .content
        .into_iter()
        .map(|content| match content {
            Content::Text(text) => json!({ "type": "text", "text": text }),
            Content::Image { data, mime } => {
                json!({ "type": "image", "data": data, "mimeType": mime })
            }
        })
        .collect();

    json!({ "content": content, "isError": outcome.is_error })
}

fn write_message(out: &mut impl Write, message: &Value) -> Result<(), CliError> {
    serde_json::to_writer(&mut *out, message)
        .map_err(|error| CliError::render(format!("cannot write a response: {error}")))?;
    out.write_all(b"\n")
        .and_then(|()| out.flush())
        .map_err(|error| CliError::render(format!("cannot write a response: {error}")))
}
