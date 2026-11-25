# The command line

`charton` turns a table into a chart without a Rust toolchain. It is one static
binary: no Node, no Python, no container.

## Install

```bash
cargo install --path charton-cli      # from a checkout
```

Prebuilt binaries for linux-x86_64, macos-x86_64, macos-aarch64 and
windows-x86_64 are attached to each release.

## Sixty seconds

```bash
charton plot sales.csv --mark line --x month --y revenue -o revenue.svg
```

`charton plot --help` lists the flags. They cover the common cases on purpose:
when a chart needs more than a handful of flags, it should be a document, not a
longer command line.

## The four commands

### `plot` — a table and a few flags

```bash
charton plot sales.csv --mark bar --x region --y revenue --color segment -o bars.svg
cat sales.csv | charton plot - --mark point --x revenue --y margin -o - > dots.svg
```

### `render` — a chart document

```bash
charton render chart.json -o report.pdf
```

A document is a subset of Vega-Lite v5; [Writing a chart document](../spec/index.md)
describes its parts, and the [lowering map](../spec/lowering-map.md) the exact
field reference. `charton schema` prints the JSON Schema to teach your editor —
or an agent — what is allowed.

### `validate` — check before you draw

```bash
charton validate chart.json
charton validate chart.json --data sales.csv --json
```

The JSON report is the product, so it goes to **stdout**; the human version goes
to stderr. Every problem has a stable code, a JSON Pointer to the offending node,
a sentence, and usually a hint:

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

### `schema` — the dialect, for tools

```bash
charton schema > charton.schema.json
```

## From a command line to a document

`plot --emit-spec` writes the chart you just drew as a spec:

```bash
charton plot sales.csv --mark bar --x month --y revenue --emit-spec chart.json -o a.svg
charton render chart.json -o b.svg
cmp a.svg b.svg        # identical
```

That equality is not a coincidence and not a promise: both commands take the
same build path, so the document *is* the chart. Edit `chart.json` to reach
everything the flags do not cover — aggregation, binning, layers, facets.

With stdin input the spec records `"url": "-"`, so it replays through a pipe:

```bash
cat sales.csv | charton plot - --x month --y revenue --emit-spec chart.json -o a.svg
cat sales.csv | charton render chart.json -o b.svg
```

## Input

| Source | How |
|---|---|
| CSV (`.csv`) | First row is the header. A column is numeric only when every cell parses as a number. |
| TSV (`.tsv`) | Same, tab-separated. |
| NDJSON (`.ndjson`, `.jsonl`) | One JSON object per line; scalars only. |
| stdin | Omit the file, or pass `-`. The format is sniffed: `{` means NDJSON. |

A spec's `data.url` is a **path, not a URL**, and it resolves against the spec
file's directory — so a document means the same thing wherever you run it.
Remote URLs are refused: download the data first.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | A bug in Charton (the message says so, and asks for a report) |
| 2 | The command line was wrong |
| 3 | The data could not be read or parsed |
| 4 | The spec is invalid, or disagrees with the data |
| 5 | Building, rendering or writing failed |

Diagnostics go to stderr. Nothing but the requested image, report or schema ever
lands on stdout, so `-o -` pipes cleanly.

## What it deliberately does not do

- No flag per core capability. Complex charts are documents; a hundred switches
  is how a command line turns into a second, worse dialect.
- No `profile`, `init` or `examples` subcommands yet. They wait for a real
  request.
- No expression evaluation, ever. A spec is data, not a program.
