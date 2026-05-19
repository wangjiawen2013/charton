# Scale Arbitration

In a single-layer chart, mapping data to scales is straightforward. However, Charton’s power lies in its ability to compose multiple independent layers into a unified visualization. This requires Scale Arbitration—a mechanism that synchronizes disparate data domains into a single, consistent visual language.

## The Problem: Domain Mismatch

Imagine you have two layers in one chart:
1. Layer A (Scatter): Data ranges from $10$ to $50$.
2. Layer B (Trendline): Data ranges from $5$ to $60$.

If each layer calculated its own scale, Layer A’s $50$ would be at the very top of the chart, while Layer B’s $50$ would be somewhere in the middle. Scale Arbitration prevents this visual "hallucination" by ensuring every layer agrees on the same mathematical boundaries.

## The Arbitration Lifecycle

As discussed in the Chart Lifecycle (Chapter 3), arbitration happens during the Training Phase. It follows a three-step process:

### Collection (Local Bounds)

The `LayeredChart` orchestrator calls `get_data_bounds()` on every individual layer. Each layer reports its "Local Domain"—the min/max for continuous data or a set of unique categories for discrete data.

### Merging (Global Union)

The system performs a Union of all local domains:

- Continuous Scales: It finds the "Global Min" and "Global Max" across all layers.
- Discrete Scales: It creates a deduplicated set of all categories (e.g., if Layer A has `["A", "B"]` and Layer B has `["B", "C"]`, the global domain becomes `["A", "B", "C"]`).

### Expansion & Padding

Once the global raw domain is found, Charton applies Expansion Rules. By default, it adds a $5\%$ padding to continuous scales to prevent data marks from clipping against the chart edges.

## Shared Aesthetics (Color, Shape, Size)

Arbitration isn't just for X and Y axes; it applies to all visual channels.

- Color Synchronization: If Layer A maps "Category" to color and Layer B maps "Category" to shape, the arbitrator ensures they both use the same categorical order.
- Legend Merging: When multiple layers use the same data field for different aesthetics (e.g., both Color and Size represent "Sales"), the arbitrator merges them into a single, coherent legend block.

## Conflict Resolution & Overrides
What happens if Layer A wants a `Linear` scale but Layer B wants a `Log` scale?

1. Type Priority: Charton follows a strict hierarchy. If any layer explicitly requests a specific scale type, that type is prioritized. If types are fundamentally incompatible, the system returns a `ChartonError`.
2. Manual Overrides: You can "break" the automatic arbitration by providing an explicit domain in the `LayeredChart` configuration.

```rust
chart.with_x_domain(0.0, 100.0); // This domain will be forced, regardless of layer data.
```

## Why it Matters: The "Single Source of Truth"

By centralizing scale logic in the `LayeredChart`, Charton ensures:

- Mathematical Integrity: $X=10$ always means the same physical pixel for every layer.
- Parallel Safety: Because the scales are resolved and "frozen" before rendering starts, multiple threads can safely read the mapping logic without worrying about state changes.

## Summary

Scale Arbitration is the "invisible hand" that negotiates between independent layers to produce a single, accurate coordinate system. It transforms a collection of data fragments into a unified visual story.