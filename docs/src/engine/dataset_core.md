# The Dataset: High-Performance Data Container

The `Dataset` is the primary unit of data movement in Charton. It is a column-oriented container designed for high-performance visualization, thread safety, and zero-copy data sharing.

## Internal Architecture

A `Dataset` manages a collection of `ColumnVector`s using a schema-based lookup. Its design focuses on three core principles:

1.  **Columnar Layout**: Data is stored as a `Vec<Arc<ColumnVector>>`. Using `Arc` allows multiple parts of a visualization (e.g., different chart layers) to share the same data without duplication.
2.  **Schema Integrity**: A `Dataset` ensures all columns have identical row counts (`row_count`), preventing out-of-bounds errors during rendering.
3.  **Fast Lookup**: An `AHashMap` maps column names to their physical index in the column vector for $O(1)$ access.

```rust
#[derive(Clone, Default)]
pub struct Dataset {
    /// Maps column names to their index in the `columns` vector.
    pub(crate) schema: AHashMap<String, usize>,
    /// Arc-wrapped columns for zero-copy sharing and threading safety.
    pub(crate) columns: Vec<Arc<ColumnVector>>,
    /// Total row count. Must be consistent across all columns.
    pub(crate) row_count: usize,
}
```

## Construction Methods

Charton provides multiple ways to ingest data, catering to different logic flows—from static configurations to dynamic processing.

### 1. Fluent / Builder Style

Best for static declarations or building datasets without `mut` variables. Each call to `with_column` consumes and returns the `Dataset`.

```rust
let ds = Dataset::new()
    .with_column("x", vec![10.0, 20.0, 30.0])?
    .with_column("y", vec![Some(100i64), None, Some(300i64)])?
    .with_column("category", vec!["A", "B", "C"])?;
```

### 2. Imperative Style

Ideal for dynamic logic or loops where you only have a mutable reference (`&mut self`) to the dataset.

```rust
let mut ds = Dataset::new();
ds.add_column("sepal_length", sepal_length)?;
ds.add_column("species", species)?;
```

### 3. Collection Conversion (`ToDataset` Trait)

The most idiomatic way to perform bulk ingestion from key-value pairs (vectors of tuples).

```rust
let raw_data = vec![
    ("mpg", vec![18, 15, 18].into_column()),
    ("car_name", vec!["chevrolet", "buick", "plymouth"].into_column()),
];

let ds = raw_data.to_dataset()?;
```

## Example

To ensure full compatibility with diverse workflows, `Dataset` can hold numerical, categorical, and temporal types simultaneously. Below is a 5-row example demonstrating every supported category using the `time` crate.

```rust
use charton::{Dataset, ColumnVector, TimeUnit, ToDataset, IntoColumn};
use time::macros::{datetime, date};

let complex_data = vec![
    // 1. Numerical & Boolean
    ("id", vec![1u64, 2, 3, 4, 5].into_column()),
    ("active", vec![true, true, false, true, false].into_column()),
    ("score", vec![Some(95.5), Some(88.0), None, Some(76.2), Some(91.0)].into_column()),
    
    // 2. Categorical (Dictionary Encoded)
    ("group", ColumnVector::from_values_as_categorical(
        vec!["High", "Low", "High", "Medium", "Low"]
    )),
    
    // 3. Raw Strings (Unique Labels)
    ("label", vec!["Alpha", "Beta", "Gamma", "Delta", "Epsilon"].into_column()),
    
    // 4. Temporal: Datetime & Date
    ("timestamp", vec![
        datetime!(2026-05-01 00:00 UTC),
        datetime!(2026-05-01 12:00 UTC),
        datetime!(2026-05-02 00:00 UTC),
        datetime!(2026-05-02 12:00 UTC),
        datetime!(2026-05-03 00:00 UTC),
    ].into_column()),
    
    ("date", vec![
        date!(2026-05-01), date!(2026-05-02), date!(2026-05-03), 
        date!(2026-05-04), date!(2026-05-05)
    ].into_column()),
    
    // 5. Duration (Time Deltas)
    ("lead_time", ColumnVector::Duration {
        data: vec![100, 250, 500, 750, 1000],
        unit: TimeUnit::Millisecond,
        validity: None,
    }),
];

let ds = complex_data.to_dataset()?;
```

## Core API Reference
### Inspection

* `height() -> usize`: Returns the number of rows.
* `width() -> usize`: Returns the number of columns.
* `get_column_names() -> Vec<String>`: Returns names in their insertion order.
* `is_null(name, row) -> bool`: Checks if a specific cell is null (handles both NaN and validity bitmasks).

### Data Access
* `column(name) -> Result<&ColumnVector>`: Access the column wrapper to inspect metadata (units, validity).
* `get_column<T>(name) -> Result<&[T]>`: High-performance access to the underlying physical slice.

    * Note: For temporal types, this returns the raw i64 slice.

### Slicing (Zero-Copy)
Charton uses "Eager Slicing." Because columns are wrapped in `Arc`, these operations are extremely lightweight and do not copy the underlying data buffers.

* `head(n)`: Returns a new `Dataset` containing the first `n` rows.
* `tail(n)`: Returns a new Dataset containing the last `n` rows.
* `slice(offset, len)`: Returns a new `Dataset` starting at `offset` with `len` rows.

## Debugging: The Tabular View

Printing the Dataset via {:?} renders a clean, aligned table with type markers.

```text
Dataset View: rows 0..5 (Total 5 rows)
id          | active      | score       | group       | lead_time   
(u64)       | (bool)      | (f64)       | (cat)       | (dur)       
--------------------------------------------------------------------
1           | true        | 95.5000     | High        | 100ms       
2           | true        | 88.0000     | Low         | 250ms       
3           | false       | null        | High        | 500ms       
4           | true        | 76.2000     | Medium      | 750ms       
5           | false       | 91.0000     | Low         | 1000ms
```