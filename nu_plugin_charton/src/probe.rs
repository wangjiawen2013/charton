//! Diagnostic command: report how Nushell launched this plugin and whether the
//! plugin's stdout actually reaches the user's terminal.
//!
//! Context: with the `nu-plugin` protocol, stdin/stdout are the protocol
//! channel in **stdio mode**, so writing to stdout would corrupt it. In
//! **local-socket mode** the protocol goes over a socket/named pipe, which
//! leaves stdout free for inline terminal rendering. This command reports which
//! mode we got and, when safe, emits an ANSI probe so we can confirm visually.

use std::io::{IsTerminal, Write};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{IntoPipelineData, LabeledError, PipelineData, Signature, Type, Value, record};

use crate::ChartonPlugin;

pub struct Probe;

impl PluginCommand for Probe {
    type Plugin = ChartonPlugin;

    fn name(&self) -> &str {
        "charton-probe"
    }

    fn description(&self) -> &str {
        "Diagnose the plugin transport (stdio vs local socket) and whether stdout reaches the terminal"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .input_output_type(Type::Any, Type::Any)
            .switch(
                "force",
                "Attempt the ANSI probe even if stdout is not detected as a TTY (never in stdio mode)",
                Some('f'),
            )
    }

    fn run(
        &self,
        _plugin: &ChartonPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let span = call.head;

        let using_stdio = engine.is_using_stdio();
        let stdout_is_tty = std::io::stdout().is_terminal();
        let stderr_is_tty = std::io::stderr().is_terminal();
        let force = call.has_flag("force")?;

        // NEVER write to stdout in stdio mode: it is the protocol channel.
        let attempt_probe = !using_stdio && (stdout_is_tty || force);

        let mut probe_written = false;
        if attempt_probe {
            let mut out = std::io::stdout();
            // Truecolor half blocks. If these show up red/green/blue, stdout
            // reaches the terminal and inline rendering is viable.
            let _ = writeln!(
                out,
                "\x1b[38;2;255;80;80m██\x1b[38;2;80;255;120m██\x1b[38;2;120;160;255m██\x1b[0m \
                 PROBE_STDOUT_REACHES_TERMINAL"
            );
            let _ = out.flush();
            probe_written = true;
        }

        // stderr is always inherited by Nushell and safe for diagnostics.
        eprintln!(
            "[charton-probe] using_stdio={using_stdio} stdout_tty={stdout_is_tty} \
             stderr_tty={stderr_is_tty} force={force} probe_written={probe_written}"
        );

        let env = |k: &str| match std::env::var(k) {
            Ok(v) => Value::string(v, span),
            Err(_) => Value::nothing(span),
        };

        let value = Value::record(
            record! {
                "using_stdio" => Value::bool(using_stdio, span),
                "stdout_is_terminal" => Value::bool(stdout_is_tty, span),
                "stderr_is_terminal" => Value::bool(stderr_is_tty, span),
                "can_print_inline" => Value::bool(!using_stdio && stdout_is_tty, span),
                "probe_written" => Value::bool(probe_written, span),
                "TERM" => env("TERM"),
                "COLORTERM" => env("COLORTERM"),
                "TERM_PROGRAM" => env("TERM_PROGRAM"),
                "WT_SESSION" => env("WT_SESSION"),
                "KITTY_WINDOW_ID" => env("KITTY_WINDOW_ID"),
                "WEZTERM_EXECUTABLE" => env("WEZTERM_EXECUTABLE"),
            },
            span,
        );

        Ok(value.into_pipeline_data())
    }
}
