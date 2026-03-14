# Summary

[Introduction](README.md)
[Getting Started](GETTING_STARTED.md)

# Concepts
- [The Data Engine (Polars)](concepts/data_engine.md)
- [System Architecture](concepts/architecture.md)

# The Grammar
- [Coordinate Systems](grammar/coordinates.md)
- [Encodings & Aesthetics](grammar/encoding.md)
- [Marks & Geometry](grammar/marks.md)

# Styling & Themes
- [Global Themes](styling/themes.md)
- [Axes & Legends](styling/axes_styling.md)
- [Colors & Palettes](styling/colors.md)
- [The Styling Model (Hybrid Pattern)](style/styling_model.md)

# Chart Gallery

Explore the expressive power of Charton’s Grammar of Graphics through these practical examples.

## Basic Marks & Geometries
- [Points & Bubbles](gallery/point_charts.md) — Visualizing individual observations and magnitude.
- [Lines & Paths](gallery/line_charts.md) — Connecting data points to show continuity and order.
- [Bars & Columns](gallery/bar_charts.md) — Discrete comparisons and rankings.
- [Area & Stacking](gallery/area_charts.md) — Visualizing volume and part-to-whole relationships.

## Statistical Distributions
- [Histograms & Density](gallery/distributions.md) — Understanding data shape and frequency.
- [Boxplots & Violins](gallery/box_violin_charts.md) — Visualizing quartiles, outliers, and density.
- [Uncertainties & Trends](gallery/uncertainties_and_trends.md) — Error bars, confidence intervals, and regression fits.

## Temporal & Sequential Analysis
- [Time Series Analysis](gallery/time_series.md) — *Powered by High-Performance Time Coordinates*
- [Ranges & Gantt Charts](gallery/ranges_gantt.md) — Visualizing events with duration and overlap.
- [Temporal Heatmaps](gallery/calendar_plots.md) — Grid-based patterns over days, weeks, or months.

## Relationships & Matrices
- [Scatter Matrices (SPLOM)](gallery/scatter_matrices.md) — Multi-variable correlations.
- [Heatmaps & Grids](gallery/heatmaps.md) — Dense matrices for structured relationship data.
- [Circular & Radial Charts](gallery/circular_charts.md) — Periodic patterns and radial projections.

## Composition & Layouts
- [Layering Marks](gallery/layering.md) — Combining multiple geometries in a single view.
- [Faceting (Small Multiples)](gallery/faceting.md) — Visualizing categorical subsets via grids.
- [Concatenation & Dashboards](gallery/concatenation.md) — Aligning independent charts for comparison.

## Interactive & Specialized Views
- [Selection & Brushing](gallery/interactivity.md) — Real-time filtering via WASM.
- [Geospatial & Projections](gallery/geospatial.md) — Mapping data to geographic coordinates.
- [Custom Plotting Logic](gallery/custom_logic.md) — Extending Charton for unique requirements.

# Web & Frontend Integration
- [WASM Runtime Rendering](web/wasm_runtime.md)
- [Vega-Lite Schema Export](web/vegalite_json.md)

# Advanced Features
- [Data Transformations](advanced/transformation.md)
- [Ecosystem Integration](advanced/integration.md)
- [Faceting, Legends & Layout](advanced/layout.md)
