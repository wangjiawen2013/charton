# Errors & Diagnostics

Charton fails where the caller can act on it. There are three scopes, and each
has one shape.

## The Rust API: `ChartonError`

Every library operation returns `Result<_, ChartonError>`. A chart that cannot be
built is an error — never a panic, never a half-drawn figure.

| Variant | What it means |
|---|---|
| `Data` | The rows are malformed: inconsistent lengths, an empty dataset, a column that is not the type the mark needs. |
| `Mark` | The mark cannot be configured that way. |
| `Encoding` | The channel and field combination is not valid. |
| `Scale` | The requested domain or scale type cannot be built from the data. |
| `Render` | Geometry or a backend failed while drawing. |
| `Font` | A font stack names no installed family. |
| `Io` | Reading or writing failed. |
| `Fmt` | A formatting operation failed. |
| `ExecutablePath` | The running executable's path could not be resolved. |
| `Unimplemented` | A recognised request this build does not implement. |
| `Internal` | A bug in Charton. |

There is no warning channel. A chart either builds or it returns the error that
explains why; a missing font is a `Font` error, not a silently smaller picture.

## The document layer: `SpecError`

A chart document is checked before it is lowered. One problem is one object:

```json
{
  "code": "E_UNKNOWN_MARK",
  "at": "/mark",
  "message": "Unknown mark `linee`",
  "hint": "Did you mean `line`?",
  "expected": ["point", "line", "bar", "area"]
}
```

- `code` is stable. `E_UNKNOWN_*` means the word is wrong, `E_UNSUPPORTED_*`
  means it is valid Vega-Lite that Charton does not implement, and `E_DATA_*`
  means the document is fine and the rows are not.
- `at` is a JSON Pointer into the original document, so an editor or an agent can
  jump to the node instead of searching for it.
- `hint` and `expected` say what to try next.

`charton validate --json` and the MCP server return one envelope for success and
failure (`ok`, `errors`), so a caller parses one shape. The full list of codes and
their guarantees is in [Writing a chart document](../spec/index.md#when-it-is-wrong).

Two invariants keep the layer honest:

1. **Validation runs before lowering.** A document that validates renders. If
   lowering still fails, the diagnostic is `E_INTERNAL` and the bug is in the
   validator, not in the document.
2. **The pointer is the original document's.** Diagnostics are attached while the
   document is read, not reconstructed after lowering.

## The command line: exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | A bug in Charton |
| 2 | The command line was wrong |
| 3 | The data could not be read or parsed |
| 4 | The document is invalid, or disagrees with the data |
| 5 | Building, rendering or writing failed |

Diagnostics go to stderr; only the requested artifact goes to stdout. That is
what makes `-o -` pipe cleanly and what keeps `validate --json` parseable.

## What keeps the boundaries honest

- The negative corpus (`tests/spec/reject/`) requires every diagnostic code to be
  reachable and every rejection to carry the expected pointer.
- `tests/spec_lower.rs` renders a document and the equivalent typed-API chart and
  requires identical bytes, so the two error/lowering paths cannot drift.
- The CLI and MCP integration tests drive the real binary: exit codes, stdout vs
  stderr, and the sandbox refusals.
