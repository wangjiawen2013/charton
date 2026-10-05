# Summary

# Getting Started
- [Installation](getting_started/installation.md)
- [Quick Start](getting_started/quick_start.md)
- [Interactive Notebooks](getting_started/notebooks.md)

# Concepts & Philosophy
- [The Charton Mental Model](concepts/mental_model.md)
- [The Layer Pipeline](concepts/grammar_pipeline.md)
- [Design Rules: Marks, Tiers & Statistical Atoms](concepts/design_rules.md)
- [System Architecture](concepts/architecture.md)
- [From Data to Pixels](concepts/chart_life.md)
- [Scale Arbitration](concepts/scale_arbitration.md)
- [Missing Values & Gaps](concepts/missing_values.md)
- [Rendering Backends](concepts/rendering.md)
- [GPU Architecture & the WgpuRenderer](concepts/gpu.md)

# The Core Engine: Dataset
- [The Atomic Unit: ColumnVector](engine/column_vector.md)
- [The Temporal Engine](engine/temporal.md)
- [The Dataset Struct](engine/dataset_core.md)
- [Data Ingestion & Polars](engine/ingestion.md)
- [Compute & Transformation](engine/compute.md)
- [Validation & Integrity](engine/integrity.md)

# The Grammar of Graphics
- [Encodings & Channels](grammar/encodings.md)
- [Scales & Domains](grammar/scales.md)
- [Marks & Geometries](grammar/marks.md)
- [Transforms & Columns](grammar/transforms.md)
- [Coordinate Systems](grammar/coordinates.md)

# Composition & Layout
- [Layering Grammar](layout/layering.md)
- [Multi-View: Faceting & Concatenation](layout/views.md)
- [Space Manager](layout/space.md)
- [Guides: Axis & Legends](layout/guides.md)

# Industrial Mastery
- [Performance & Scaling](industrial/performance.md)
- [Publication-Ready Themes](industrial/themes.md)
- [Safety & Error Registry](industrial/safety.md)
- [Production Integration](industrial/production.md)

# Cookbook
- [Recipe Index](gallery/index.md)
- [Histogram](gallery/histogram.md)
- [1-D Density](gallery/density_1d.md)
- [2-D Density](gallery/density_2d.md)
- [Cumulative Density](gallery/cumulative_density.md)
- [Violin](gallery/violin.md)
- [Box Plots](gallery/box_plot.md)
- [Bars & Error Bars](gallery/uncertainties_and_trends.md)
- [Line Charts](gallery/line_charts.md)
- [Area Charts](gallery/area_charts.md)
- [Scatter](gallery/point_charts.md)
- [Strip & Rug Plots](gallery/tick_chart.md)
- [Heatmaps](gallery/heatmaps.md)
- [Contour Plots](gallery/contours.md)
- [Circular Charts](gallery/circular_charts.md)
- [Faceting](gallery/faceting.md)
- [Geospatial](gallery/geospatial.md)

# Case Studies
- [Biomedicine (NEJM Study)](case_studies/biomedicine.md)
- [Feature Analysis for Classification (Iris)](case_studies/data_science.md)
- [Risk Dashboard for an Equity Index](case_studies/finance.md)
- [Economist-Style Highlights](case_studies/economist_style.md)
- [Lorenz Attractor](case_studies/lorenz_attractor.md)
- [System Monitor (egui Dashboard)](case_studies/system_monitor.md)

# WebAssembly & Vega-Lite JSON
- [Vega-Lite Schema Export](web/vegalite_json.md)
- [WASM CPU-Driven SVG Animation](web/wasm_cpu_svg.md)
- [WASM WGPU Blazing-Fast Rendering](web/wasm_gpu_wgpu.md)

# Native GUI & Engine Integration
- [GUI Integration Concepts](gui/concepts.md)
- [Zero-Copy Rendering](gui/zero_copy.md)
- [Integrating with Winit & WGPU](gui/winit.md)
- [Integrating with Bevy](gui/bevy.md)
- [Integrating with egui](gui/egui.md)

# Ecosystem & Integrations
- [The Nushell Plugin](ecosystem/nushell.md)
- [Nushell Plugin Internals](ecosystem/nushell_internals.md)

# Appendix
- [Wgpu Text](appendix/wgpu_text.md)
