# Summary
- [Getting Started](getting_started.md)
    - [Installation](getting_started/installation.md)
    - [Quick Start](getting_started/quick_start.md)
    - [Interactive Notebooks](getting_started/interactive_notebooks)

# Concepts & Philosophy
- [The Charton Mental Model](concepts/mental_model.md)
- [The Life of a Chart: From Data to Pixels](concepts/chart_life.md)
- [Scale Arbitration](concepts/scale_arbitration.md)
- [System Architecture](concepts/system_architecture.md)

# The Grammar of Graphics
- [Data & Transformation](data_transformation.md)
- [Encodings & Channels](encodings_channels.md)
- [Scales & Domains](scales_domains.md)
    - [The Temporal Engine: Precise Time Series](temporal_engine.md)
- [Marks & Geometries](marks_geometries.md)
- [Coordinate Systems](coordinate_systems.md)
    -[Cartesian](cartesian.md)
    -[Polar](polar.md)
    -[Geographic Projections](geographic_projections.md)

# Composition & Layout
- [Layering: The multi-Layer Grammar](layering.md)
- [Concatenation & Faceting](concatenation_faceting.md)
- [The Layout Engine: Geometry & Space](layout_engine.md)
- [Guides: Axis & Legends](guides.md)

# Industrial Mastery
- [Performance Scaling: 大规模数据、LazyFrame 与下采样策略。]
- [Theme System: 实现 NEJM/出版级视觉的一致性。]
- [Error Handling & Type Safety]
    -The Type-Safe Grammar: 编译时的属性校验。
    -The ChartonError Registry: 运行时错误分类与诊断。
    -Data Integrity: 处理 NaN、Null 与异常值的策略。
- [Production Integration]: CI/CD 中的图表自动化与测试。

# Chart Gallery
- [Basic Marks & Geometries](gallery/basic_marks.md)
    - [Points & Bubbles](gallery/point_charts.md)
    - [Lines & Paths](gallery/line_charts.md)
    - [Bars & Columns](gallery/bar_charts.md)
    - [Area & Stacking](gallery/area_charts.md)
    - [Ticks](gallery/tick_chart.md)
- [Statistical Distributions](gallery/distributions_and_statistics.md)
    - [Histograms & Density](gallery/distributions.md)
    - [Boxplots & Violins](gallery/box_violin_charts.md)
    - [Uncertainties & Trends](gallery/uncertainties_and_trends.md)
- [Temporal & Sequential Analysis](gallery/temporal_sequential_analysis.md)
    - [Time Series Analysis](gallery/time_series.md)
    - [Ranges & Gantt Charts](gallery/ranges_gantt.md)
    - [Temporal Heatmaps](gallery/calendar_plots.md)
- [Relationships & Matrices](gallery/relationships_matrices.md)
    - [Scatter Matrices (SPLOM)](gallery/scatter_matrices.md)
    - [Heatmaps & Grids](gallery/heatmaps.md)
    - [Circular & Radial Charts](gallery/circular_charts.md)
- [Composition & Layouts](gallery/composition_layouts.md)
    - [Layering Marks](gallery/layering.md)
    - [Faceting (Small Multiples)](gallery/faceting.md)
    - [Concatenation & Dashboards](gallery/concatenation.md)
- [Interactive & Specialized Views](gallery/interactive_specialized_views.md)
    - [Selection & Brushing](gallery/interactivity.md)
    - [Geospatial & Projections](gallery/geospatial.md)
    - [Custom Plotting Logic](gallery/custom_logic.md)
- [Web & Frontend Integration](gallery/web_frontend_integration.md)
    - [WASM Runtime Rendering](web/wasm_runtime.md)
    - [Vega-Lite Schema Export](web/vegalite_json.md)

# Case Studies
- [Biomedicine & Life Sciences](case_studies/biomedicine.md)
    - [Semaglutide Weight Loss Curve](case_studies/semaglutide.md)
- [Data Science & Engineering](case_studies/data_science.md)
- [Machine Learning & AI](case_studies/ai.md)
- [Mathematics & Physics](case_studies/math.md)
- [Social & Humanities](case_studies/social.md)
- [Finance & Economics](case_studies/finance.md)
- [Geography & Geospatial](case_studies/geography.md)

# WebAssembly & Vega-Lite JSON
- [WASM Runtime Rendering](web/wasm_runtime.md)
- [Vega-Lite Schema Export](web/vegalite_json.md)

# The IPC Bridge
- [Seamless Python Interop](ipc/python_interop.md)