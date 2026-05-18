# Quick Start

Charton’s API design mirrors the declarative philosophy of the Grammar of Graphics. To balance rapid prototyping flexibility with production-grade engineering rigor, Charton offers a dual-API paradigm: a fluid, concise `chart!` macro syntax and a deterministic, explicitly managed `Chart::build` Builder API.

## Swift Prototyping with Macros

For data exploration, standalone scripts, or interactive notebook environments, the `chart!` macro offers an elegant, one-liner fluid interface to bind and map raw vectors instantaneously.

```rust
use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Prepare raw observation vectors (Physical measurements: Height vs. Weight)
    let height = vec![160.0, 165.0, 170.0, 175.0, 180.0];
    let weight = vec![55.0, 62.0, 68.0, 75.0, 82.0];

    // 2. Linear declarative pipeline: bind -> instantiate mark -> map encoding -> save
    chart!(height, weight)?
        .mark_point()?
        .encode((alt::x("height"), alt::y("weight")))?
        .save("out.svg")?;

    Ok(())
}
```

## Production-Grade Builder API

While the macro interface is exceptional for quick iterations, enterprise applications demand explicit control over data structures and memory boundaries. The `Chart::build` API decouples data layout from visual marks, ensuring absolute type safety and allowing for dynamic dataset mutation.

```rust
use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let height = vec![160.0, 165.0, 170.0, 175.0, 180.0];
    let weight = vec![55.0, 62.0, 68.0, 75.0, 82.0];

    // 1. Explicitly manage the lifecycle of your Dataset
    let mut ds = Dataset::new()
        .with_column("height", height)?
        .with_column("weight", weight)?;

    // Note: If you need to append data dynamically within conditional branches or loops,
    // utilize the `add_column` method instead:
    // ds.add_column("age", vec![20, 22, 25, 30, 35])?;

    // 2. Build the chart deterministically via a strongly-typed constructor pipeline
    Chart::build(ds)?
        .mark_point()?
        .encode((alt::x("height"), alt::y("weight")))?
        .save("production_out.svg")?;

    Ok(())
}
```

## High-Performance Polars Integration

Charton provides native, high-efficiency ingestion interfaces for Polars DataFrames. To shield your codebase from Polars’ rapid API evolution, Charton ships with versioned compilation macros to maintain bulletproof backwards compatibility.

```rust
use polars::prelude::*;
use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Instantiate a standard Polars DataFrame
    let df = df![
        "height" => vec![160.0, 165.0, 170.0, 175.0, 180.0],
        "weight" => vec![55.0, 62.0, 68.0, 75.0, 82.0]
    ]?;

    // 2. Perform zero-copy / highly efficient conversion into a Charton Dataset 
    // using the optimized version-specific macro
    let ds = load_polars_df!(df)?;

    // 3. Bind to the production Builder API
    Chart::build(ds)?
        .mark_point()?
        .encode((alt::x("height"), alt::y("weight")))?
        .save("polars_chart.svg")?;

    Ok(())
}
```

⚠️ Polars Version Compatibility Reference:
- Polars 0.53+: Use the modern standard macro `load_polars_df!(df)?`.
- Polars 0.44 - 0.52: Use the legacy support macro `load_polars_v44_52!(df)?`.
- Polars < 0.44: Unsupported. Upgrading your upstream Polars dependency is highly recommended.