# Spec → Core: the lowering map

This page is the contract between the JSON dialect and the engine. It lists, per
field, what the field turns into. This is not a wish list: every row here is code
plus a test.

Three states, and no fourth:

| State | Meaning |
|---|---|
| **Lossless** | The Core call has the same meaning as the Vega-Lite field. |
| **Lossy** | Accepted, but the result is not exactly what Vega-Lite would draw. The reason is stated in the row. |
| **Error** | Refused with a code and a pointer. Never silently ignored. |

Two invariants hold at all times, and both are tested:

1. **A document that validates is a document that renders.** Anything the
   validator accepts, lowering can build. If lowering fails anyway, the error is
   `E_INTERNAL` — a bug in the validator, not in your document.
2. **One meaning, one build path.** A spec and the equivalent Rust calls produce
   the same SVG, byte for byte
   (`tests/spec_lower.rs::a_document_and_the_typed_api_produce_the_same_output`).

## Top level

| Spec | Core | State |
|---|---|---|
| `$schema` | version gate only | Lossless |
| `data` | resolved by the entry point; rows reach the core as a `Dataset` | Lossless — see [`data`](#data) |
| `mark`, `charton.mark` | `mark_*` | Lossless — see [`mark`](#mark) |
| `encoding` | `alt::*` / `encode` | Lossless — see [`encoding`](#encoding) |
| `transform` | `transform_*` | Lossless — see [`transform`](#transform) |
| `layer` | each layer becomes a `Chart`, combined with `.and()` | Lossless for one level — see [`layer`](#layer) |
| `facet` | `FacetSpec` | Lossless — see [`facet`](#facet) |
| `title`, `description` | `LayeredChart::with_title` / `with_description` | Lossless |
| `theme` | `LayeredChart::with_theme` (`light`, `dark`) | Lossless |
| anything else | — | **Error** (`E_UNKNOWN_FIELD`) |

## `data`

| Spec | Core | State |
|---|---|---|
| `values` | `spec::inline_dataset` | Lossless, with limits: 10 000 rows, 200 000 cells. Column types are inferred per column: all numbers → continuous, all booleans → categorical, anything else → text. A single text cell makes the whole column text — the only choice that never drops a value. **Date strings are not sniffed**; they stay categorical. |
| `url` | resolved by the entry point | Lossless. The core never reads a file: the CLI and the MCP server apply their own sandbox. |
| a URL with a scheme (`https://…`) | — | **Error** (`E_DATA_SOURCE`). |
| both `url` and `values`, or neither | — | **Error** (`E_DATA_SOURCE`). |

## `mark`

| Spec | Core | State |
|---|---|---|
| `point`, `line`, `bar`, `area`, `rect`, `rule`, `text`, `tick` | `mark_point`, `mark_line`, … | Lossless |
| `boxplot`, `errorbar` | `mark_boxplot`, `mark_errorbar` | Lossless: both are composite marks on both sides, built from the same transforms. |
| `mark.color` | `configure_*(..).with_color` | Lossless |
| `mark.size`, `mark.opacity` | — | **Error** (`E_UNSUPPORTED_FIELD`): each mark has a different name for its size (`radius`, `stroke_width`, `bar_width`), so a single field would be a guess. |
| `charton.mark` | see below | |

### Charton-only marks

| `charton.mark` | Core | State |
|---|---|---|
| `violin`, `density`, `density_2d`, `contour`, `hist` | `mark_violin`, `mark_density`, `mark_density_2d`, `mark_contour(z)`, `mark_hist` | Lossless. Same statistics the Rust API uses. |
| `polygon`, `path` | `mark_polygon`, `mark_path` | Lossless. `pathGroup` selects which rows form one shape. |
| `beeswarm` | `mark_point` + `PointLayout::Beeswarm` | Lossless: a swarm is a placement strategy, not a mark. |
| `quantile_box` | — | **Error** (`E_UNSUPPORTED_FIELD`): the dialect knows the name, this build does not draw it. The list lives in `SUPPORTED_EXTENSION_MARKS` in `src/spec/lower.rs`, next to the dispatch, so the two cannot drift. |

`hist` writes its own count column to `y`, so `y` may be omitted. If it is
present it must name a real column, and the mark replaces it with the count.

`contour` needs a height for each grid point, and `x`/`y` already use both
channels, so it names one explicitly: `"charton": {"mark": "contour", "z": "value"}`.
`charton.z` is required for `contour` and refused everywhere else.

`mark` and `charton.mark` are mutually exclusive. Both, or neither, is
`E_MARK_SOURCE`.

### Required channels

Taken from the core's own checks, not from the mark's name: a rule the renderer
does not enforce would reject working charts, and a rule it enforces silently
would surface as a mid-render failure.

| Mark | Channels that must be present |
|---|---|
| `point` | `x` |
| `line`, `bar`, `area`, `tick`, `boxplot`, `errorbar`, `rule` | `x`, `y` |
| `text` | `x`, `y`, `text` |
| `rect` | `x`, `y`, `color` — a heatmap cell carries its value in colour |

`rule` needs both positions in Charton even though Vega-Lite accepts a
horizontal rule with only `y`. That is a real difference and it is reported as
`E_MISSING_FIELD`, not drawn differently.

## `encoding`

A field is only accepted on a channel that can use it. Anything else is
`E_UNSUPPORTED_FIELD` with a hint naming the channels that do support it — never
a silent drop.

| Channel | `aggregate` | `bin` | `stack` | `scale` | `title` |
|---|---|---|---|---|---|
| `x` | — | rect, hist | — | ✓ | ✓ |
| `y` | ✓ | rect | ✓ | ✓ | ✓ |
| `y2` | — | — | — | — | — |
| `color` | ✓ | — | — | ✓ | ✓ |
| `size` | — | — | — | ✓ | ✓ |
| `shape` | — | — | — | ✓ | ✓ |
| `text` | — | — | — | — | — |
| `pathGroup` | — | — | — | ✓ | — |

`pathGroup` is allowed on `line`, `path` and `polygon`: those marks need to know
which rows form one shape. Anywhere else it is `E_MARK_CHANNEL`.

`bin` is per *mark*, not per channel: only `rect` (x and y) and
`charton.mark: hist` (x) bin their input. `bin: false` and `bin: {step: …}` are
refused — the binned marks choose their own resolution, and pretending to honour
a request we ignore would be worse than refusing it.

### Per channel

| Spec | Core | State |
|---|---|---|
| `field` | the column name | Lossless |
| `type` | scale hint (`quantitative`→linear, `nominal`/`ordinal`→band, `temporal`→time), **and** checked against the data | Lossless |
| `aggregate` | `AggregateOp` | Lossless. `sum`/`mean`/`average`/`median`/`min`/`max`/`count`. Other Vega-Lite operators are `E_UNKNOWN_AGGREGATE`. |
| `bin` | `X::bins` / `Y::bins` | Lossless for `rect`/`hist`. `maxbins` is honoured; `step` is refused. |
| `stack` | `StackMode` | Lossless. `"zero"`→`Stacked`, `"normalize"`→`Normalize`, `"center"`→`Center`, `"none"`/`false`→`None`, `true`→`Stacked`. |
| `scale.type` | `Scale::{Linear, Log, Discrete, Temporal}` | Lossless for `linear`/`log`/`band`/`point`/`ordinal`/`time`/`utc`. Anything else is `E_UNKNOWN_SCALE`. |
| `scale.domain` | `ScaleDomain::Continuous` / `Discrete` | Lossless. `[min, max]` needs a continuous scale; a category list needs a categorical one; numeric domains on a time scale are refused. |
| `scale.zero`, `scale.reverse` | `X`/`Y` fields | Lossless |
| `title` | `label` | Lossless |

Two channel meanings are enforced against the data, because the renderer would
otherwise fail mid-build:

| Rule | Error |
|---|---|
| `size` needs a continuous field — a category has no area to map | `E_DATA_TYPE` at `/encoding/size/field` |
| `shape` needs a categorical field — a measure has no finite set of symbols | `E_DATA_TYPE` at `/encoding/shape/field` |
| A `linear`/`log` scale can only read a continuous field; `time`/`utc` only a temporal one | `E_DATA_TYPE` at `/encoding/<channel>/scale/type` |

### The one place the two grammars disagree

`bar` and `area` in Charton **always** aggregate: a `(x, colour)` cell becomes one
mark, using `y.aggregate` (sum by default). Vega-Lite draws one mark per row
unless `aggregate` is present, and stacks by colour.

Where the two agree, Charton stays silent: with a colour encoding and stacking
(the default), the summed stack has the same height and the same colour bands.

Where they would disagree, Charton refuses with `E_DATA_SHAPE` instead of drawing
a different picture: repeated x values under `bar`/`area` with no `aggregate`,
no colour, or `stack: "none"`. The fix is one field — add `aggregate`, or
aggregate the data before writing the spec.

## `transform`

An ordered array, applied before the mark's own statistics. Order is meaning:
filtering before aggregating is not the same chart as aggregating before
filtering.

| Spec | Core | State |
|---|---|---|
| `{"filter": {"field": "g", "equal": "a"}}` | `transform_filter(Predicate::Equal)` | Lossless |
| `{"filter": {"field": "v", "range": [0, 10]}}` | `transform_filter(Predicate::Range)` | Lossless. Inclusive; null cells never match. |
| `{"filter": {"field": "g", "oneOf": ["a", "b"]}}` | `transform_filter(Predicate::OneOf)` | Lossless |
| `{"filter": "datum.v > 1"}` | — | **Error** (`E_UNSUPPORTED_EXPRESSION`). A spec is data, not a program. |
| `{"calculate": ...}` | — | **Error** (`E_UNSUPPORTED_EXPRESSION`), permanently. |
| `{"fold": ["a", "b"], "as": ["key", "value"]}` | `Fold::apply` | Lossless. `as` is optional (defaults to `key`/`value`). The value column is numeric when every folded column is, text otherwise. |
| `{"aggregate": [{"op": "mean", "field": "sales", "as": "avg"}], "groupby": ["region"]}` | `Aggregate::apply` | Lossless. The result keeps only the group columns and the measures. `count` may omit `field`. |
| `{"window": [{"op": "cumulative_sum", "field": "amount", "as": "running"}], "groupby": ["region"]}` | `WindowTransform::apply` | Lossless for `rank`, `row_number`, `cume_dist`, `cumulative_sum`. |
| window `sort` / `frame` | — | **Error** (`E_UNSUPPORTED_FIELD`): the window functions choose their own order and frame. Ranking sorts by the encoded field. |
| window `dense_rank` / `percent_rank` / `first_value` / `last_value` | — | **Error** (`E_UNSUPPORTED_FIELD`): the core knows the names, not the implementations. |
| `{"bin": true, "field": "x", "as": ["low", "high"]}` | `Bin::apply` | Lossless. Writes the two edges, keeps every row and column. `as` is optional (defaults to `bin_<field>` / `bin_<field>_end`). `maxbins` is honoured; `step` is refused. |
| `{"joinaggregate"}` | — | **Error** (`E_UNSUPPORTED_FIELD`): recognised Vega-Lite, not implemented. |
| anything else | — | **Error** (`E_UNKNOWN_FIELD`) |

Transforms run inside validation as well as inside lowering — the same pure
functions, so the two cannot disagree about what a filter, fold, aggregate or
window does. That is also why the row-shape check for `bar` (see below) looks at
filtered rows: removing the repetition is a legitimate fix, and the validator
says so. The same mechanism tracks columns across steps: a column created by
`fold` is visible to the encoding, and a column dropped by `aggregate` is not.

Two window limitations worth stating: every implemented operation reads a field
(the window is ordered by it, even for `row_number`), and the output name may
not shadow an existing column (`E_NAME_CLASH`).

`fold` refuses to overwrite a column: its outputs must be free names in the
current table, otherwise `E_NAME_CLASH`.

`bin` uses the same edges as `hist` and `rect`, from the one implementation in
`BinEdges`. A pre-binned chart and a mark that bins internally therefore line up
instead of drifting by half a bin.

## `layer`

| Spec | Core | State |
|---|---|---|
| `layer: [{mark, encoding}, …]` | each layer becomes a `Chart`, collected with `.and()` | Lossless for one level. |
| root `encoding` | inherited channel by channel; a layer's own channel wins | Lossless |
| root `data`, `transform` | inherited; layers may not set their own | Lossless |
| nested `layer` inside a layer | — | **Error** (`E_UNSUPPORTED_FIELD`): flatten them. |
| layer `title` / `description` / `facet` / `transform` | — | **Error** (`E_UNSUPPORTED_FIELD`): these belong to the root. |
| both `mark` and `layer` | — | **Error** (`E_MARK_SOURCE`): a document is one or the other. |

## `facet`

| Spec | Core | State |
|---|---|---|
| `{"field": "g", "columns": 2}` | `FacetSpec::wrap(g).with_columns(2)` | Lossless |
| `{"row": "a", "column": "b"}` | `FacetSpec::grid(a, b)` | Lossless |
| `field` together with `row`/`column` | — | **Error** (`E_TYPE_MISMATCH`): wrap or grid. |
| `columns` on a grid | — | **Error** (`E_UNSUPPORTED_FIELD`): the grid's shape comes from its two fields. |

Facets change what "repeated" means: the `bar`/`area` duplicate-x check runs
inside each panel, so the same month in two regions is not a repeated x.

## Not in this version

Conditional encodings are not a key of the dialect yet, so writing them is
`E_UNKNOWN_FIELD` rather than a silent no-op. Expression strings (`filter: "datum.x > 1"`, `calculate`) are permanently
out of bounds: a spec is data, not a program.
