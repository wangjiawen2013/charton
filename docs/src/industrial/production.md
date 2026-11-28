# CI & Deployment

Charton is headless by default: it writes an SVG and needs no display. The same
engine is behind the CLI, so a report job can be a shell step instead of a Rust
service.

## A report job

```bash
charton validate chart.json --data sales.csv --json    # exit 4 on problems
charton render chart.json -o report.svg
charton render chart.json -o report.pdf
```

The JSON report is the product: commit it, or fail the step on exit code 4.
Diagnostics go to stderr and only the artifact goes to stdout, so `-o -` pipes
into another process. `charton schema` writes the document format's JSON Schema for an
editor, a code generator or a contract test.

## What is deterministic

SVG output is text and byte-stable: the same document and rows produce the same
bytes. The spec corpus and the CLI tests depend on that, and the library's
`tests/spec_lower.rs` uses it to prove a document and the typed API are one path.

Raster text is **not** platform-independent. Glyph metrics come from the fonts the
host provides, so the CI matrix runs the raster tests on Linux, macOS and Windows
and the Chinese-text example on all three. Compare SVGs across machines; compare
PNGs only inside one pinned image.

## Keep the build small

The default feature set has no rasterizer, no PDF encoder and no GPU backend:

```toml
charton = { version = "0.9" }                              # SVG only
charton = { version = "0.9", features = ["png"] }          # + CPU raster
charton = { version = "0.9", features = ["pdf"] }          # + tagged PDF
```

Build the backend in the image that uses it. A validation-only job needs no
renderer at all; a report job needs `png`/`pdf`; a desktop or WASM build adds
`wgpu`. The full matrix is in [Installation](../getting_started/installation.md).

## The guards that make a change reviewable

The repository already runs the checks a downstream project would write:

| Guard | Where |
|---|---|
| Public API surface | `tests/api_baseline.rs` vs `docs/api-baseline.txt` |
| Dialect schema | `tests/spec_schema.rs` vs `schema/charton-v0.schema.json` |
| Accept / reject corpus | `tests/spec_corpus.rs` vs `tests/spec/{accept,reject}/` |
| Document == typed API bytes | `tests/spec_lower.rs` |
| CLI and MCP behaviour | `charton-cli/tests/{cli,mcp}.rs` |

Adopting the same idea in your own repository is enough: render a reference SVG
per report, compare bytes, and run `charton validate` on every document before it
is rendered.
