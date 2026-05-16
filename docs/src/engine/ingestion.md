# Data Ingestion & Interop

Charton is positioned as the "Rendering Layer" in the Rust data science ecosystem. To provide the most efficient workflow, Charton offers deep integration with **Polars**, the de facto standard for high-performance DataFrames in Rust.

## Data Pipeline Overview

In a typical Charton application, data flows through the following pipeline:

1.  **External Input**: Load raw data from CSV, Parquet, JSON, or SQL databases.
2.  **Polars Processing**: Utilize Polars' Lazy engine for filtering, joins, aggregations, or feature engineering.
3.  **Charton Conversion**: Convert the processed `polars::DataFrame` into a `charton::Dataset`.
4.  **Visualization**: Pass the `Dataset` to Charton's rendering engine.

## Polars Integration

Charton uses the `load_polars_df!()` macro to convert a Polars `DataFrame` into a Charton `Dataset`.

```rust
use charton::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create DataFrame with diverse types, including native Polars temporal types
    let df = df!(
        "id" => &[1, 2, 3, 4, 5],
        "status" => &["High", "Low", "High", "Medium", "Low"],
        "value" => &[Some(1.2), None, Some(5.6), Some(7.8), None],
        "date" => Series::new("date".into(), &[19858i32, 19859, 19860, 19861, 19862]).cast(&DataType::Date)?, // ~2024-05-15
        "datetime" => Series::new("datetime".into(), &[1715760000000i64, 1715763600000, 1715767200000, 1715770800000, 1715774400000])
            .cast(&DataType::Datetime(TimeUnit::Milliseconds, None))?,
        "duration" => Series::new("duration".into(), &[3_600_000i64, 7_200_000, 1_800_000, 10_800_000, 5_400_000])
            .cast(&DataType::Duration(TimeUnit::Milliseconds))?,
    )?;

    // Conversion to Charton dataset
    let ds = load_polars_df!(df)?;
    println!("{:?}", ds);

    Ok(())
}
```

### Type Mapping & Metadata Preservation

Charton ensures strict metadata alignment during conversion. The following table illustrates how Polars logical types map to Charton physical storage:

| Polars Logical Type   | Charton Physical Type | Notes |
|-----------------------|-----------------------|-------|
| `Int8`, `Int16`, `Int32`, `Int64` | `i8`, `i16`, `i32`, `i64` | Direct physical mapping. |
| `UInt32`, `UInt64` | `u32`, `u64` | Direct physical mapping. |
| `Float32`, `Float64` | `f32`, `f64` | NaN values are treated as Nulls. |
| `Boolean` | `bool` | Mapped to nullable boolean vector. |
| `Utf8` / `String` | `String` | Stored as nullable string vectors. |
| `Categorical(_, _)`, `Enum(_, _)` | `Categorical` | Preserves dictionary encoding + validity. |
| `Date` | `Date` | Stored as i32 days since Unix epoch. |
| `Time` | `Time` | Stored as i64 nanoseconds since midnight. |
| `Datetime(unit, _)` | `Datetime` | Normalized to i64 nanoseconds since Unix epoch. |
| `Duration(unit)` | `Duration` | Normalized to i64 nanoseconds. |

*Note: Categorical does not appear to be a primitive type in rust Polars.*
