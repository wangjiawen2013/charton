# Installation

Charton is one engine with opt-in backends. The default build draws SVG and
nothing else; every other feature adds a dependency only when you ask for it.

```toml
[dependencies]
charton = "0.9"
```

`use charton::prelude::*;` brings the `chart!` macro, `Chart::build`, the marks,
encodings and the export traits into scope.

## Cargo features

| Feature | Adds | Reach for it when |
|---|---|---|
| `spec` | The JSON chart document: types, validation, lowering, `schema()` (`serde`, `schemars`) | You read or write a `.json` chart; the CLI and MCP server are built on this. |
| `png` (alias `raster`) | The `tiny-skia` raster backend and `.save("out.png")` | You need a bitmap for a report, a dashboard or a terminal. |
| `pdf` | The `krilla` document backend, `.save("out.pdf")`, tagged PDF | The figure has to survive print or a journal's accessibility check. |
| `parallel` | Rayon-backed data processing | The table is large enough that serial geometry is the bottleneck. |
| `wgpu` | The GPU renderer (`WgpuRenderer`, `render_to_surface`) | You are integrating with winit, Bevy or egui, or animating a live chart. |
| `geo` | GeoJSON ingestion and the geographic coordinate system | You draw a map from a `.geojson` file. |
| `arrow` | Apache Arrow `RecordBatch` ingestion (`from_arrow`, `from_record_batches`) | Your data already arrives as Arrow (Parquet, a database, Arrow Flight). |

Features compose:

```toml
charton = { version = "0.9", features = ["png", "pdf"] }
```

The CLI, the MCP server and the Nushell plugin are separate entry points, not
features of the library. See [The command line](cli.md) and the
[Nushell plugin](../ecosystem/nushell.md).
