# Agents (MCP)

`charton mcp` serves a local [Model Context Protocol](https://modelcontextprotocol.io)
server over stdio. An agent can profile a table, write a chart document, check it, fix what
the check says, and render — without running generated code and without reaching
outside one directory.

The **complete reference** — every tool, its arguments and result, the raw
JSON-RPC exchange, and the limits — is the
[CLI README](https://github.com/wangjiawen2013/charton/blob/main/charton-cli/README.md#the-mcp-server).
This chapter is the design behind it.

```bash
charton mcp --data-dir ./data               # read-only
charton mcp --data-dir ./data --allow-write # export_spec may write inside it
```

Add it to a client the way any stdio server is added:

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

## The loop

```text
get_schema ──► profile_data ──► write a document ──► validate_spec ──► fix what it says
                                                      │
                                                      ▼
                                                 render_spec ──► an image
                                                      │
                                                      ▼
                                                 export_spec ──► a file
```

Every step returns something the next step can use. Validation never says merely
"invalid": each problem carries a stable code, a JSON Pointer into the document,
a sentence, and usually a hint.

## Why tools, not generated code

The agent never runs code. It sends a document, and the server validates and
builds it through the same path the CLI uses. That is what makes the loop
auditable:

- the document is data, inspectable before and after;
- the validator is the contract, not the model's self-assessment;
- the renderer is the library, so the same document cannot mean two things.

## The seven tools

| Tool | In one line |
|---|---|
| `get_schema` | The document format's JSON Schema. Read it before writing a document. |
| `profile_data` | Columns, types, missing counts, ranges or distinct values, a sample. |
| `list_marks` | Every mark, its namespace, support, and required/refused channels. |
| `list_themes` | The themes `theme` accepts. |
| `validate_spec` | Every diagnostic, with a code and a pointer. |
| `render_spec` | SVG as text, or PNG as an image; problems replace the chart. |
| `export_spec` | Write the validated document; needs `--allow-write`. |

`spec` is either the document itself or a path inside the data directory. In the
server, every path — including a spec's own `data.url` — is resolved against
`--data-dir`, not against the document file.

## Two error layers

- **Tool failures are in band.** The result has `isError: true` and a readable
  text block. A bad document, a missing file and a refused write are all this kind,
  so the model can read what happened and try again.
- **JSON-RPC errors are protocol mistakes.** An unknown method or tool, a
  malformed message, or arguments that do not deserialize return a JSON-RPC
  `error` (`-32601` / `-32602`). These are for the client, not the model.

## The sandbox

These are not configurable in spirit: they are the reason a server is safe to run
at all.

- **One data directory.** Every path is joined to `--data-dir` and canonicalised
  before the check; `../` and symlinks cannot climb out.
- **Read-only by default.** `export_spec` refuses unless the server was started
  with `--allow-write`, and never creates directories.
- **Limits.** 64 MiB per input file, 32 MiB per output.
- **No code, no network.** No expressions, no closures, no file access from the
  document, no sockets.

## Protocol

One JSON object per line, both directions. Stdout is the transport: nothing else
is ever written there, and logs go to stderr. The server supports revisions
`2024-11-05`, `2025-03-26` and `2025-06-18`, advertises only the `tools`
capability, and serves requests even before `notifications/initialized`.

It deliberately has no `resources`, no `prompts` and no sampling. The seven tools
are the whole API; the CLI or a direct library call is the escape hatch.
