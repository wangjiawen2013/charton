//! The CLI, driven as a user drives it.
//!
//! These tests run the real binary and assert on what a script would see: the
//! exit code, stdout, stderr and the files on disk. The release checklist in the
//! plan depends on exactly these things, so they are tested here rather than
//! inferred from unit tests.

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// A scratch directory that cleans itself up.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "charton-cli-{}-{}-{name}",
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

    fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Run the binary with args, from the scratch directory.
fn run(scratch: &Scratch, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_charton"))
        .args(args)
        .current_dir(&scratch.path)
        .output()
        .expect("the binary runs")
}

/// Run the binary with stdin.
fn run_with_stdin(scratch: &Scratch, args: &[&str], stdin: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_charton"))
        .args(args)
        .current_dir(&scratch.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("the binary finishes")
}

fn code(output: &std::process::Output) -> i32 {
    output.status.code().expect("the process exited normally")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const SALES: &str = "month,revenue\nJan,12.5\nFeb,15\nMar,9.5\n";

fn sales(scratch: &Scratch) -> PathBuf {
    scratch.write("sales.csv", SALES)
}

// ---------------------------------------------------------------------------
// schema
// ---------------------------------------------------------------------------

#[test]
fn schema_prints_a_json_document() {
    let scratch = Scratch::new("schema");
    let output = run(&scratch, &["schema"]);

    assert_eq!(code(&output), 0, "stderr: {}", stderr(&output));
    let schema: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("valid JSON");
    assert!(
        schema["$id"]
            .as_str()
            .is_some_and(|id| id.contains("charton-v0.schema.json")),
        "the schema must carry Charton's own URL"
    );
}

// ---------------------------------------------------------------------------
// plot
// ---------------------------------------------------------------------------

#[test]
fn plot_writes_an_svg() {
    let scratch = Scratch::new("plot");
    let input = sales(&scratch);

    let output = run(
        &scratch,
        &[
            "plot",
            input.to_str().unwrap(),
            "--mark",
            "line",
            "--x",
            "month",
            "--y",
            "revenue",
            "-o",
            "chart.svg",
        ],
    );

    assert_eq!(code(&output), 0, "stderr: {}", stderr(&output));
    let svg = std::fs::read_to_string(scratch.join("chart.svg")).expect("svg written");
    assert!(svg.contains("<svg"), "not an SVG");
    // Success is quiet: nothing to parse on stdout.
    assert!(stdout(&output).is_empty());
}

#[test]
fn plot_refuses_to_overwrite_without_force() {
    let scratch = Scratch::new("force");
    let input = sales(&scratch);
    scratch.write("chart.svg", "existing");

    let args = [
        "plot",
        input.to_str().unwrap(),
        "--x",
        "month",
        "--y",
        "revenue",
        "-o",
        "chart.svg",
    ];
    let output = run(&scratch, &args);
    assert_eq!(code(&output), 2);
    assert!(stderr(&output).contains("--force"));
    assert_eq!(
        std::fs::read_to_string(scratch.join("chart.svg")).unwrap(),
        "existing",
        "the existing file must be untouched"
    );

    let mut forced = args.to_vec();
    forced.push("--force");
    let output = run(&scratch, &forced);
    assert_eq!(code(&output), 0, "stderr: {}", stderr(&output));
}

#[test]
fn plot_reads_stdin_and_writes_stdout() {
    let scratch = Scratch::new("stdin");
    let output = run_with_stdin(
        &scratch,
        &["plot", "-", "--x", "month", "--y", "revenue", "-o", "-"],
        SALES,
    );

    assert_eq!(code(&output), 0, "stderr: {}", stderr(&output));
    assert!(stdout(&output).starts_with("<svg"));
}

#[test]
fn an_unknown_extension_is_a_usage_error() {
    let scratch = Scratch::new("ext");
    let input = sales(&scratch);

    let output = run(
        &scratch,
        &[
            "plot",
            input.to_str().unwrap(),
            "--x",
            "month",
            "--y",
            "revenue",
            "-o",
            "chart.txt",
        ],
    );

    assert_eq!(code(&output), 2);
    assert!(stderr(&output).contains(".svg"));
}

#[test]
fn a_missing_input_file_is_a_data_error() {
    let scratch = Scratch::new("missing");
    let output = run(&scratch, &["plot", "nope.csv", "--x", "a", "--y", "b"]);

    assert_eq!(code(&output), 3);
    assert!(stderr(&output).contains("nope.csv"));
}

// ---------------------------------------------------------------------------
// The invariant: a spec is the chart, not a description of one
// ---------------------------------------------------------------------------

#[test]
fn an_emitted_spec_renders_to_the_same_image() {
    let scratch = Scratch::new("roundtrip");
    let input = sales(&scratch);

    let plot = run(
        &scratch,
        &[
            "plot",
            input.to_str().unwrap(),
            "--mark",
            "bar",
            "--x",
            "month",
            "--y",
            "revenue",
            "--title",
            "Revenue",
            "--emit-spec",
            "chart.json",
            "-o",
            "a.svg",
        ],
    );
    assert_eq!(code(&plot), 0, "stderr: {}", stderr(&plot));

    let render = run(&scratch, &["render", "chart.json", "-o", "b.svg"]);
    assert_eq!(code(&render), 0, "stderr: {}", stderr(&render));

    let plotted = std::fs::read(scratch.join("a.svg")).expect("a.svg");
    let rendered = std::fs::read(scratch.join("b.svg")).expect("b.svg");
    assert_eq!(
        plotted, rendered,
        "render(emit-spec(plot)) must equal plot — there is a second build path somewhere"
    );
}

#[test]
fn a_spec_emitted_from_stdin_replays_through_stdin() {
    let scratch = Scratch::new("stdin-replay");

    let emitted = run_with_stdin(
        &scratch,
        &[
            "plot",
            "-",
            "--x",
            "month",
            "--y",
            "revenue",
            "--emit-spec",
            "-",
            "-o",
            "a.svg",
        ],
        SALES,
    );
    assert_eq!(code(&emitted), 0, "stderr: {}", stderr(&emitted));

    let spec: serde_json::Value = serde_json::from_str(&stdout(&emitted)).expect("valid JSON");
    assert_eq!(
        spec["data"]["url"], "-",
        "a spec read from stdin records stdin as its source"
    );

    let spec_path = scratch.write("chart.json", &stdout(&emitted));
    let rendered = run_with_stdin(
        &scratch,
        &["render", spec_path.to_str().unwrap(), "-o", "-"],
        SALES,
    );
    assert_eq!(code(&rendered), 0, "stderr: {}", stderr(&rendered));
    assert!(stdout(&rendered).starts_with("<svg"));
}

// ---------------------------------------------------------------------------
// validate
// ---------------------------------------------------------------------------

#[test]
fn validate_reports_problems_as_json_and_exits_4() {
    let scratch = Scratch::new("validate");
    let spec = scratch.write(
        "bad.json",
        r#"{"data":{"url":"sales.csv"},"mark":"linee","encoding":{"x":{"field":"month"}}}"#,
    );

    let output = run(&scratch, &["validate", spec.to_str().unwrap(), "--json"]);
    assert_eq!(code(&output), 4);

    let report: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("valid JSON");
    assert_eq!(report["ok"], false);
    assert_eq!(report["errors"][0]["code"], "E_UNKNOWN_MARK");
    assert_eq!(report["errors"][0]["at"], "/mark");
    // The human text stays on stderr in JSON mode.
    assert!(stderr(&output).is_empty());
}

#[test]
fn validate_explains_problems_in_text() {
    let scratch = Scratch::new("validate-text");
    let spec = scratch.write(
        "bad.json",
        r#"{"data":{"url":"sales.csv"},"mark":"linee","encoding":{"x":{"field":"month"}}}"#,
    );

    let output = run(&scratch, &["validate", spec.to_str().unwrap()]);
    assert_eq!(code(&output), 4);

    let text = stderr(&output);
    assert!(text.contains("E_UNKNOWN_MARK"));
    assert!(text.contains("/mark"));
    assert!(text.contains("Did you mean `line`?"));
}

#[test]
fn validate_checks_the_data_when_asked() {
    let scratch = Scratch::new("validate-data");
    sales(&scratch);
    let spec = scratch.write(
        "chart.json",
        r#"{"data":{"url":"sales.csv"},"mark":"line","encoding":{"x":{"field":"month"},"y":{"field":"revenue"}}}"#,
    );

    let output = run(
        &scratch,
        &[
            "validate",
            spec.to_str().unwrap(),
            "--data",
            "sales.csv",
            "--json",
        ],
    );
    assert_eq!(code(&output), 0, "stderr: {}", stderr(&output));

    let wrong = scratch.write(
        "wrong.json",
        r#"{"data":{"url":"sales.csv"},"mark":"line","encoding":{"x":{"field":"month"},"y":{"field":"profit"}}}"#,
    );
    let output = run(
        &scratch,
        &[
            "validate",
            wrong.to_str().unwrap(),
            "--data",
            "sales.csv",
            "--json",
        ],
    );
    assert_eq!(code(&output), 4);
    let report: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("valid JSON");
    assert_eq!(report["errors"][0]["code"], "E_DATA_FIELD");
    assert_eq!(report["errors"][0]["at"], "/encoding/y/field");
}

#[test]
fn a_spec_with_problems_is_reported_before_the_file_is_touched() {
    let scratch = Scratch::new("validate-nodata");
    let spec = scratch.write("chart.json", r#"{"mark":"line"}"#);

    let output = run(&scratch, &["validate", spec.to_str().unwrap()]);
    assert_eq!(code(&output), 4);
    assert!(
        stderr(&output).contains("/data"),
        "stderr: {}",
        stderr(&output)
    );
}

// ---------------------------------------------------------------------------
// render
// ---------------------------------------------------------------------------

#[test]
fn render_reports_a_missing_data_file_with_the_paths_it_tried() {
    let scratch = Scratch::new("render-missing-data");
    let spec = scratch.write(
        "chart.json",
        r#"{"data":{"url":"gone.csv"},"mark":"line","encoding":{"x":{"field":"a"},"y":{"field":"b"}}}"#,
    );

    let output = run(&scratch, &["render", spec.to_str().unwrap(), "-o", "-"]);
    assert_eq!(code(&output), 3);
    let text = stderr(&output);
    assert!(text.contains("gone.csv"), "stderr: {text}");
}

#[test]
fn help_and_version_are_available() {
    let scratch = Scratch::new("help");
    for flag in ["--help", "--version"] {
        let output = run(&scratch, &[flag]);
        assert_eq!(code(&output), 0, "`{flag}` must succeed");
        assert!(!stdout(&output).is_empty());
    }
}
