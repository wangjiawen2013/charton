# Summary

# Getting Started
- [Installation](getting_started/installation.md)          # 环境配置与 Cargo 特性（Feature）选择
- [Quick Start](getting_started/quick_start.md)            # 宏语法与 Builder API 的快速对比示例
- [Interactive Notebooks](getting_started/notebooks.md)    # 在 Jupyter/evcxr 中的交互式绘图配置

# Concepts & Philosophy
- [The Charton Mental Model](concepts/mental_model.md)     # 声明式绘图思维与所有权处理逻辑
- [System Architecture](concepts/architecture.md)          # Input/Core/Render/Output 四层分层架构图
- [From Data to Pixels](concepts/chart_life.md)            # 图表渲染的生命周期：从原始数据到几何图形
- [Scale Arbitration](concepts/scale_arbitration.md)       # 缩放同步机制：多个 Layer 如何共用一个 Scale

# The Core Engine: Dataset
- [The Dataset Struct](engine/dataset_core.md)             # 基础定义：内存布局、Arc 共享及列的增删改查 (CRUD)
- [Data Ingestion & Polars](engine/ingestion.md)           # 数据来源：Polars 零拷贝转换与原生类型导入
- [Compute & Transformation](engine/compute.md)            # 高级操作：GroupBy、Slice、Take 和下采样算法
- [Validation & Integrity](engine/integrity.md)            # 数据质量：长度校验、空值 (NaN/Null) 处理策略

# The Grammar of Graphics
- [Encodings & Channels](grammar/encodings.md)             # 数据维度到视觉通道（x, y, color）的映射
- [Scales & Domains](grammar/scales.md)                    # 定义域与值域：线性、对数、序数缩放
- [The Temporal Engine](grammar/temporal.md)               # 高精度时间序列：纳秒级精度与格式化
- [Marks & Geometries](grammar/marks.md)                   # 几何标记：Point, Line, Bar, Area 等
- [Coordinate Systems](grammar/coordinates.md)             # 笛卡尔坐标、极坐标与地理投影

# Composition & Layout
- [Layering Grammar](layout/layering.md)                   # 逻辑组合：在同一坐标系叠加多个图层
- [Multi-View: Faceting & Concatenation](layout/views.md)  # 视图组合：分面图与多图并列排版
- [Space Manager](layout/space.md)                         # 空间排版：边距、对齐、画布比例与间距控制
- [Guides: Axis & Legends](layout/guides.md)               # 辅助元素：坐标轴刻度与图例自动生成

# Industrial Mastery
- [Performance & Scaling](industrial/performance.md)       # 大规模数据处理、ahash 加速与并行渲染
- [Publication-Ready Themes](industrial/themes.md)         # 视觉一致性：预设主题与 NEJM 出版级样式
- [Safety & Error Registry](industrial/safety.md)          # 错误分类诊断与类型安全的语法校验
- [Production Integration](industrial/production.md)       # 自动化测试、CI/CD 集成与部署建议

# Chart Gallery
- [Basic Marks](gallery/basic_marks.md)                    # 基础图表：点、线、柱、面积图
- [Statistical Distributions](gallery/statistics.md)       # 统计展示：Jitter, Beeswarm, Boxplot, Density
- [Temporal Analysis](gallery/temporal.md)                 # 时间维度：走势图、甘特图、日历图
- [Relationships & Matrices](gallery/matrices.md)          # 多维关系：热力图、散点矩阵、径向图
- [Specialized Views](gallery/special.md)                  # 进阶图表：地理地图、自定义绘图逻辑

# Case Studies
- [Biomedicine (NEJM Study)](case_studies/biomedicine.md)  # 经典案例：复现 NEJM 临床试验数据曲线
- [Data Science & ML](case_studies/data_science.md)        # 机器学习：训练曲线、混淆矩阵、残差分析
- [Finance & Economics](case_studies/finance.md)           # 金融领域：K 线图、波动率与收益率分析

# WebAssembly & Vega-Lite JSON
- [WASM Runtime Rendering](web/wasm_runtime.md)            # Rust 前端渲染：在浏览器中高效绘图
- [Vega-Lite Schema Export](web/vegalite_json.md)          # 导出标准 JSON 以支持 Web 框架集成

# The IPC Bridge
- [Seamless Python Interop](ipc/python_interop.md)         # 高性能桥接：调用 Altair/Matplotlib 的原理