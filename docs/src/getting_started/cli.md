# The command line

`charton` turns a table into a chart without a Rust toolchain. It is one static
binary: no Node, no Python, no container.

The **complete reference** — every flag, the mark list, input/output rules, the
MCP tools and the raw protocol — is the
[CLI README](https://github.com/wangjiawen2013/charton/blob/main/charton-cli/README.md).
This chapter is the design behind it: why there are four commands, how a chart
is built, and what the exit codes promise.

## Four commands, on purpose

```text
charton plot [INPUT] ...        a table and a few flags
charton render <SPEC> ...       a chart document
charton validate <SPEC> ...     check before you draw
charton schema                  the dialect, for editors and agents
charton mcp                     the agent server
```

`plot` is the short path. It exposes only the common encodings (`--x`, `--y`,
`--y2`, `--color`, `--size`, `--shape`, `--text`, `--path-group`, `--z`) and
deliberately has no `--transform`, `--layer` or `--facet`. A flag for every core
capability is how a command line turns into a second, worse dialect.

When a chart outgrows the flags, it becomes a document. `render` takes that
document, and `validate` checks it without drawing. `schema` is the same dialect
as a JSON Schema, for an editor or an agent.

## One build path

`plot` does not have its own renderer. Its flags build the `Spec` a person would
have written, and that spec goes through the same validation and lowering as
`render`:

```text
plot flags ──► Spec ──┐
                      ├──► validate ──► lower ──► SVG/PNG/PDF
chart.json ──► Spec ──┘
```

That is why `--emit-spec` is meaningful: the document **is** the chart that was
drawn, not a description of a parallel code path.

```bash
charton plot sales.csv --mark bar --x month --y revenue --emit-spec chart.json -o a.svg
charton render chart.json -o b.svg
cmp a.svg b.svg        # identical
```

It is also why a document emitted from stdin records `"url": "-"` and replays
through a pipe:

```bash
cat sales.csv | charton plot - --x month --y revenue --emit-spec chart.json -o a.svg
cat sales.csv | charton render chart.json -o b.svg
```

The rule behind both: **one meaning, one build path**. The CLI is not a second
semantics system beside the library.

## Input and output

The short version:

- Input is CSV, TSV, NDJSON or stdin. Column typing is per column, not per cell;
  date strings stay categorical.
- A document's `data.url` is a local path, resolved against the spec file's
  directory. Remote URLs are refused.
- Output is chosen by extension: `.svg`, `.png`, `.pdf`. No `-o` writes
  `chart.svg`; `-o -` writes an SVG to stdout; `--force` overwrites.
- Diagnostics go to stderr, artifacts to stdout. `-o -` pipes cleanly.

The full tables are in the [CLI README](https://github.com/wangjiawen2013/charton/blob/main/charton-cli/README.md#input).

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | A bug in Charton (the message says so, and asks for a report) |
| 2 | The command line was wrong |
| 3 | The data could not be read or parsed |
| 4 | The document is invalid, or disagrees with the data |
| 5 | Building, rendering or writing failed |

Exit code 4 is a linter outcome, not a crash: `validate --json` prints the
report on stdout and leaves the exit code to the caller.

## What it deliberately does not do

- No flag per core capability. Complex charts are documents.
- No expression evaluation, ever. A document is data, not a program.
- No `profile`, `init` or `examples` subcommands yet; `profile_data` exists as an
  MCP tool.
