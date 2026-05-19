# From Data to Pixels (The Chart Life Cycle)

A chart in Charton is a dynamic sequence of transformations. This chapter traces the "biography" of a data point—from its raw state in a Polars DataFrame to its final geometric representation on a canvas.

## Phase 1: Specification (The "Lazy" Definition)

When you call `Chart::build()` and chain methods like `.mark_point()` or `.encode()`, Charton does not perform any calculations. Instead, it populates a ChartSpec.

- Intent Gathering: The system records which columns are mapped to which channels (X, Y, Color, etc.).
- Lazy Evaluation: Data remains in its original DataFrame. This allows you to define complex multi-layer charts without triggering expensive computations prematurely.

## Phase 2: Training (The Arbitration)

As defined in the `Layer` trait, the system must perform a "Training" phase before rendering.

- Data Extraction: The `LayeredChart` triggers `get_data_bounds()` for every layer.
- Columnar Efficiency: Thanks to the Apache Arrow integration, Charton accesses contiguous memory slices (`&[T]`) for specific columns. This "Columnar" approach minimizes CPU cache misses.
- Domain Resolution: The orchestrator merges these bounds into a global ScaleDomain. This is where the "mathematical truth" of the chart is established.

## Phase 3: Layout Negotiation

Before a single pixel is drawn, Charton must solve a spatial puzzle: How much space is left for the data after placing the labels? As seen in `layout.rs`, Charton uses a Greedy Stacking Algorithm:

- First Pass (Measurement): The engine estimates the width/height of axis titles and tick labels based on font metrics.
- Constraint Calculation: It subtracts these dimensions from the total canvas size to determine the `PanelContext`—the exact "Physical Rectangle" where the data marks will live.
- Legend Placement: Legends are "stacked" (either vertically or horizontally) using a Flex-box style logic, further refining the available plotting area.

## Phase 4: Realization (Mapping to Geometry)

Now the abstract data meets physical space. The `Mapper` system takes over:

- Coordinate Translation: The `CoordSystem` transforms normalized data values into physical `(x, y)` coordinates within the Plot Panel.
- Visual Mapping: The `VisualMapper` converts normalized ratios into concrete visual properties:
    - `0.5` $\rightarrow$ `#ff0000` (Color)
    - `0.8` $\rightarrow$ `PointShape::Diamond` (Shape)
    - `0.2` $\rightarrow$ `4.0px` (Radius)

## Phase 5: Rendering (The Final Output)
The final stage involves the RenderBackend. Charton iterates through the resolved geometric primitives (Circles, Paths, Rects) and translates them into the target format:

- SVG/PDF: Generates vector instructions for high-fidelity documents.
- PNG/Raster: Uses hardware-accelerated drawing for high-performance previews.
- HTML/Canvas: Renders interactive frames for web environments.

## Key Takeaway: The "Single-Pass" Advantage

Because Charton resolves all scales and layouts before the drawing phase, the actual rendering is a "Single-Pass" operation. This predictable flow is what enables Charton to handle millions of points with near-zero latency, as the expensive logical "negotiations" are handled once per frame.