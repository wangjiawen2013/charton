# Agents (MCP)

`charton mcp` serves a local [Model Context Protocol](https://modelcontextprotocol.io)
server over stdio. An agent can profile a table, write a spec, check it, fix what
the check says, and render — without running generated code and without reaching
outside one directory.

```bash
charton mcp --root ./data
```

Add it to a client the way any stdio server is added:

```json
{
  "mcpServers": {
    "charton": {
      "command": "charton",
      "args": ["mcp", "--root", "/absolute/path/to/data"]
    }
  }
}
```

## The loop

```text
profile_data ──► write a spec ──► validate_spec ──► fix what it says
                                        │
                                        ▼
                                   render_spec ──► an image
```

Every step returns something the next step can use. Validation never says merely
"invalid": each problem carries a stable code, a JSON Pointer into the document,
a sentence, and usually a hint.

## The tools

| Tool | Arguments | Returns |
|---|---|---|
| `get_schema` | — | The JSON Schema of the dialect. Read this before writing a spec. |
| `profile_data` | `path`, `sample_rows?` | Rows, columns, types, missing counts, ranges or distinct values, a sample. |
| `list_marks` | — | Every mark, its namespace, whether this build can draw it, and the channels it requires and refuses. |
| `list_themes` | — | The themes `theme` accepts. |
| `validate_spec` | `spec`, `data?` | Every diagnostic, with `ok: false`. With `data`, field names and types are checked too. |
| `render_spec` | `spec`, `format?` | SVG as text, or PNG as an image block. Problems replace the chart. |
| `export_spec` | `spec`, `path`, `overwrite?` | Writes the canonical spec. Off unless `--allow-write`. |

`spec` is either the document itself or a path inside the root. The dialect is
described in [Writing a chart document](../spec/index.md).

Tool failures come back **in band** (`isError: true` plus a readable message), so
the model can read what went wrong and try again. JSON-RPC errors are reserved
for protocol mistakes — an unknown tool, a malformed message.

## Safety

These are not configurable in spirit: they are the reason a server is safe to
run at all.

- **One root.** Every path — tool arguments and a spec's own `data.url` — is
  canonicalised and must stay inside `--root`. `../` and symlinks cannot climb
  out.
- **Read-only by default.** `export_spec` refuses unless the server was started
  with `--allow-write`, and even then it will not create directories and will not
  overwrite without `overwrite: true`.
- **Limits.** 64 MiB per input file, 32 MiB per output.
- **No code.** A spec is data. There is no expression evaluation, no `calculate`,
  no closure, no file access from a document — the same red line the rest of the
  dialect holds.
- **No network.** Nothing here opens a socket.

## Protocol notes

One JSON object per line, both directions, as MCP specifies for stdio. Stdout is
the transport: nothing else is ever written there. Logs go to stderr, so
`charton mcp 2>server.log` is a safe way to see what happened.

The server answers `initialize` with the revision it prefers (echoing the
client's when it is one it supports), advertises only the `tools` capability, and
serves requests even before `notifications/initialized` — being liberal there
turns a client's protocol slip into an answered call instead of a mystery.
