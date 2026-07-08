# Summary

# Getting Started
- [Installation](getting_started/installation.md)
- [Quick Start](getting_started/quick_start.md)
- [Interactive Notebooks](getting_started/notebooks.md)

# Concepts & Philosophy
- [The Charton Mental Model](concepts/mental_model.md)
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
- [Compute & Transformation](engine/compute.md)            # 高级操作：GroupBy、Slice、Take 与下采样算法
- [Validation & Integrity](engine/integrity.md)            # 数据质量：长度校验、空值 (NaN/Null) 处理策略

# The Grammar of Graphics
- [Encodings & Channels](grammar/encodings.md)
- [Scales & Domains](grammar/scales.md)
- [Marks & Geometries](grammar/marks.md)
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

# Chart Gallery
- [Basic Marks](gallery/basic_marks.md)                    # 基础图表：点、线、柱、面积图
- [Statistical Distributions](gallery/statistics.md)       # 统计展示：Jitter, Beeswarm, Boxplot, Density
- [Temporal Analysis](gallery/temporal.md)                 # 时间维度：走势图、甘特图、日历图
- [Relationships & Matrices](gallery/matrices.md)          # 多维关系：热力图、散点矩阵、径向图
- [Specialized Views](gallery/special.md)                  # 进阶图表：地理地图、自定义绘图逻辑

# Case Studies
- [Biomedicine (NEJM Study)](case_studies/biomedicine.md)
- [Data Science & ML](case_studies/data_science.md)        # 机器学习：训练曲线、混淆矩阵、残差分析
- [Finance & Economics](case_studies/finance.md)           # 金融领域：K 线图、波动率与收益率分析
- [Lorenz Attractor](case_studies/lorenz_attractor.md)

# WebAssembly & Vega-Lite JSON
- [Vega-Lite Schema Export](web/vegalite_json.md)          # 导出标准 JSON 以支持 Web 框架集成
- [WASM CPU-Driven SVG Animation](web/wasm_cpu_svg.md)
- [WASM WGPU Blazing-Fast Rendering](web/wasm_gpu_wgpu.md)

# Native GUI & Engine Integration
- [GUI Integration Concepts](gui/concepts.md)
- [Zero-Copy Rendering](gui/zero_copy.md)
- [Integrating with Winit & WGPU](gui/winit.md)
- [Integrating with Bevy](gui/bevy.md)
- [Integrating with egui](gui/egui.md)

# The IPC Bridge
- [Seamless Python Interop](ipc/python_interop.md)

# Appendix
- [Wgpu Text](appendix/wgpu_text.md)
