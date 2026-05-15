# Data Ingestion & Interop

Charton is positioned as the "Rendering Layer" in the Rust data science ecosystem. To provide the most efficient workflow, Charton offers deep integration with **Polars**, the de facto standard for high-performance DataFrames in Rust.

## Data Pipeline Overview

In a typical Charton application, data flows through the following pipeline:

1.  **External Input**: Load raw data from CSV, Parquet, JSON, or SQL databases.
2.  **Polars Processing**: Utilize Polars' Lazy engine for filtering, joins, aggregations, or feature engineering.
3.  **Charton Conversion**: Convert the processed `polars::DataFrame` into a `charton::Dataset`.
4.  **Visualization**: Pass the `Dataset` to Charton's rendering engine.

## Polars Integration

By enabling the `charton-polars` feature, you can leverage the `TryFrom` trait to convert data seamlessly.

### Basic Conversion Example

```rust
use polars::prelude::*;
use charton::core::Dataset;
use std::convert::TryFrom;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load data using Polars
    let df = CsvReader::from_path("sensor_metrics.csv")?
        .infer_schema(None)
        .has_header(true)
        .finish()?;

    // 2. Convert to Charton Dataset
    // This preserves all physical types and validity masks (Nulls)
    let ds = Dataset::try_from(df)?;

    println!("Ingested: {} rows x {} columns", ds.height(), ds.width());
    Ok(())
}
```

### Type Mapping & Metadata Preservation

Charton ensures strict metadata alignment during conversion. The following table illustrates how Polars logical types map to Charton physical storage:

| Polars Logical Type   | Charton Physical Type | Notes |
|-----------------------|-----------------------|-------|
| `Int32`, `Int64`      | `i32`, `i64`          | Direct physical mapping. |
| `Float32`, `Float64`  | `f32`, `f64`          | NaN values are treated as Nulls. |
| `Boolean`             | `bool`                | Mapped to bitmask-backed boolean vectors. |
| `Utf8 / String`       | `String`              | Stored as raw string vectors. |
| `Categorical`         | `Categorical`         | Preserves dictionary encoding; ideal for legends. |
| `Datetime`            | `Datetime`            | Mapped to `i64` preserving `TimeUnit` (ns, ms, s). |
| `Duration`            | `Duration`            | Mapped to `i64` preserving `TimeUnit`. |

### Advanced: Heterogeneous "Full-Stack" Ingestion

This example demonstrates a complex scenario where Polars handles numerical, categorical, and multiple temporal types simultaneously before converting to Charton.

```rust
use polars::prelude::*;
use charton::{Dataset, TimeUnit};
use std::convert::TryInto;

fn ingest_complex_dataframe() -> Result<Dataset, Box<dyn std::error::Error>> {
    // Create a DataFrame with diverse types
    let df = df!(
        "id" => &[1, 2, 3, 4, 5],
        "status" => &["High", "Low", "High", "Medium", "Low"],
        "val" => &[Some(1.2), None, Some(5.6), Some(7.8), None],
        "ts" => &[1715760000000i64, 1715763600000, 1715767200000, 1715770800000, 1715774400000],
        "lead_time" => &[100i64, 250, 500, 750, 1000],
    )?;

    // Cast types in Polars for optimal Charton ingestion
    let processed_df = df.lazy()
        .with_column(col("status").cast(DataType::Categorical(None, Default::default())))
        .with_column(col("ts").cast(DataType::Datetime(TimeUnit::Milliseconds, None)))
        .with_column(col("lead_time").cast(DataType::Duration(TimeUnit::Milliseconds)))
        .collect()?;

    // Final conversion to Charton
    let ds: Dataset = processed_df.try_into()?;
    
    Ok(ds)
}
```

### Performance & Memory Best Practices

#### 1. Shallow Copying

While Polars utilizes the Arrow memory layout and Charton uses its own `Arc<ColumnVector>` abstraction, Charton aims to minimize overhead. For large primitive columns (f64/i64), the conversion involves very little overhead as it primarily re-wraps existing buffers into `Arc` containers.

#### 2. Pre-aggregation (The "Gold" Rule)

Do not send millions of raw rows directly to Charton. While the `Dataset` can hold them, the browser's rendering engine will struggle with the sheer number of elements.

* Best Practice: Perform heavy lifting (GroupBy, Rolling windows, Aggregations) in Polars' lazy mode first. Only convert the "reduced" result (e.g., a few thousand points) to a Charton `Dataset`.

#### 3. Automatic Unit Recognition

Charton's `Debug` implementation automatically recognizes units inherited from Polars. If a Polars column is in `Microseconds`, Charton will display `500us`; if `Milliseconds`, it shows `500ms`. This ensures clarity when debugging visualization logic.

```text
Dataset Debug Output (converted from Polars):
---------------------------------------------
lead_time (dur) | ts (dt)
---------------------------------------------
100ms           | 2026-05-15T00:00:00Z
250ms           | 2026-05-15T00:00:01Z
```