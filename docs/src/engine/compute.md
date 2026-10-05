# Compute & Transformation

[`ColumnVector`](column_vector.md) and [`Dataset`](dataset_core.md) are the
storage layer; this chapter is the **arithmetic layer** on top of it. These are
the primitives every statistic and transform is built from — aggregation,
quantiles, cardinality, null masks and row selection — kept in one place so the
transforms never reimplement them.

## 1. Reading values

A cell is read as an [`AnyValue`], a borrowed view that knows its physical type.
The `to_f64()` conversion is the universal bridge used by every numeric
algorithm (it maps temporal and integer types to `f64`, and returns `None` for a
null):

```rust
for i in 0..ds.height() {
    if let Some(v) = ds.column("score")?.get(i).to_f64() {
        // v is a valid f64; nulls and NaN are already filtered out
    }
}
```

`Dataset::get_column::<T>(name)` is the fast path when you already know the
physical type — it returns the contiguous `&[T]` slice with no per-cell dispatch.

## 2. Aggregation

[`AggregateOp`] reduces a group of rows to one number. A "group" is just a list
of row indices, so the same operator serves bar heights, box summaries and
colorbin aggregation:

| Operator | Result |
|---|---|
| `Sum` (default) | total of the valid values; `NaN` if all are null |
| `Mean` | arithmetic mean; `NaN` if all are null |
| `Median` | the 0.5 quantile (partial sort) |
| `Min` / `Max` | the extremes of the valid values |
| `Count` | the number of rows (nulls included) |

```rust
let indices = [0, 3, 4, 9];
let total = AggregateOp::Sum.aggregate_by_index(ds.column("revenue")?, &indices);
let avg   = AggregateOp::Mean.aggregate_by_index(ds.column("revenue")?, &indices);
```

`From<&str>` lets `y("revenue").with_aggregate("mean")` accept the usual names
(`sum`, `mean`/`avg`, `median`, `min`, `max`, `count`/`n`).

## 3. Quantiles

`get_quantile(sorted_data, q)` computes a quantile by **linear interpolation**
between the two nearest order statistics (the NumPy default). The input must
already be sorted; the callers that need it sort first with `total_cmp`, so the
ordering of equal/`NaN` values is stable across platforms.

It is the shared core of the median, the box plot (`q1`/`q2`/`q3`) and the
quantile box geometry — they differ only in `q`.

## 4. Range and cardinality

```rust
let (lo, hi) = ds.column("value")?.min_max();   // (NaN, NaN) when empty
let n_categories = ds.column("species")?.n_unique();
```

`n_unique` counts the distinct physical values and is used to choose bin counts
for histograms and heatmaps. `n_unique_serial` is the non-parallel fallback.

## 5. Categories, order and missing labels

The same column can be read as a *positional* list or a *non-positional* one, and
the difference is deliberate (see [Missing Values & Gaps](../concepts/missing_values.md)):

| Method | Returns | Null handling |
|---|---|---|
| `unique_values()` | the positional category list, in first-appearance order | nulls **excluded** |
| `labels_with_missing()` | the full level list, first-appearance order | the reserved `MISSING_CATEGORY` (`"NA"`) listed **last** |
| `label_with_missing(row)` | one row's label | null becomes `"NA"` |

Positional channels (`x`, `y`) use `unique_values`, so a null drops the row.
Non-positional channels (`color`, a transform's `group`) use
`labels_with_missing`, so a null is kept as a grey `NA` level.

## 6. Null masks

`Dataset::is_null(name, row)` tests a single cell (validity bit **or** a `NaN`).
For bulk work, `Dataset::get_combined_mask(&[names])` returns a **bit-packed**
mask (one bit per row) that is `1` only where every named column is valid. It
folds the stored validity bitmasks together and folds in `NaN`s for float
columns, so downstream code makes one cheap pass instead of re-checking each
column.

```rust
let keep = ds.get_combined_mask(&["x", "y", "value"])?; // Vec<u8>, bit-packed
```

## 7. Row selection and partitioning

- `take_rows(&[usize]) -> Dataset` materializes a subset. It is the primitive
  behind faceting: each panel is a set of row indices turned into a layer copy.
- `partition_by(&[fields]) -> FacetPartition` groups the row indices by the
  facet field combinations; `FacetPartition::row_indices(values)` looks a panel
  up. A missing facet field is an error, so a typo is reported instead of
  silently leaving every panel unfiltered.
- `row_indices(&[(name, value)]) -> Option<&[usize]>` is the direct filter used
  by per-panel re-execution.

Because columns are `Arc`-backed, `take_rows` and the facet snapshots share
buffers rather than copying them.

## 8. In-place updates

For live/animated charts, `Dataset::update_column_f64(name, &[f64])` overwrites a
column's buffer without rebuilding the dataset or reallocating when the length is
unchanged. This is what makes a zero-allocation render loop possible — see
[Lorenz Attractor](../case_studies/lorenz_attractor.md).

## 9. Parallelism

With the `parallel` feature, the hot paths go through the `Parallelizable` /
`IntoParallelizable` traits (Rayon under the hood): per-row geometry, binning and
aggregation fan out and collect back into a stable order. The results are
identical to the serial path — only the wall-clock time changes.

## 10. Where these are used

Every transform is a thin composition of the primitives above:

| Transform | Compute used |
|---|---|
| `transform_density` / `transform_density_2d` | `to_f64`, `min_max`, `get_quantile` (KDE) |
| `transform_contour` | `to_f64`, `n_unique`, grid recovery |
| `transform_quantile_box` | `AggregateOp`-style quantiles, `build_lane_layout` |
| `transform_window` | ordered row scan, `update_column_f64` |
| `mark_rect` / `mark_hist` | `n_unique`, `min_max`, `aggregate_by_index` |

## See also

- [The Atomic Unit: ColumnVector](column_vector.md) — the physical types.
- [The Dataset Struct](dataset_core.md) — construction, access, slicing.
- [Missing Values & Gaps](../concepts/missing_values.md) — the null policy.
- [Transforms & Columns](../grammar/transforms.md) — the column contract.
