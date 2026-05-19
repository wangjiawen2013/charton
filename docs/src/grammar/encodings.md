# Encodings & Channels

In Charton, **Encoding** is the bridge between the "Data World" and the "Visual World." It defines how a specific column (Dimension) in your dataset is transformed into a visual property that a human can perceive.

## What are Visual Channels?

A visual channel is a physical property of a graphic that can carry information. Based on the implementation in `encode.rs`, Charton supports the following core channels:

* **Position Channels**: `X` and `Y`. These are the most powerful channels, used to represent numerical magnitude or categorical ordering.
* **Aesthetic Channels**:
    * `Color`: Used for distinguishing categories (Palettes) or representing gradients (Continuous maps).
    * `Shape`: Used exclusively for categorical data, using different geometric primitives (Circle, Square, Triangle).
    * `Size`: Typically mapped to point radii or line stroke widths to represent weight or importance.
* **Support Channels**: `Text` (for labels) and `Y2` (used as a baseline for area charts or interval bars).

## Mapping Data to Aesthetics

Encoding is a **Semantic Declaration**. When you write the following code:

```rust
chart.encode((
    x("timestamp"),
    y("temperature"),
    color("city")
))
```

You are instructing Charton's engine to:

1. Extract: Retrieve the "timestamp," "temperature," and "city" columns from the dataset.
2. Assign: Bind these columns to the horizontal position, vertical position, and color hue, respectively.
3. Infer: The system automatically detects the data types (e.g., Temporal for time, Linear for temperature, Discrete for city) to choose the correct mapping logic.

## The `Encoding` Architecture

Under the hood in `encode.rs`, all encoding requests are consolidated into a central `Encoding` container:

```rust
pub struct Encoding {
    pub(crate) x: Option<X>,
    pub(crate) y: Option<Y>,
    pub(crate) color: Option<Color>,
    pub(crate) shape: Option<Shape>,
    pub(crate) size: Option<Size>,
    // ... other channels
}
```

Each channel (e.g., `Color`) stores the Field name and a ResolvedScale. In the "Arbitration Phase" of the chart lifecycle, the engine injects a concrete mathematical scale into this placeholder based on the global data boundaries.

## Global Aesthetic Consistency

A key design goal of Charton is Global Aesthetics. As demonstrated in `aesthetics.rs`, if multiple layers use the same field for the same channel (e.g., both a Point layer and a Line layer use `color("species")`), the `GlobalAesthetics` orchestrator ensures:

1. Uniformity: The "Species A" category will have the exact same hex color in every layer.
2. Consolidation: Only one unified Legend is generated for that field, preventing visual clutter and conflicting guides.

## Tuple-Based Expressiveness

To streamline the developer experience, Charton utilizes Rust macros to implement `IntoEncoding` for tuples. This allows you to define multiple channels in a single, concise call, making the code highly readable and expressive.

## Key Takeaways

* Declarative Intent: You define "what" maps to "where," not "how" to calculate pixels.
* Channel Hierarchy: Position channels (X/Y) are prioritized for accuracy, while aesthetics (Color/Shape) are used for grouping.
* Lazy Resolution: Encodings store the "Intent"; the actual pixel values are calculated only during the realization phase.