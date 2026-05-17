# The Atomic Unit: ColumnVector

At the heart of Charton's performance lies the `ColumnVector`. While most visualization libraries treat data as a collection of loose objects or rows, Charton adopts a Columnar Memory Layout. This architecture is inspired by Apache Arrow and Polars, ensuring that data is stored in contiguous memory blocks for CPU cache efficiency and potential SIMD acceleration.

## The Anatomy of a Column
A `ColumnVector` is a specialized enum that encapsulates data types relevant to data science and visualization. Every variant (except for those with intrinsic null representation) follows a dual-structure:

1. Data Buffer: A `Vec<T>` containing the raw physical values.
2. `Validity Bitmask: An `Option<Vec<u8>>` where each bit represents whether a row is "Valid" (1) or "Null" (0).

## The Categorical Advantage

One of the most important types for visualization is `Categorical`. Instead of storing repetitive strings (like "Group A", "Group A"...), it stores `u32` keys pointing to a unique dictionary of values. This is essential for rendering large datasets with repetitive labels while keeping memory usage flat.

## Manual Construction

Charton provides high-level constructors to turn various Rust string collections into memory-efficient categorical columns automatically.

### 1. From Raw Strings (No Nulls)

If your data is complete, you can pass collections of `String` or `&str` directly. Charton will handle the deduplication and dictionary encoding.

```rust
// Supporting Vec<&str> or Vec<String>
let cities = vec!["London", "Paris", "London", "Tokyo"];
let col = ColumnVector::from_str_as_cat(cities);
```

### 2. From Optional Strings (With Null Support)

For datasets with missing values, use `from_str_as_cat_opt`. This version automatically builds the internal Validity Bitmask.

```rust
// Supporting Vec<Option<&str>> or Vec<Option<String>>
let status = vec![Some("High"), None, Some("Low"), Some("High")];
let col = ColumnVector::from_str_as_cat_opt(status);
```

### 3. Why Use Categorical?
* Memory Efficiency: 1 million rows of "Male"/"Female" takes ~1MB as `Categorical`, compared to ~20MB+ as raw `String`.
* Encoding Ready: The underlying `u32` keys are used directly by Charton's color scales and legend generators.

## Why this Layout ?

* Polars-Friendly: The variants map 1:1 to Polars `DataTypes`, allowing for near zero-cost ingestion from Polars DataFrames via the `load_polars_df!` macro.
* Wasm-Ready: By preserving narrow types like `Int8`, Charton minimizes memory footprint in memory-constrained WebAssembly environments.
* Zero-Abstraction Temporal Data: Time data is stored as raw `i64` integers, allowing coordinate arithmetic without the cost of high-level object wrapping.

## Full Type Mapping Reference

| Charton Variant | Physical Storage | Polars Equivalent | Best Use Case |
| :--- | :--- | :--- | :--- |
| `Boolean` | `bool` | `Boolean` | Binary flags, True/False categories. |
| `Int8` / `Int16` | `i8` / `i16` | `Int8` / `Int16` | Memory-efficient small integers (e.g., months). |
| `Int32` / `Int64` | `i32` / `i64` | `Int32` / `Int64` | General purpose integers or primary IDs. |
| `UInt32` | `u32` | `UInt32` | Array indices or internal dictionary keys. |
| `UInt64` | `u64` | `UInt64` | Large hashes or 64-bit unique identifiers. |
| `Float32` | `f32` | `Float32` | Memory-efficient coordinates for high-density plots. |
| `Float64` | `f64` | `Float64` | The Standard for most coordinate and value axes. |
| `String` | `String` | `String` / `Utf8` | Unique labels or long descriptions. |
| `Categorical` | `u32` Keys + `String` Dict | `Categorical` / `Enum` | Recommended for Legends, Colors, and repeated labels. |
| `Date` | `i32` (days since epoch) | `Date` | Calendar-based timelines. |
| `Datetime` | `i64` + `TimeUnit` | `Datetime` | Time-series data with sub-second precision. |
| `Duration` | `i64` + `TimeUnit` | `Duration` | Time deltas or Gantt chart intervals. |
| `Time` | `i64` (nanos since midnight) | `Time` | Daily cycles and clock-time analysis. |