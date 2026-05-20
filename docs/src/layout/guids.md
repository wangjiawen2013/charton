# Guides: Axis & Legends

In a chart, the data marks (points, lines, bars) are the "what," but the Guides are the "how to read it." Guides act as the translation layer between abstract mathematical scales and human-readable visual cues. In Charton, these are primarily categorized into Axes (which interpret spatial mappings) and Legends (which interpret aesthetic mappings).

## The Guide Hierarchy

Following the Grammar of Graphics, Charton treats Axes and Legends as secondary structures that are semantically derived from the primary `Encoding` specification.

* Axes: Provide spatial context. They map the underlying continuous or discrete `Scale` domain to visual ticks and labels along the dimensions of the plotting area.
* Legends: Provide categorical or gradient context. They map aesthetic scales (Color, Shape, Size) back to labeled groups or color ramps.

## Axis Generation: From Math to Ticks

The creation of an axis is a multi-step process triggered during the rendering lifecycle:

1. Tick Calculation: The system queries the `Scale` domain to determine the ideal intervals. For linear data, it uses a "pretty number" algorithm to ensure labels land on clean integers or decimal fractions. For temporal data, it selects sensible units (e.g., hours, days, or months).
2. Physical Positioning: Using the `Coordinate System` (Cartesian or Polar), the axis generator calculates the pixel location for each tick. In Polar systems, this involves converting radial and angular distances into curved labels.
3. Rotation & Collision: If labels are long or dense, the system calculates their physical footprint in pixels. If a conflict occurs, the system dynamically rotates labels (e.g., to 45-degree angles) to avoid overlap, ensuring readability.

## Legend Consolidation: The Semantic Bridge

A major architectural feature in Charton is the Semantic Merging of legends. In many visualization libraries, color, shape, and size legends are generated separately, leading to cluttered interfaces. Charton optimizes this:

- Field Mapping: The system scans all aesthetic mappings (Color, Shape, Size) and groups them by their data `Field` name.
- Unified Guide Specs: If `Color` and `Shape` both map to the same field (e.g., "Category"), the system merges their definitions into a single `GuideSpec`.
- Rendering Strategy: The legend renderer then determines the `GuideKind` (Legend vs. ColorBar):
    - Discrete Legend: Used for categorical data. It builds a list of visual symbols (e.g., colored squares, specific shapes) that represent the group.
    - ColorBar: Used for continuous gradients. It renders a continuous strip that maps the data domain to a visual color ramp.

## Layout-Aware Rendering

Guides are not independent objects; they are aware of the chart's total physical constraints.

- Space Reservation: Before the plotting area is determined, the `Layout Engine` queries the Guides for their required dimensions. If the axis labels are long, the engine increases the reserved padding on that side of the plot to prevent clipping.
- Anchor Anchoring: Guides utilize a `PanelContext` to anchor themselves relative to the plot. For example, a legend placed on the `Right` position calculates its starting pixel coordinate based on the `Panel` width plus the theme-defined margin.

## Why Consolidation Matters

The automated generation of these guides provides three core advantages:

1. Mathematical Integrity: Because guides are generated directly from the resolved global scales, the labels are guaranteed to match the data precisely. You never have to manually update a label when the data changes.
2. Reduced Visual Noise: By merging multiple aesthetics into a single legend block, the chart keeps the viewer's focus on the data, not on redundant interface elements.
3. Automated Layout: Because the layout manager is aware of guide requirements, you don't need to manually configure margins. The chart automatically adjusts to accommodate the font sizes and number of categories present in your specific dataset.

## Key Takeaways

Axes map space; Legends map aesthetics.

- Guides are derived: They are not manually created, but are inferred directly from the Encoding and Scale specifications.
- Semantic Merging: Multiple aesthetics mapping to the same field are consolidated into a single guide to minimize visual clutter.
- Layout Awareness: Guides communicate their size requirements to the layout engine, ensuring the chart is always self-contained and perfectly padded.