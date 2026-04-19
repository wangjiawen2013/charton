[English](./README.md) | [简体中文](./README_zh.md)

# Charton - A versatile plotting library for Rust
**Altair-style declarative plotting for Rust. High-performance, Polars-native, and Wasm-ready.**
> *"Really nice project. ... This works perfectly as an ecosystem."*
> — [**Ritchie Vink**](https://github.com/pola-rs/polars/issues/25941), Creator of Polars

[![Crates.io](https://img.shields.io/crates/v/charton.svg)](https://crates.io/crates/charton)
[![Documentation](https://docs.rs/charton/badge.svg)](https://docs.rs/charton)
[![Build Status](https://github.com/wangjiawen2013/charton/actions/workflows/ci.yml/badge.svg)](https://github.com/wangjiawen2013/charton/actions)
[![License](https://img.shields.io/badge/license-Apache2.0-blue.svg)](LICENSE)

**Charton** is a high-performance Rust plotting library featuring a declarative API inspired by [Altair](https://altair-viz.github.io/). It provides native [Polars](https://github.com/pola-rs/polars) support and bridges the gap to the Python visualization ecosystem (Altair/Matplotlib). Integrated with evcxr_jupyter, it enables seamless interactive data exploration in notebooks.

<table>
    <tr>
        <td><img src="docs/src/images/altair.svg" alt="Altair" /><p align="center">Altair</p></td>
        <td><img src="docs/src/images/matplotlib.png" alt="Matplotlib" /><p align="center">Matplotlib</p></td>
        <td><img src="docs/src/images/stacked_bar.svg" alt="Stacked Bar Chart" /><p align="center">Stacked Bar Chart</p></td>
        <td><img src="docs/src/images/grouped_bar_with_errorbar_2.svg" alt="Grouped Bar Chart with Errorbar" /><p align="center">Grouped Bar With Errorbar</p></td>
        <td><img src="docs/src/images/density.svg" alt="Density" /><p align="center">Density</p></td>
    </tr>
    <tr>
        <td><img src="docs/src/images/histogram.svg" alt="Histogram" /><p align="center">Histogram</p></td>
        <td><img src="docs/src/images/2d_density.svg" alt="2d Density" /><p align="center">2d Density Chart</p></td>
        <td><img src="docs/src/images/heatmap.svg" alt="Heatmap" /><p align="center">Heatmap</p></td>
        <td><img src="docs/src/images/grouped_boxplot.svg" alt="Grouped Boxplot" /><p align="center">Grouped Boxplot</p></td>
        <td><img src="docs/src/images/cumulative_frequency.svg" alt="Cumulative Frequency" /><p align="center">Cumulative Frequency</p></td>
    </tr>
    <tr>
        <td><img src="docs/src/images/distribution.svg" alt="Distribution" /><p align="center">Distribution</p></td>
        <td><img src="docs/src/images/pie.svg" alt="Pie" /><p align="center">Pie</p></td>
        <td><img src="docs/src/images/donut.svg" alt="Donut" /><p align="center">Donut</p></td>
        <td><img src="docs/src/images/rose.svg" alt="Rose" /><p align="center">Rose</p></td>
        <td><img src="docs/src/images/nightingale.svg" alt="Nightingale" /><p align="center">Nightingale</p></td>
    </tr>
    <tr>
        <td><img src="docs/src/images/simple_stacked_area.svg" alt="Simple Stacked Area" /><p align="center">Simple Stack Area</p></td>
        <td><img src="docs/src/images/normalized_stacked_area.svg" alt="Normalized Stacked Area" /><p align="center">Normalized Stacked Area</p></td>
        <td><img src="docs/src/images/steamgraph.svg" alt="Steamgraph" /><p align="center">Steamgraph</p></td>
        <td><img src="docs/src/images/rule.svg" alt="Rule" /><p align="center">Rule</p></td>
        <td><img src="docs/src/images/strip.svg" alt="Strip" /><p align="center">Strip</p></td>
    </tr>
</table>

## Installation
Add to `Cargo.toml`:

```toml
[dependencies]
charton = "0.5"                                         # Standard (Parallel enabled)
charton = { version = "0.5", default-features = false } # For WASM / Single-thread
charton = { version = "0.5", features = ["resvg"] }     # With PNG support
charton = { version = "0.5", features = ["bridge"] }    # With Altair/Matplotlib interop
```

## Quick Start
Charton provides a high-level, declarative API. Standard visualizations can be generated using a concise one-liner syntax:

```rust
use charton::prelude::*;

// Data: Physical measurements (Height vs. Weight)
let height = vec![160.0, 165.0, 170.0, 175.0, 180.0];
let weight = vec![55.0, 62.0, 68.0, 75.0, 82.0];

// One-liner plotting
chart!(height, weight)?.mark_point()?.encode((alt::x("height"), alt::y("weight")))?.save("out.svg")?;
```

## From Macros to Production API
While the `chart!` macro is a convenient syntactic sugar for rapid prototyping and simple scripts, the underlying `Chart::build` API is recommended for production environments where explicit data handling is required.

### 1. Professional Build API
For complex applications, use `Chart::build` to gain full control over the `Dataset` lifecycle.

```rust
let ds = Dataset::new()
    .with_column("height", height)?
    .with_column("weight", weight)?;

Chart::build(ds)? // Equivalent to chart!(ds)?
    .mark_point()?
    .encode((alt::x("height"), alt::y("weight")))?
    .save("out.svg")?;
```

> **Tip**: Use `add_column` instead if you need to add columns dynamically within loops or conditional logic.

### 2. Polars Integration
For Polars users, Charton provides the `load_polars_df!` macro to seamlessly convert a `DataFrame` into a Charton-ready `Dataset`.

```rust
use polars::prelude::*;

let df = df![
    "height" => vec![160.0, 165.0, 170.0, 175.0, 180.0],
    "weight" => vec![55.0, 62.0, 68.0, 75.0, 82.0]
]?;

// Efficiently convert Polars DataFrame to Charton Dataset
let ds = load_polars_df!(df)?;

Chart::build(ds)? // Equivalent to chart!(ds)?
    .mark_point()?
    .encode((alt::x("height"), alt::y("weight")))?
    .save("out.svg")?;
```

**Compatibility Note**: Charton uses versioned macros to handle Polars' rapid API evolutions. Versions below 0.42 are no longer supported.

|Polars Version       |Macro to Use              |Status              |
|:--------------------|:-------------------------|:-------------------|
|0.53+                |`load_polars_df!(df)?`    |Latest (Standard)   |
|0.42 - 0.52          |`load_polars_v42_52!(df)?`|Legacy Support      |
|< 0.42               |N/A                       |Unsupported         |

## Layered Grammar
Inspired by the Grammar of Graphics (as seen in `ggplot2` and `Altair`), Charton replaces rigid templates with a modular, layer-based system. Visualizations are constructed by stacking atomic marks, offering infinite flexibility beyond fixed chart types.

```rust
// Create individual layers
let line = chart!(height, weight)?
    .mark_line()?
    .encode((alt::x("height"), alt::y("weight")))?;

let point = chart!(height, weight)?
    .mark_point()?
    .encode((alt::x("height"), alt::y("weight")))?;

// Combine into a composite chart
line.and(point).save("layered.svg")?;
```

Charton can also leverages Rust’s functional paradigms, enabling infinite layer composition via fluent chaining or iterator folding. This allows for effortless, dynamic generation of complex multi-layered plots.

```rust
let layers: Vec<LayeredChart> = vec![line.into(), point.into(), bar.into() /* , ... etc */];

// Equivalent to line.and(point).and(bar)...
let lc = layers.into_iter()
    .reduce(|acc, layer| acc.and(layer))
    .expect("Failed to fold layers");
```

## Interactive Notebooks (Jupyter)
Charton integrates with evcxr_jupyter for interactive data exploration. Replacing .save() with .show() renders SVGs directly within notebook cells:

![evcxr jupyter](assets/evcxr_jupyter.png)

## WebAssembly and Frontend
Charton supports WebAssembly and modern web frontend; please refer to [Charton Docs](https://wangjiawen2013.github.io/charton) for details.

## Leveraging External Plotting Power
Charton bridges Rust with mature visualization ecosystems like **Altair** and **Matplotlib** via a high-speed IPC, enabling users to leverage diverse, professional-grade plotting tools within a unified workflow. please refer to [Charton Docs](https://wangjiawen2013.github.io/charton) for details.

## Industrial-Grade Visualization
Charton scales the Grammar of Graphics to heavy-duty production. By adopting the same proven philosophy as ggplot2, Altair, and the evolving ECharts, it validates its architecture as the industry standard, delivering strict type safety and zero-copy Polars integration for robust pipelines under extreme loads. This is powered by a rigorous Scale Arbitration engine that consolidates data domains into a "Single Source of Truth," ensuring absolute mathematical consistency and seamless cross-plot mapping while eliminating the fragile, hard-coded patches and silent overrides common in template-based tools.

![Comparison](assets/comparison.png)
*This figure demonstrates semantic synchronization in Charton. Heterogeneous samples (point layer) are anchored to a global background (bar layer). By enforcing a single mathematical truth across all layers, Charton maintains absolute color consistency, ensuring samples are accurately contextualized within the global background.*

## Publish Quality
Designed for precision, Charton provides pixel-perfect control over complex marks. Whether it is a multi-layered ErrorBar for biomedical research or a high-density scatter plot for finance, Charton delivers the aesthetic rigor required by top-tier journals.

<table>
    <tr>
        <td><img src="docs/src/images/weight_loss_curve_NEJM.png" alt="NEJM" /><p align="center">NEJM</p></td>
        <td><img src="docs/src/images/weight_loss_curve.svg" alt="Charton" /><p align="center">Charton</p></td>
    </tr>
</table>

*A reproduction of Figure 1A from the 2021 NEJM landmark study on once-weekly semaglutide for weight management, implemented using Charton.*

## Documentation
Please go to the [Charton Docs](https://wangjiawen2013.github.io/charton) for full documentation.

## License
Charton is licensed under the **Apache License 2.0**.
