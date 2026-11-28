# Performance & Scaling

Charton is built to keep the hot path cache-friendly: columnar storage, cheap
grouping, one binning implementation, and statistics that do not rescale with the
number of raw rows.

## Columnar storage

A `Dataset` is a set of named `ColumnVector`s. Each vector is a contiguous,
`Arc`-backed array of one physical type (`Float64`, `Int32`, `String`, …), so a
column can be shared by several charts or facet panels without copying.

There is no per-row object and no JSON round-trip between ingestion and drawing.
`take_rows` and facet snapshots share the underlying buffers.

## Grouping and binning

Categorical work — box plots, KDE groups, faceted panels, discrete scales — uses
`ahash` (`AHashMap` / `AHashSet`) instead of the standard hasher. Grouping is the
common cost in statistical marks, and a non-cryptographic hasher is the right
trade for a local plotting process.

Histogram edges have one implementation, `BinEdges`, shared by `hist`, `rect` and
the `bin` transform. A pre-binned chart and a mark that bins internally therefore
land on the same edges instead of drifting by half a bin.

## Statistics on a grid

Kernel density (1-D and 2-D) and contouring evaluate on a grid rather than on
every pair of points. That changes the cost from quadratic in the row count to
linear in the rows and the grid, which is what makes a violin of a large table
practical. The grid is generated from the data's own bounds.

## Facets

Faceting re-executes a layer on a row subset per panel, using the same pure
transform functions as the root. Panels share the underlying column buffers, so
adding a facet multiplies the drawn geometry but not the stored data.

## Parallelism

With the `parallel` feature, the row-heavy paths (geometry, binning, aggregation)
fan out over Rayon and collect back into a stable order. The output is identical
to the serial path; only wall-clock time changes.

```toml
charton = { version = "0.9", features = ["parallel"] }
```

## Animation

`Dataset::update_column_f64(name, &[f64])` overwrites a column in place when the
length is unchanged. A live chart can reuse its dataset across frames instead of
rebuilding it — the zero-allocation loop the GPU backends are designed around.

## Where to look when it is slow

1. Is the table being rebuilt every frame? Reuse the `Dataset` and update columns.
2. Is a per-row object or a JSON round-trip in the ingestion path? Feed a
   `Dataset`, Polars or Arrow directly.
3. Is the backend the cost? SVG writes every element; `png`/`wgpu` batch instead.
4. Is a statistical mark rescaling every panel? Facet panels re-run the statistic;
   `parallel` helps when the panels are independent.
