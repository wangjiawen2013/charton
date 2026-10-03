# Summary

# Getting Started
- [Installation](getting_started/installation.md)
- [Quick Start](getting_started/quick_start.md)
- [Interactive Notebooks](getting_started/notebooks.md)

# Concepts & Philosophy
- [The Charton Mental Model](concepts/mental_model.md)
- [The Layer Pipeline](concepts/grammar_pipeline.md)
- [System Architecture](concepts/architecture.md)
- [From Data to Pixels](concepts/chart_life.md)
- [Scale Arbitration](concepts/scale_arbitration.md)
- [Rendering Backends](concepts/rendering.md)
- [Gpu Architecture](concepts/gpu.md)
- [The WgpuRenderer Internals](concepts/wgpu_renderer.md)

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
- [Transforms & Columns](grammar/transforms.md)       # transform 读哪些列、产出哪些列
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
- [Violin](gallery/violin.md)
- [Statistical Distributions](gallery/statistics.md)
- [Bars & Error Bars](gallery/uncertainties_and_trends.md)
- [Line Charts](gallery/line_charts.md)
- [Area Charts](gallery/area_charts.md)
- [Strip & Rug Plots](gallery/tick_chart.md)
- [Circular Charts](gallery/circular_charts.md)
- [Heatmaps](gallery/heatmaps.md)
- [Scatter](gallery/point_charts.md)
- [Contour Plots](gallery/contours.md)
- [Box & Violin Combinations](gallery/box_violin_charts.md)
- [Faceting](gallery/faceting.md)
- [Geospatial](gallery/geospatial.md)

# Case Studies
- [Biomedicine (NEJM Study)](case_studies/biomedicine.md)
- [Data Science & ML](case_studies/data_science.md)
- [Finance & Economics](case_studies/finance.md) 
- [Economist-Style Highlights](case_studies/economist_style.md)
- [Lorenz Attractor](case_studies/lorenz_attractor.md)

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
