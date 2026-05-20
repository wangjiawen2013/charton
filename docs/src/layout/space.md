# Space Manager

In the Grammar of Graphics, defining data layers is only the first step. To create a professional visualization, you must orchestrate the placement of axes, legends, titles, and the plotting area itself. The Space Manager is Charton’s layout engine, responsible for the precise partitioning of the physical canvas.

## The Box Model Philosophy

Charton adopts a nested "Box Model" similar to modern web layout engines. Every chart is composed of concentric rectangular regions, each serving a specific purpose:

1. Plot Panel: The central core where coordinates are resolved and geometric marks are rendered.
2. Axis Region: The area surrounding the Plot Panel, housing tick marks, labels, and axis titles.
3. Guide Region: The outermost boundary used for placement of Legends and ColorBars.
4. Canvas Padding: The external buffer area that prevents outer elements (such as rotated labels) from being clipped by the container boundary.

## The Greedy Backfilling Algorithm

The Space Manager faces a classic "chicken-and-egg" problem: to determine the final size of the canvas, we need the axis label widths; but to calculate the density of axis labels, we need to know the available width.

Charton resolves this through a two-phase Backfilling Algorithm:

* Phase 1: Estimation: The engine uses font metrics (such as `estimate_text_width`) to calculate the theoretical minimum space required by all Axis and Legend regions.
* Phase 2: Constraint Injection: These calculated depths are injected into `AxisLayoutConstraints` and `LegendLayoutConstraints` structs. The Space Manager then shrinks the canvas from the outer edges toward the center, reserving the necessary space for guides and axes before finalizing the `PanelContext` for the core data plot.

## Layout Control Strategies

The Space Manager offers fine-grained control to ensure visual uniformity across complex plots:

### Synchronized Alignment

Even in multi-panel (faceted) visualizations, the Space Manager enforces "Synchronized Layouts." If Panel A has very long tick labels while Panel B has short ones, the Space Manager calculates the maximum required depth across all panels and applies it uniformly, ensuring that the plotting areas remain perfectly aligned.

### Greedy Stacking (Flex-box logic)

When managing multiple legends or colorbars, the Space Manager applies a strategy similar to Flex-box layout:

* Horizontal Stacking: Legends are laid out in a row; if the content exceeds the canvas width, it automatically wraps to a new row.
* Vertical Stacking: When positioned on the sides, legends are stacked in columns, dynamically adjusting the axis spacing to accommodate the total height required by the consolidated legend blocks.

### Aspect Ratio Preservation

While mark positions are data-driven, the Space Manager respects global aspect ratio constraints. If a fixed aspect ratio is requested, the manager calculates the optimal `Panel` size and treats the resulting excess pixels as additional outer padding, ensuring that the visual representation of the data remains undistorted.

## Coordinate-Driven Compensation

The Space Manager is tightly coupled with the Coordinate System:

* Rotation Compensation: When using Polar coordinates (e.g., in a radial plot), the Space Manager automatically detects the "angular sweep." It triggers a rotation compensation logic that recalculates the collision boundary for labels, preventing them from overlapping with the circular plotting area.
* Flip Awareness: When a user invokes `coord_flip()`, the manager automatically swaps the depth-calculation logic for the axes. It recognizes that vertical labels in a standard plot become horizontal labels in a flipped plot, adjusting the padding calculations accordingly.

## Key Takeaways

* Nested Regions: Layout follows an "inside-out" box model progression.
* Two-Phase Backfilling: The algorithm solves the cyclic dependency between label sizing and canvas allocation.
* Synchronized Layouts: The engine ensures that multiple sub-plots maintain perfect alignment regardless of local content differences.
* Adaptive Compensation: Layout strategies are coordinate-aware, automatically preventing overlaps based on whether the chart is Cartesian or Radial.

*With this chapter, you have completed the journey through the Charton orchestration pipeline—from data ingestion and aesthetic mapping to spatial layout and rendering.*