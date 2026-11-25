//! The MCP server, driven the way a client drives it.
//!
//! Every test speaks the real protocol over the real binary's stdin/stdout. That
//! is the only way to be sure of the two things that matter here: that stdout
//! carries protocol messages and nothing else, and that the sandbox holds.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

/// A scratch directory that cleans itself up.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "charton-mcp-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.subsec_nanos())
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create scratch dir");
        Self { path }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path.join(name);
        std::fs::write(&path, text).expect("write fixture");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

const SALES: &str = "month,revenue\nJan,12.5\nFeb,15\nMar,9.5\n";

/// A running server, and the pipes to talk to it.
struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Server {
    fn start(scratch: &Scratch, extra: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_charton"))
            .arg("mcp")
            .arg("--root")
            .arg(&scratch.path)
            .args(extra)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the server starts");

        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = BufReader::new(child.stdout.take().expect("stdout piped"));
        Self {
            child,
            stdin,
            stdout,
        }
    }

    /// Send a message and read exactly one response.
    fn exchange(&mut self, message: &Value) -> Value {
        self.send(message);
        self.read()
    }

    fn send(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}")
            .and_then(|()| self.stdin.flush())
            .expect("write a message");
    }

    fn read(&mut self) -> Value {
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .expect("read a response line");
        serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {line}"))
    }

    /// Send a notification, which must produce no response.
    fn notify(&mut self, message: &Value) {
        self.send(message);
    }

    fn call(&mut self, id: i64, tool: &str, arguments: Value) -> Value {
        self.exchange(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": tool, "arguments": arguments },
        }))
    }

    /// Ask for a known-good request after `id`, which proves the server survived
    /// whatever came before it.
    fn still_alive(&mut self, id: i64) -> Value {
        self.exchange(&json!({"jsonrpc": "2.0", "id": id, "method": "ping"}))
    }

    fn initialise(&mut self) -> Value {
        self.exchange(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" },
            },
        }))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The first content block of a tool result.
fn content_0(response: &Value) -> &Value {
    &response["result"]["content"][0]
}

fn is_error(response: &Value) -> bool {
    response["result"]["isError"].as_bool().expect("isError")
}

// ---------------------------------------------------------------------------
// Handshake and catalogue
// ---------------------------------------------------------------------------

#[test]
fn the_server_initialises_and_offers_its_tools() {
    let scratch = Scratch::new("init");
    let mut server = Server::start(&scratch, &[]);

    let initialize = server.initialise();
    assert_eq!(initialize["jsonrpc"], "2.0");
    assert_eq!(initialize["result"]["serverInfo"]["name"], "charton");
    assert_eq!(initialize["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(
        initialize["result"]["capabilities"]["tools"]["listChanged"],
        false
    );

    server.notify(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));

    let list = server.exchange(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    let tools = list["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("name"))
        .collect();

    assert_eq!(
        names,
        [
            "get_schema",
            "profile_data",
            "list_marks",
            "list_themes",
            "validate_spec",
            "render_spec",
            "export_spec",
        ]
    );
    // Every tool carries a schema, because a model needs one to call it.
    for tool in tools {
        assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
        assert!(
            tool["description"]
                .as_str()
                .is_some_and(|text| !text.is_empty())
        );
    }
}

#[test]
fn the_schema_tool_returns_the_dialect() {
    let scratch = Scratch::new("schema");
    let mut server = Server::start(&scratch, &[]);

    let response = server.call(1, "get_schema", json!({}));
    assert!(!is_error(&response));
    let text = content_0(&response)["text"].as_str().expect("text");
    let schema: Value = serde_json::from_str(text).expect("the schema is JSON");
    assert!(
        schema["$id"]
            .as_str()
            .is_some_and(|id| id.contains("charton"))
    );
}

#[test]
fn list_marks_says_what_each_mark_needs() {
    let scratch = Scratch::new("marks");
    let mut server = Server::start(&scratch, &[]);

    let response = server.call(1, "list_marks", json!({}));
    let text = content_0(&response)["text"].as_str().expect("text");
    let payload: Value = serde_json::from_str(text).expect("JSON");

    let marks = payload["marks"].as_array().expect("marks");
    let line = marks
        .iter()
        .find(|mark| mark["name"] == "line")
        .expect("line is listed");
    assert_eq!(line["namespace"], "vega");
    assert_eq!(line["supported"], true);
    assert_eq!(line["required"], json!(["x", "y"]));

    // A name the dialect knows but this build cannot draw says so.
    let quantile_box = marks
        .iter()
        .find(|mark| mark["name"] == "quantile_box")
        .expect("quantile_box is listed");
    assert_eq!(quantile_box["namespace"], "charton");
    assert_eq!(quantile_box["supported"], false);
}

#[test]
fn list_themes_lists_what_a_spec_may_name() {
    let scratch = Scratch::new("themes");
    let mut server = Server::start(&scratch, &[]);

    let response = server.call(1, "list_themes", json!({}));
    let text = content_0(&response)["text"].as_str().expect("text");
    let payload: Value = serde_json::from_str(text).expect("JSON");
    assert_eq!(payload["field"], "theme");
    assert_eq!(payload["themes"][0]["name"], "light");
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

#[test]
fn profile_data_describes_the_columns() {
    let scratch = Scratch::new("profile");
    scratch.write("sales.csv", SALES);
    let mut server = Server::start(&scratch, &[]);

    let response = server.call(1, "profile_data", json!({ "path": "sales.csv" }));
    assert!(!is_error(&response), "{response}");

    let text = content_0(&response)["text"].as_str().expect("text");
    let profile: Value = serde_json::from_str(text).expect("JSON");
    assert_eq!(profile["rows"], 3);

    let columns = profile["columns"].as_array().expect("columns");
    let revenue = columns
        .iter()
        .find(|column| column["name"] == "revenue")
        .expect("revenue column");
    assert_eq!(revenue["type"], "continuous");
    assert_eq!(revenue["missing"], 0);
    assert_eq!(revenue["min"], 9.5);

    let month = columns
        .iter()
        .find(|column| column["name"] == "month")
        .expect("month column");
    assert_eq!(month["type"], "categorical");

    assert_eq!(profile["sample"][0]["month"], "Jan");
}

// ---------------------------------------------------------------------------
// Validation and rendering
// ---------------------------------------------------------------------------

#[test]
fn validate_spec_returns_actionable_diagnostics() {
    let scratch = Scratch::new("validate");
    scratch.write("sales.csv", SALES);
    let mut server = Server::start(&scratch, &[]);

    let response = server.call(
        1,
        "validate_spec",
        json!({ "spec": {
            "data": { "url": "sales.csv" },
            "mark": "linee",
            "encoding": { "x": { "field": "month" } }
        }}),
    );
    assert!(is_error(&response), "a bad spec must be a tool error");

    let text = content_0(&response)["text"].as_str().expect("text");
    let report: Value = serde_json::from_str(text).expect("JSON");
    assert_eq!(report["errors"][0]["code"], "E_UNKNOWN_MARK");
    assert_eq!(report["errors"][0]["at"], "/mark");
    assert_eq!(report["errors"][0]["hint"], "Did you mean `line`?");
}

#[test]
fn validate_spec_checks_the_data_when_given_it() {
    let scratch = Scratch::new("validate-data");
    scratch.write("sales.csv", SALES);
    let mut server = Server::start(&scratch, &[]);

    let response = server.call(
        1,
        "validate_spec",
        json!({
            "spec": {
                "data": { "url": "sales.csv" },
                "mark": "line",
                "encoding": { "x": { "field": "month" }, "y": { "field": "profit" } }
            },
            "data": "sales.csv",
        }),
    );
    assert!(is_error(&response));

    let text = content_0(&response)["text"].as_str().expect("text");
    let report: Value = serde_json::from_str(text).expect("JSON");
    assert_eq!(report["errors"][0]["code"], "E_DATA_FIELD");
    assert_eq!(report["errors"][0]["at"], "/encoding/y/field");
}

#[test]
fn render_spec_returns_svg_as_text() {
    let scratch = Scratch::new("render");
    scratch.write("sales.csv", SALES);
    let mut server = Server::start(&scratch, &[]);

    let response = server.call(
        1,
        "render_spec",
        json!({ "spec": {
            "data": { "url": "sales.csv" },
            "mark": "line",
            "encoding": { "x": { "field": "month" }, "y": { "field": "revenue" } }
        }}),
    );
    assert!(!is_error(&response), "{response}");
    assert_eq!(content_0(&response)["type"], "text");
    assert!(
        content_0(&response)["text"]
            .as_str()
            .is_some_and(|text| text.starts_with("<svg"))
    );
}

#[test]
fn render_spec_returns_png_as_an_image() {
    let scratch = Scratch::new("png");
    scratch.write("sales.csv", SALES);
    let mut server = Server::start(&scratch, &[]);

    let response = server.call(
        1,
        "render_spec",
        json!({
            "spec": {
                "data": { "url": "sales.csv" },
                "mark": "bar",
                "encoding": { "x": { "field": "month" }, "y": { "field": "revenue" } }
            },
            "format": "png",
        }),
    );
    assert!(!is_error(&response), "{response}");

    let content = content_0(&response);
    assert_eq!(content["type"], "image");
    assert_eq!(content["mimeType"], "image/png");
    let data = content["data"].as_str().expect("base64 payload");
    assert!(data.starts_with("iVBORw0KGgo"), "not a PNG payload");
}

#[test]
fn a_spec_that_cannot_render_returns_its_diagnostics_instead_of_a_chart() {
    let scratch = Scratch::new("render-bad");
    scratch.write("sales.csv", SALES);
    let mut server = Server::start(&scratch, &[]);

    let response = server.call(
        1,
        "render_spec",
        json!({ "spec": { "data": { "url": "sales.csv" }, "mark": "line" } }),
    );
    assert!(is_error(&response));
    let text = content_0(&response)["text"].as_str().expect("text");
    assert!(text.contains("E_MISSING_FIELD"), "{text}");
}

// ---------------------------------------------------------------------------
// The sandbox
// ---------------------------------------------------------------------------

#[test]
fn a_path_outside_the_root_is_refused() {
    let scratch = Scratch::new("escape");
    scratch.write("sales.csv", SALES);
    let mut server = Server::start(&scratch, &[]);

    for path in ["../outside.csv", "/etc/passwd", "sub/../../outside.csv"] {
        let response = server.call(1, "profile_data", json!({ "path": path }));
        assert!(is_error(&response), "`{path}` must be refused: {response}");
        let text = content_0(&response)["text"].as_str().expect("text");
        assert!(
            text.contains("allowed root") || text.contains("cannot read"),
            "unexpected message: {text}"
        );
    }

    // Still serving after three refusals.
    assert!(server.still_alive(9)["result"].is_object());
}

#[test]
fn a_spec_cannot_read_data_outside_the_root() {
    let scratch = Scratch::new("escape-spec");
    let mut server = Server::start(&scratch, &[]);

    let response = server.call(
        1,
        "render_spec",
        json!({ "spec": {
            "data": { "url": "../../../etc/passwd" },
            "mark": "point",
            "encoding": { "x": { "field": "a" }, "y": { "field": "b" } }
        }}),
    );
    assert!(is_error(&response));
}

#[test]
fn export_spec_is_disabled_until_asked_for() {
    let scratch = Scratch::new("export-off");
    scratch.write("sales.csv", SALES);
    let mut server = Server::start(&scratch, &[]);

    let spec = json!({
        "data": { "url": "sales.csv" },
        "mark": "point",
        "encoding": { "x": { "field": "month" }, "y": { "field": "revenue" } }
    });
    let response = server.call(
        1,
        "export_spec",
        json!({ "spec": spec, "path": "out.json" }),
    );
    assert!(is_error(&response));
    assert!(!scratch.path.join("out.json").exists());
}

#[test]
fn export_spec_writes_a_canonical_document() {
    let scratch = Scratch::new("export-on");
    scratch.write("sales.csv", SALES);
    let mut server = Server::start(&scratch, &["--allow-write"]);

    let spec = json!({
        "data": { "url": "sales.csv" },
        "mark": "point",
        "encoding": { "x": { "field": "month" }, "y": { "field": "revenue" } }
    });
    let response = server.call(
        1,
        "export_spec",
        json!({ "spec": spec, "path": "out.json" }),
    );
    assert!(!is_error(&response), "{response}");

    let written = std::fs::read_to_string(scratch.path.join("out.json")).expect("file written");
    let parsed: Value = serde_json::from_str(&written).expect("valid JSON");
    assert_eq!(parsed["mark"], "point");
    // Canonical: a trailing newline, and defaults omitted.
    assert!(written.ends_with('\n'));
    assert!(parsed["data"]["url"].is_string());
}

// ---------------------------------------------------------------------------
// Protocol hygiene
// ---------------------------------------------------------------------------

#[test]
fn a_broken_line_is_reported_and_the_server_survives() {
    let scratch = Scratch::new("garbage");
    let mut server = Server::start(&scratch, &[]);

    server.send(&json!({ "this is": "not a message" }));
    writeln!(server.stdin, "{{not json").expect("write garbage");
    server.stdin.flush().expect("flush");

    // The first line is a JSON object without a method; the second is not JSON.
    let first = server.read();
    let second = server.read();
    assert!(
        first["error"]["code"] == -32600 || first["error"]["code"] == -32700,
        "{first}"
    );
    assert!(
        second["error"]["code"] == -32700 || second["error"]["code"] == -32600,
        "{second}"
    );

    assert!(server.still_alive(42)["result"].is_object());
}

#[test]
fn an_unknown_method_and_an_unknown_tool_are_protocol_errors() {
    let scratch = Scratch::new("unknown");
    let mut server = Server::start(&scratch, &[]);

    let method = server.exchange(&json!({"jsonrpc": "2.0", "id": 1, "method": "nope"}));
    assert_eq!(method["error"]["code"], -32601);

    let tool = server.exchange(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": { "name": "nope", "arguments": {} },
    }));
    assert_eq!(tool["error"]["code"], -32602);
    assert!(
        tool["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("profile_data")),
        "the error must list what exists: {tool}"
    );
}
