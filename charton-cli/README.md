# charton-cli

A table in, a chart out — no Rust toolchain, no Node, no Python.

`charton` is the command-line front end of the [charton](https://github.com/wangjiawen2013/charton)
engine. It reads CSV, TSV, NDJSON or stdin, and it renders the same JSON chart
document (a Vega-Lite v5 subset) that the MCP server and the playground speak.

## Install

```sh
cargo install --path charton-cli      # from a checkout
```

Prebuilt binaries are not published yet; building from a checkout is the
supported path for now.

## Sixty seconds

```sh
charton plot sales.csv --mark line --x month --y revenue -o revenue.svg
```

`charton plot --help` lists the flags. They cover the common cases on purpose:
when a chart needs more than a handful of flags, it should be a document, not a
longer command line.

## The four commands

```sh
# A table and a few flags.
charton plot sales.csv --mark bar --x region --y revenue --color segment -o bars.svg
cat sales.csv | charton plot - --mark point --x revenue --y margin -o - > dots.svg

# A chart document (Vega-Lite v5 subset). `charton schema` prints its JSON Schema.
charton render chart.json -o report.pdf

# Check before you draw. Diagnostics go to stderr; with --json the report is stdout.
charton validate chart.json --data sales.csv --json     # exit code 4 on problems

# The same dialect, for editors and agents.
charton schema
```

A spec's `data.url` is a path, not a URL, and it resolves against the spec
file's directory. `url: "-"` means stdin, so `--emit-spec` documents replay
through a pipe:

```sh
charton plot sales.csv --x month --y revenue --emit-spec chart.json -o a.svg
charton render chart.json -o b.svg
cmp a.svg b.svg        # identical: plot is sugar over render
```

## Agents

```sh
charton mcp --root ./data
```

JSON-RPC over stdio, one message per line. Tools: `get_schema`, `profile_data`,
`list_marks`, `list_themes`, `validate_spec`, `render_spec`, `export_spec`.
Every path is confined to `--root`; the server is read-only unless
`--allow-write` is passed; there is no network access and no expression
evaluation.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | A bug in Charton |
| 2 | The command line was wrong |
| 3 | The data could not be read or parsed |
| 4 | The spec is invalid, or disagrees with the data |
| 5 | Building, rendering or writing failed |

## Documentation

- [Command line guide](https://github.com/wangjiawen2013/charton/blob/main/docs/src/getting_started/cli.md)
- [Writing a chart document](https://github.com/wangjiawen2013/charton/blob/main/docs/src/spec/index.md)
- [Field reference (lowering map)](https://github.com/wangjiawen2013/charton/blob/main/docs/src/spec/lowering-map.md)
- [Agents (MCP)](https://github.com/wangjiawen2013/charton/blob/main/docs/src/getting_started/mcp.md)

## License

Apache-2.0
