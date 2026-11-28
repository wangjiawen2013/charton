# charton-cli

A table in, a chart out — no Rust toolchain, no Node, no Python.

`charton` is the command-line front end of the [charton](https://github.com/wangjiawen2013/charton)
engine. It reads CSV, TSV, NDJSON or stdin, and it renders the same JSON chart
document (a Vega-Lite v5 subset) that the MCP server and the Nushell plugin
use.

## Install

```sh
cargo install charton-cli
```

From a checkout:

```sh
cargo install --path charton-cli
```

Prebuilt binaries are not published yet; both commands build from source. The
crate is a workspace member, so `cargo build -p charton-cli --release` also
works.

## Sixty seconds

```sh
charton plot sales.csv --mark line --x month --y revenue -o revenue.svg
```

`charton plot --help` prints the exact flags for the installed build. The rest of
this README is the complete usage.

## Commands

```text
charton plot [INPUT] [OPTIONS]     # a table and a few flags
charton render <SPEC> [OPTIONS]    # a chart document
charton validate <SPEC> [OPTIONS]  # check before you draw
charton schema [OPTIONS]           # the JSON Schema
charton mcp [OPTIONS]              # the MCP server for agents
```

### `plot` — a table and a few flags

`INPUT` is a CSV, TSV or NDJSON file. Omit it, or pass `-`, to read stdin.

| Flag | Meaning |
|---|---|
| `--mark <MARK>` | What to draw. Defaults to `point`. |
| `--x <COLUMN>` | Horizontal position. |
| `--y <COLUMN>` | Vertical position. |
| `--y2 <COLUMN>` | Far end of a vertical span (`rule`, floating `bar` bodies). |
| `--color <COLUMN>` | Colour / grouping channel. |
| `--size <COLUMN>` | Mark size (points). |
| `--shape <COLUMN>` | Mark shape (points). |
| `--text <COLUMN>` | Label text (`text`). |
| `--path-group <COLUMN>` | Groups rows into one path or polygon. |
| `--z <COLUMN>` | Surface height, for `--mark contour`. |
| `--title <TEXT>` | Chart title. |
| `--description <TEXT>` | Accessible description. |
| `--emit-spec <FILE>` | Write the equivalent document (`-` for stdout). |
| `-o, --output <FILE>` | Output image; defaults to `chart.svg`. |
| `--force` | Overwrite existing files. |

`--mark` accepts both namespaces:

| Namespace | Marks |
|---|---|
| Vega-Lite | `point` `line` `bar` `area` `rect` `rule` `text` `tick` `boxplot` `errorbar` |
| Charton | `polygon` `path` `violin` `density` `density_2d` `contour` `hist` `beeswarm` `quantile_box` |

`quantile_box` is recognised but not implemented in this build; `contour` also
needs `--z`.

```sh
charton plot sales.csv --mark bar --x region --y revenue --color segment -o bars.svg
cat sales.csv | charton plot - --mark point --x revenue --y margin -o - > dots.svg
charton plot sales.csv --mark line --x month --y revenue --title Revenue -o trend.svg
```

`plot` is deliberately small. It has no `--transform`, `--layer` or `--facet`:
when a chart needs more than a handful of flags, write a document.

### `render` — a chart document

```sh
charton render chart.json -o report.pdf
```

`render` validates the document and writes the image. The document format is
described in [Writing a chart document](../docs/src/spec/index.md)
and the exact fields in [the field reference](../docs/src/spec/lowering-map.md).

The document names its data:

- inline `data.values`, or
- `data.url`, a **local path** resolved against the document's directory (with
  a fallback to the current directory). A URL with a scheme is refused.
- `url: "-"` means stdin, so a document emitted from a pipe replays through a
  pipe.

```sh
charton render chart.json -o report.svg
cat data.csv | charton render chart.json -o report.svg   # if data.url is "-"
```

`render` has no `--data`: the document is the single source of truth.

### `validate` — check before you draw

```sh
charton validate chart.json
charton validate chart.json --data sales.csv --json
```

Without `--json`, the human summary goes to stderr. With `--json`, the report is
the product and goes to **stdout**:

```json
{
  "ok": false,
  "errors": [
    {
      "code": "E_UNKNOWN_MARK",
      "at": "/mark",
      "message": "Unknown mark `linee`",
      "hint": "Did you mean `line`?",
      "expected": ["point", "line", "bar", "area"]
    }
  ]
}
```

Every problem has a stable code, a JSON Pointer into the document, a sentence,
and usually a hint. Exit code **4** means "the document has problems", which is a
normal linter outcome. `--data` adds the second tier: field names, value types,
and row-shape checks.

### `schema` — the document format, for tools

```sh
charton schema > charton.schema.json
charton schema -o charton.schema.json --force
```

Prints the JSON Schema of this build. Give it to an editor or an agent; it is the
same schema the `$schema` URL names.

### `mcp` — agents

```sh
charton mcp --data-dir ./data
```

A stdio [MCP](https://modelcontextprotocol.io) server. `--data-dir` is the only
directory the tools may read; `--allow-write` lets `export_spec` write inside it.
The complete tool and protocol reference is below. Add it to a client like any
other local server:

```json
{
  "mcpServers": {
    "charton": {
      "command": "charton",
      "args": ["mcp", "--data-dir", "/home/you/data"]
    }
  }
}
```

## From a command line to a document

`plot --emit-spec` writes the chart you just drew as a document:

```sh
charton plot sales.csv --mark bar --x month --y revenue --emit-spec chart.json -o a.svg
charton render chart.json -o b.svg
cmp a.svg b.svg        # identical
```

Both commands build the chart the same way, so the document *is* the chart. Edit
`chart.json` to reach everything the flags do not cover — transforms, layers,
facets, scales.

With stdin input the document records `"url": "-"`, so it replays through a pipe:

```sh
cat sales.csv | charton plot - --x month --y revenue --emit-spec chart.json -o a.svg
cat sales.csv | charton render chart.json -o b.svg
```

`--force` applies to both the emitted document and the image.

## Input

| Source | How |
|---|---|
| CSV (`.csv`) | First row is the header. A column is numeric only when every cell parses as a number. |
| TSV (`.tsv`) | Same, tab-separated. |
| NDJSON (`.ndjson`, `.jsonl`) | One JSON object per line; scalars only. |
| stdin | Omit the file, or pass `-`. The format is sniffed: `{` means NDJSON. |

Column typing is per column, not per cell: one word makes the whole column text,
so an `id` column is never split into numbers and nulls. Date strings are not
sniffed; they stay categorical.

A document's `data.url` is a path, not a URL, and it resolves against the
document's directory. Remote URLs are refused.

## Output

| Extension | Output |
|---|---|
| `.svg` | Vector, always available. |
| `.png` | CPU-rendered raster image. |
| `.pdf` | Tagged vector document. |

- No `-o` writes `chart.svg`.
- `-o -` writes to stdout. Stdout is only used for an SVG, a JSON report or the
  schema; diagnostics never land there.
- An existing file is kept unless `--force` is given.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | A bug in Charton (the message says so, and asks for a report) |
| 2 | The command line was wrong |
| 3 | The data could not be read or parsed |
| 4 | The document is invalid, or disagrees with the data |
| 5 | Building, rendering or writing failed |

## The MCP server

`charton mcp` serves seven tools over JSON-RPC 2.0, one JSON object per line.
Stdout is the transport: nothing else is ever written there. Logs go to stderr.

| Tool | Arguments | Returns |
|---|---|---|
| `get_schema` | — | The document format's JSON Schema, as text. |
| `profile_data` | `path`, `sample_rows?` | Columns, types, missing counts, ranges or distinct values, a sample. |
| `list_marks` | — | Every mark, its namespace, support, and required/refused channels. |
| `list_themes` | — | The themes `theme` accepts. |
| `validate_spec` | `spec`, `data?` | The same `{ "ok", "errors" }` result as `validate --json`. |
| `render_spec` | `spec`, `format?` | SVG as text, or PNG as an image block. Problems replace the chart. |
| `export_spec` | `spec`, `path`, `overwrite?` | Writes the validated document. Needs `--allow-write`. |

`spec` is either the document itself (a JSON object) or a path inside the data
directory. `data` is a path inside the data directory. In the server, every path
— including a spec's own `data.url` — is resolved against `--data-dir`, not
against the document file.

`profile_data` defaults to 5 sample rows (max 50). `render_spec` accepts `svg`
(default) or `png`; a PNG comes back as a base64 `image` content block.
`export_spec` validates first, will not create directories, and will not
overwrite without `overwrite: true`.

### Protocol

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"example","version":"0.1"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"validate_spec","arguments":{"spec":{"data":{"values":[{"region":"north","sales":12}]},"mark":"bar","encoding":{"x":{"field":"region"},"y":{"field":"sales"}}}}}}
```

Supported revisions: `2024-11-05`, `2025-03-26`, `2025-06-18`. The server
advertises only the `tools` capability. `ping` and `shutdown` return `{}`;
notifications get no answer.

Tool failures are **in band**: the result has `isError: true` and a readable
text block. JSON-RPC errors are reserved for protocol mistakes (`-32700` parse,
`-32600` invalid request, `-32601` unknown method/tool, `-32602` bad params).

### Safety

- **One data directory.** Every path is resolved against `--data-dir` before the check;
  `../` and symlinks cannot climb out.
- **Read-only by default.** `export_spec` refuses unless the server was started
  with `--allow-write`, and never creates directories.
- **Limits.** 64 MiB per input file, 32 MiB per output.
- **No code, no network.** A document is data: no expressions, no closures, no
  file access from the document, no sockets.

## Documentation

The README is the reference. The book goes deeper:

- [The command line](../docs/src/getting_started/cli.md) — why the CLI is four commands and how a chart is built.
- [Agents (MCP)](../docs/src/getting_started/mcp.md) — how the agent loop works, error handling, and the sandbox.
- [Writing a chart document](../docs/src/spec/index.md) — the document format.
- [Field reference](../docs/src/spec/lowering-map.md) — every field.

## License

Apache-2.0
