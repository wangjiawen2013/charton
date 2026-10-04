# Validation & Integrity

Charton validates **eagerly** and fails with a typed error rather than drawing a
silently wrong chart. This chapter is the map of those checks: the invariants the
engine maintains, the errors it raises, and the pipeline that runs when a layer
is built.

## 1. The schema invariant

A [`Dataset`](dataset_core.md) enforces one rule above all: **every column has
the same number of rows**. `add_column` calls `validate_len` before inserting, so
a length mismatch is rejected at the point of error rather than surfacing later
as an out-of-bounds read during rendering.

The `schema` (`AHashMap<String, usize>`) is the name → index map, and **column
names are the contract** between transforms and encodings: a transform writes
named columns, and `encode` reads them by name. Adding a column whose name
already exists *overwrites* it (a deliberate convenience), which is why
transforms that emit several columns guard against accidental self-collisions
(see §6).

## 2. The error registry

All failures are one enum, [`ChartonError`], so `?` composes cleanly:

| Variant | Raised when |
|---|---|
| `Data(String)` | inconsistent lengths, an empty/invalid dataset, a grid too small to contour |
| `Mark(String)` | a mark is missing or unknown, or a mark's configuration is absent |
| `Encoding(String)` | a required channel is missing, or a field cannot use the requested scale |
| `Scale(String)` | two layers disagree on a channel's scale type |
| `Render(String)` | a backend cannot draw a primitive |
| `Io(std::io::Error)` | file I/O when saving |
| `ExecutablePath(String)` | a bundled executable/binary path is invalid |
| `Unimplemented(String)` | a documented-but-not-yet-built feature |
| `Internal(String)` | an invariant the engine expected to hold was violated |
| `Fmt(std::fmt::Error)` | a formatting failure |

## 3. Semantic types

Every column has a [`SemanticType`], inferred from its physical type:

| Semantic type | Physical columns | Default scale |
|---|---|---|
| `Continuous` | `Float*`, `Int*`, `UInt*`, `Boolean` | `Linear` / `Log` / `Sqrt` |
| `Discrete` | `String`, `Categorical` | `Discrete` |
| `Temporal` | `Date`, `Datetime`, `Duration`, `Time` | `Temporal` |

The type drives everything downstream — binning, colour, the axis. A user can
override the scale (`alt::x("year").with_scale(Scale::Discrete)`), but only
combinations that make sense are accepted:

- **legal**: numbers treated as categories (`2024` → `"2024"`), temporal data on
  a linear scale (raw timestamps);
- **illegal**: `String`/`Categorical` data on a `Linear`, `Log` or `Temporal`
  scale — this returns an `Encoding` error instead of placing categories at
  nonsensical coordinates.

## 4. The validation pipeline

Building a layer runs `validate_and_transform`, in order:

| Step | Check |
|---|---|
| 1. Identification | the mark exists and reports a `mark_type` |
| 2. Mandatory encodings | required channels are present (e.g. a bar needs `x` and `y`; a density needs `x`) |
| 3. First semantic resolution | infer/validate each channel's scale from the source columns |
| 4. Scale-to-mark validation | the mark can work with the resolved scale (see §5) |
| 5. Statistics | `resolve_pre_transform_encodings` (automatic bin counts) then the mark's transform |
| 6. Second semantic resolution | resolve the **generated** columns (density, count, `path_group`, …) |
| 7. Visual defaults | zero baselines and axis padding for marks that need them |

A failed check stops the build with the matching error variant. Steps 3 and 6
run twice on purpose: step 3 validates what the user wrote, step 6 validates what
the statistic produced.

## 5. Scale compatibility

Each mark declares which scales it accepts per channel; the engine checks the
resolved scale against them. Examples: a bar or box plot expects a **discrete
x** and a linear y; a histogram expects a **continuous x** (it bins); a contour
expects continuous x and y; a density expects a continuous value axis.

When several layers share a channel they must agree on the scale type, or
`resolve_scale_spec_from` raises a `Scale` error naming the conflicting layers —
a scatter and a bar cannot share an axis that is both continuous and discrete.

## 6. Column-name integrity

Because names are the contract, a transform that writes the same output name
twice (or clashes with a grouping column it copies back) would silently drop a
column. `ensure_distinct_columns` turns that into a clear `Data` error:

> `transform writes the column 'x' more than once; rename one of its outputs with
> with_as(...), or choose a different input column`

Every composable transform (`transform_density`, `transform_density_2d`,
`transform_contour`, `transform_band`, `transform_quantile_box`) runs this check
before touching the data.

## 7. Facet integrity

`facet_partition` partitions the rows taken **before** the statistic ran, so a
panel sees the original columns even when the statistic drops them. A facet field
that is missing from a layer's dataset raises a `Data` error — a mistyped facet
field fails fast instead of silently leaving every panel unfiltered.

## 8. Missing-value integrity

The null policy is uniform and deliberate: a null in a **positional** channel
drops the row; a null in a **non-positional** channel is kept as the reserved
`MISSING_CATEGORY` (`"NA"`). The rules and the helpers behind them are in
[Missing Values & Gaps](../concepts/missing_values.md), and the bit-packed
machinery is in [Compute & Transformation](compute.md#6-null-masks).

## 9. Ingestion integrity

When data comes from loose Rust collections, the ingestion layer promotes a
column to a single physical type: an all-integer column stays integer, a column
that starts with integers but contains a decimal later is promoted to float, and
a column containing nulls stays validity-aware. See
[Data Ingestion & Polars](ingestion.md) for the promotion table.

## See also

- [Compute & Transformation](compute.md) — the arithmetic primitives.
- [The Dataset Struct](dataset_core.md) — the schema and row-count invariant.
- [Design Rules](../concepts/design_rules.md) — the statistic contract.
- [Safety & Error Registry](../industrial/safety.md) — handling errors in an application.
