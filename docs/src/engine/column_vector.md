# The Atomic Unit: ColumnVector

At the heart of Charton's performance lies the `ColumnVector`. While most visualization libraries treat data as a collection of loose objects or rows, Charton adopts a Columnar Memory Layout. This architecture is inspired by Apache Arrow and Polars, ensuring that data is stored in contiguous memory blocks for CPU cache efficiency and potential SIMD acceleration.

## The Anatomy of a Column

A `ColumnVector` is a specialized enum that encapsulates data types relevant to data science and visualization. Every variant (except for those with intrinsic null representation) follows a dual-structure:

1. Data Buffer: A `Vec<T>` containing the raw physical values.
2. Validity Bitmask: An `Option<Vec<u8>>` where each bit represents whether a row is "Valid" (1) or "Null" (0).

By using an `Option`, Charton achieves zero overhead for datasets without missing values.

## Why this Layout?

* Polars-Friendly: The variants and naming (e.g., `Float64`, `Int32`, `Categorical`) map 1:1 to Polars `DataTypes`. This allows for near zero-cost ingestion from Polars DataFrames.
* Wasm-Ready: By preserving narrow types like `Int8` and `Int16`, Charton minimizes memory footprint in memory-constrained environments like WebAssembly.
* Zero-Abstraction Temporal Data: Time-related data (`Datetime`, `Duration`) is stored as raw `i64` integers. This allows the Charton scaling engine to perform coordinate arithmetic without the cost of high-level object wrapping.

## Key Data Variants
### Numerical Types

Charton supports a full range of signed and unsigned integers, along with single and double-precision floats.

```rust
// Example: A 64-bit float column with null tracking
ColumnVector::Float64 {
    data: vec![1.2, 0.0, 3.4], // 0.0 is a placeholder for Null
    validity: Some(vec![0b00000101]), // Binary: 101 (Row 1 is Null)
}
```

### Categorical Type

One of the most important types for visualization is `Categorical`. Instead of storing repetitive strings (like "Group A", "Group A"...), it stores `u32` keys pointing to a unique dictionary of values. This is essential for rendering large datasets with repetitive labels while keeping memory usage flat.

### High-Precision Temporal Types

Charton handles time with precision using `TimeUnit`. Whether the Polars data is in Milliseconds or Nanoseconds, Charton preserves that metadata, ensuring that axes and scales are rendered with scientific accuracy.

### Full Type Mapping Reference

To ensure seamless integration with Polars and high-performance rendering, Charton provides a comprehensive set of physical types. Below is the complete mapping of `ColumnVector` variants and their intended usage:

| Charton Variant | Physical Storage | Polars Equivalent | Best Use Case |
| :--- | :--- | :--- | :--- |
| `Boolean` | `bool` | `Boolean` | Binary flags, True/False categories. |
| `Int8` / `Int16` | `i8` / `i16` | `Int8 / Int16` | Memory-efficient small integers (e.g., months, ratings). |
| `Int32` / `Int64` | `i32` / `i64` | `Int32 / Int64` | General purpose integers, counts, or primary IDs. |
| `UInt32` | `u32` | `UInt32` | Array indices or internal dictionary keys. |
| `UInt64` | `u64` | `UInt64` | Large hashes or 64-bit unique identifiers. |
| `Float32` | `f32` | `Float32` | Memory-efficient coordinates for high-density plots. |
| `Float64` | `f64` | `Float64` | The Standard for most coordinate and value axes. |
| `String` | `String` | `String / Utf8` | Unique labels, tooltips, or long descriptions. |
| `Categorical` | `u32` Keys + `String` Dict | `Categorical / Enum` | Highly Recommended for Legends, Color encodings, and repeated axis labels. |
| `Date` | `i32` (days since epoch) | `Date` | Calendar-based timelines with Day-level precision. |
| `Datetime` | `i64` + `TimeUnit` | `Datetime` | Time-series data with sub-second precision. |
| `Duration` | `i64` + `TimeUnit` | `Duration` | Time deltas, Gantt chart intervals, or process durations. |
| `Time` | `i64` (nanos since midnight) | `Time` | Daily cycles and clock-time analysis. |

> Note on Performance: 
> While Charton stores data in these specific physical types to save memory (especially in Wasm), the internal scaling engine automatically performs Upcasting during computation. For instance, an `Int8` column is treated as `f64` when calculating axis positions, ensuring you get the memory savings of small types without sacrificing visual precision.

By understanding the `ColumnVector`, you understand how Charton bridges the gap between the heavy-duty processing of Polars and the high-speed requirements of modern rendering engines.