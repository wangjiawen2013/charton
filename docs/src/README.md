# Introduction
## What is Charton? (The Core Idea)
Charton is a modern Rust visualization library designed around a simple, declarative framework for data visualization.

- Declarative API: It offers an API similar to Python's Altair/Vega-Lite, allowing users to define "what to visualize" rather than "how to draw it.". That is, "the Grammar of Graphics".
- Native Polars Support: Charton is tightly integrated with the high-performance Rust DataFrame library Polars, enabling efficient, zero-copy data plotting.
- Dual Rendering Capability: You can utilize its pure Rust SVG renderer for dependency-free plotting, or leverage its IPC mechanism to seamlessly connect with external Python visualization ecosystems like Altair and Matplotlib.

## Design Philosophy and Key Advantages
Charton is engineered to be an efficient, safe, and flexible solution, built on the principle that visualization should be declarative.
- 🚀 Performance and Safety: It leverages Rust's strong type system to achieve compile-time safety and utilizes Polars' integration for superior data handling performance.
- 💡 Layered and Expressive: It features a multi-layer plotting architecture that easily combines various marks (e.g., points, lines, bars, boxplots, error bars) within a shared coordinate system to create complex composite visualizations.
- 🌐 Frontend Ready: It can generate standard Vega-Lite JSON specifications, making it ready for easy integration into modern web applications using libraries like React-Vega or Vega-Embed.
- 🔗 Efficient Integration: Through Inter-Process Communication (IPC), it efficiently communicates with external Python libraries, avoiding slow, temporary file operations and maintaining compatibility with environments like Conda in Jupyter.
- 📓 Jupyter Interactivity: It offers native support for the evcxr Jupyter Notebook environment, enabling interactive and real-time exploratory data analysis.

## System Architecture
Charton adopts a modern, decoupled architecture designed for high-performance data processing and cross-language interoperability.
```text
┌───────────────────────────────────────────────────────────────────────────┐
│                            Input Layer                                    │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────────┐ │
│  │ Rust Polars  │    │ External     │    │ Jupyter/evcxr Interactive    │ │
│  │ DataFrame/   │    │ Datasets     │    │ Input                        │ │
│  │ LazyFrame    │    │ (CSV/Parquet)│    │ (Notebook cell data/commands)│ │
│  └──────────────┘    └──────────────┘    └──────────────────────────────┘ │
└────────────────────────────────────┬──────────────────────────────────────┘
                                     │
┌────────────────────────────────────▼──────────────────────────────────────┐
│                          Core Layer                                       │
│  ┌──────────────────────────────────────────────────────────────────────┐ │
│  │            Charton Core Engine                                       │ │
│  │  ┌──────────────┐    ┌───────────────┐    ┌──────────────────────┐   │ │
│  │  │ Declarative  │    │ Layered       │    │ Cross-backend Data   │   │ │
│  │  │ API (Altair- │    │ Chart         │    │ Converter            │   │ │
│  │  │ style)       │    │ Management    │    │ (Rust ↔ Python/JSON) │   │ │
│  │  └──────────────┘    │ (LayeredChart)│    └──────────────────────┘   │ │
│  │                      └───────────────┘                               │ │
│  │  ┌──────────────┐    ┌───────────────┐    ┌──────────────────────┐   │ │
│  │  │ Data         │    │ IPC           │    │ Vega-Lite Spec       │   │ │
│  │  │ Validation/  │    │ Communication │    │ Generator            │   │ │
│  │  │ Mapping      │    │ Module        │    │                      │   │ │
│  │  └──────────────┘    └───────────────┘    └──────────────────────┘   │ │
│  └──────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────┬──────────────────────────────────────┘
                                     │
┌────────────────────────────────────▼──────────────────────────────────────┐
│                        Render Backends                                    │
│  ┌──────────────────────┐    ┌────────────────────────────────────────┐   │
│  │ Rust Native Backend  │    │ External Cross-Language Backends       │   │
│  │                      │    │                                        │   │
│  │  ┌────────────────┐  │    │  ┌─────────────┐  ┌──────────────────┐ │   │
│  │  │ Pure Rust SVG  │  │    │  │ Altair      │  │ Matplotlib       │ │   │
│  │  │ Renderer       │  │    │  │ Backend     │  │ Backend          │ │   │
│  │  └────────────────┘  │    │  │ (Python IPC)│  │ (Python IPC)     │ │   │
│  │                      │    │  └─────────────┘  └──────────────────┘ │   │
│  │  ┌────────────────┐  │    │                                        │   │
│  │  │ Wasm Renderer  │  │    │  ┌────────────┐  ┌──────────────────┐  │   │
│  │  │ (Partial       │  │    │  │ Other      │  │ Extended Backends│  │   │
│  │  │  Support)      │  │    │  │ Python     │  │ (Future)         │  │   │
│  │  └────────────────┘  │    │  │ Viz Libs   │  │ (R/Julia, etc.)  │  │   │
│  │                      │    │  └────────────┘  └──────────────────┘  │   │
│  └──────────────────────┘    └────────────────────────────────────────┘   │
└────────────────────────────────────┬──────────────────────────────────────┘
                                     │
┌────────────────────────────────────▼──────────────────────────────────────┐
│                          Output Layer                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │ SVG Vector   │  │ Vega-Lite    │  │ PNG Bitmap   │  │ Jupyter      │   │
│  │ Graphics     │  │ JSON         │  │ Image        │  │ Inline       │   │
│  │ (Native/Wasm)│  │ (for Web)    │  │ (via Ext.)   │  │ Rendering    │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
└───────────────────────────────────────────────────────────────────────────┘
```

**1. Input Layer (Data Orchestration)**
- **Polars-Native**: Unlike other libraries that require heavy data cloning, Charton is built on **Apache Arrow** (via Polars), enabling efficient, zero-copy data access.
- **Versatile Sourcing**: It supports `DataFrame` and `LazyFrame`, allowing for out-of-core data processing before visualization.

**2. Core Layer (The Grammar Engine)**
- **Declarative DSL**: A type-safe implementation of the **Grammar of Graphics**, allowing users to compose complex visualizations using intuitive tuples (e.g., `.encode((x, y, color))`).
- **Universal Data Bridge**: This is the core innovation of Charton. It utilizes **Parquet-serialized bytes** as an intermediate format to exchange data between different Polars versions and languages, effectively bypassing Rust's orphan rules and dependency conflicts.
- **Vega-Lite Spec Generator**: A high-level compiler that transforms Rust structures into standard Vega-Lite JSON for seamless frontend integration.

**3. Render Backends (Multi-Engine)**
- **Rust Native Backend**: A **zero-dependency**, pure Rust implementation. It uses a custom SVG renderer for ultra-fast, server-side batch generation and provides partial support for WebAssembly (Wasm).
- **IPC Bridge (External)**: For features not yet in the native engine, Charton provides a high-speed Inter-Process Communication (IPC) bridge to Python’s mature ecosystem (**Altair/Matplotlib**), eliminating the need for slow temporary disk I/O.

**4. Output Layer (Multi-Format Delivery)**
- **Vector & Raster**: Support for SVG and high-resolution PNG (via `resvg`).
- **Web & Notebook**: Direct JSON output for **React/Vue** integration and inline rendering for **evcxr Jupyter** notebooks.

## Why This Architecture Matters
🚀 **Solving the "Version Hell"**

In the Rust ecosystem, if your project depends on Polars `v0.50` and a plotting library depends on `v0.40`, your code won't compile. Charton’s **Parquet-encoded IPC** bypasses this entirely, making it the most robust visualization tool for production Rust environments.

🔌 **Hot-Swappable Backends**

You can develop interactively using the **Altair backend** to leverage its rich feature set, and then switch to the **Native SVG backend** for deployment to achieve maximum performance and minimum container size.

🌐 **Frontend-First Design**

By generating standard **Vega-Lite JSON**, Charton allows you to handle heavy data lifting in Rust while letting the browser’s GPU handle the final rendering via `Vega-Embed` or `React-Vega`.
