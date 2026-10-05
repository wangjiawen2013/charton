# The Charton Mental Model

This chapter introduces the design philosophy of Charton. Understanding the "Mental Model" is more important than memorizing APIs, as it governs how you structure data and compose complex, production-grade visualizations in Rust.

## The Declarative Paradigm: "What, Not How"
Charton is built on the Grammar of Graphics. In traditional imperative plotting libraries, you manually calculate coordinates and "draw" shapes on a canvas. In Charton, you provide a Specification that describes the relationship between data and visual properties.

```rust
Chart::build(&df)?
    .mark_point()           // Geometric Mark: Point
    .encode((
        x("weight"),        // Position Encoding: Map "weight" to X-axis
        y("horsepower"),    // Position Encoding: Map "horsepower" to Y-axis
        color("origin"),    // Visual Encoding: Map "origin" to Color
    ))?
```

## The Layer Pipeline

Every layer answers four questions, in order:

1. **stat** — what do the data mean? (density, binning, quartiles)
2. **position** — where does each mark go? (identity, side by side)
3. **geometry** — what shape is drawn? (point, bar, polygon)
4. **scale** + **coordinate** — how does a value become a pixel?

Keeping the four apart is what lets a handful of basic parts describe almost any
chart. A violin plot, for example, is a density statistic, a position and a
polygon; the convenience mark `mark_violin` is just a name for that composition,
not a separate implementation. See
[The Layer Pipeline](grammar_pipeline.md) for the full model.

## The Orchestrator: LayeredChart

The `LayeredChart` is the central orchestrator of the system. It is not just a container; it is a State Machine that manages the visualization lifecycle through three key roles:

1. Structural Intent: It holds multiple `Layer` objects. You can overlay a scatter plot, a regression line, and annotation text, ensuring they all exist within a shared logical space.
2. Stateful Overrides: It stores user-defined overrides for domains, ranges, and layouts. These explicit instructions take precedence over the default `Theme`.
3. Physical Awareness: It bridges the gap between abstract math and pixels by managing canvas dimensions and coordinate transformations.

## Scale Arbitration & Global Aesthetics

A core challenge in multi-layer charts is visual consistency. Charton solves this through Scale Arbitration:
* Unified Domains: Charton scans every layer to calculate a shared `ScaleDomain` (the default, *fixed* behaviour). A faceted chart can instead train a scale inside each panel with `with_strategy("free")`. Statistics are computed per panel, so a cumulative summary never mixes data across panels.
* Aesthetic Consistency: The system maintains a unified visual language. If multiple layers map the "origin" column to `Color`, Charton ensures they share the exact same palette and legend, preventing conflicting visual cues.

## Space and Layout: From Logic to Physical

Charton treats space as a first-class citizen, separating the "logic" of a chart from its "physical" appearance:
* Coordinate Systems: Charton supports `Cartesian` and `Polar` systems. The underlying mark logic remains identical; the coordinate system simply handles the transformation of normalized $[0, 1]$ values into canvas positions.
* The Layout Engine: Charton uses a greedy stacking algorithm (similar to Flexbox). It automatically calculates the space required for titles, axis labels, and legends, ensuring the "Plot Panel" is perfectly centered and legible.

## Performance & Rust's Ownership Model

Charton is engineered for the Rust ecosystem, leveraging its unique strengths:

* Zero-Copy with Polars: By utilizing the Apache Arrow format, Charton processes massive DataFrames with minimal memory overhead.
* Thread-Safe Resolution: Resolved scales are stored in `Arc<RwLock<...>>`. This allows the rendering backend to safely access scale metadata across multiple threads, enabling high-performance parallel rendering.
* The Version Bridge: To solve "dependency hell," the `bridge` module allows passing data as Parquet-serialized bytes. This ensures Charton works seamlessly even if your project uses a version of Polars different from the one Charton was compiled with.

## Summary: The Charton Workflow

1. Data: Prepare your Polars DataFrame.
2. Specification: Compose `Layers` using `Marks` and `Encodings`.
3. Arbitration: `LayeredChart` resolves global `Scales`, `Guides`, and `Layouts`.
4. Rendering: The `RenderBackend` (SVG/PNG/PDF) translates geometry into the final output.