# Chapter 1 · Introduction
## 1.1 What is Charton? (The Core Idea)
Charton is a modern Rust visualization library designed around a simple, declarative framework for data visualization.

- Declarative API: It offers an API similar to Python's Altair/Vega-Lite, allowing users to define "what to visualize" rather than "how to draw it."
- Native Polars Support: Charton is tightly integrated with the high-performance Rust DataFrame library Polars, enabling efficient, zero-copy data plotting.
- Dual Rendering Capability: You can utilize its pure Rust SVG renderer for dependency-free plotting, or leverage its IPC mechanism to seamlessly connect with external Python visualization ecosystems like Altair and Matplotlib.

## 1.2 Design Philosophy and Key Advantages
Charton is engineered to be an efficient, safe, and flexible solution, built on the principle that visualization should be declarative.
- 🚀 Performance and Safety: It leverages Rust's strong type system to achieve compile-time safety and utilizes Polars' integration for superior data handling performance.
- 💡 Layered and Expressive: It features a multi-layer plotting architecture that easily combines various marks (e.g., points, lines, bars, boxplots, error bars) within a shared coordinate system to create complex composite visualizations.
- 🌐 Frontend Ready: It can generate standard Vega-Lite JSON specifications, making it ready for easy integration into modern web applications using libraries like React-Vega or Vega-Embed.
- 🔗 Efficient Integration: Through Inter-Process Communication (IPC), it efficiently communicates with external Python libraries, avoiding slow, temporary file operations and maintaining compatibility with environments like Conda in Jupyter.
- 📓 Jupyter Interactivity: It offers native support for the evcxr Jupyter Notebook environment, enabling interactive and real-time exploratory data analysis.

## 1.3 System Architecture
Charton adopts a modern, decoupled architecture designed for high-performance data processing and cross-language interoperability.
```text
┌───────────────────────────────────────────────────────────────────────────┐
│                            Input Layer                                    │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────────┐ │
│  │ Rust Polars  │    │ External     │    │ Jupyter/evcxr Interactive    │ │
│  │ DataFrame/   │    │ Datasets     │    │ Input                        │ │
│  │ LazyFrame    │    │ (CSV/Parquet)│    │ (Notebook cell data/commands)│ │
│  └──────────────┘    └──────────────┘    └──────────────────────────────┘ │
└───────────────────────────┬───────────────────────────────────────────────┘
                            │
┌───────────────────────────▼───────────────────────────────────────────────┐
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
└───────────────────────────┬───────────────────────────────────────────────┘
                            │
┌───────────────────────────▼───────────────────────────────────────────────┐
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
└───────────────────────────┬───────────────────────────────────────────────┘
                            │
┌───────────────────────────▼───────────────────────────────────────────────┐
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

## 1.4 Why This Architecture Matters
🚀 **Solving the "Version Hell"**

In the Rust ecosystem, if your project depends on Polars `v0.50` and a plotting library depends on `v0.40`, your code won't compile. Charton’s **Parquet-encoded IPC** bypasses this entirely, making it the most robust visualization tool for production Rust environments.

🔌 **Hot-Swappable Backends**

You can develop interactively using the **Altair backend** to leverage its rich feature set, and then switch to the **Native SVG backend** for deployment to achieve maximum performance and minimum container size.

🌐 **Frontend-First Design**

By generating standard **Vega-Lite JSON**, Charton allows you to handle heavy data lifting in Rust while letting the browser’s GPU handle the final rendering via `Vega-Embed` or `React-Vega`.

# Chapter 2 · Quick Start
Welcome to **Charton Quick Start**! 

This chapter will guide you through creating charts in Rust using Charton from scratch. By the end of this chapter, you'll know how to:

- Initialize a Rust project and add Charton dependencies
- Load and preprocess data using Polars
- Build charts using Chart, Mark, and Encoding
- Render charts in multiple formats and environments
- Avoid common pitfalls and errors

The goal is to make you productive **within minutes**.

## 2.1 Project Setup
First, create a new Rust project:

```bash
cargo new demo
cd demo
```
Edit your `Cargo.toml` to add Charton and Polars dependencies:
```toml
[dependencies]
charton = "0.2.0"
polars = { version = "0.49", features = ["lazy", "csv", "parquet"] }
```
Run `cargo build` to ensure everything compiles.

## 2.2 Creating Your First Chart
Charton adopts a **declarative visualization** philosophy, drawing heavily from the design principles of Altair and Vega-Lite. Every Charton chart is composed of **three core elements** which allow you to specify *what* you want to see, rather than *how* to draw it:

1. **Chart** – The base container that holds your data (`Chart::build(&df)`).
2. **Mark** – The visual primitive you choose (point, bar, line, etc., defined by `.mark_point()`).
3. **Encoding** – The mapping that links data fields to visual properties (x, y, color, size, etc., defined by `.encode(...)`).

**Example: Analyzing Car Weight vs. MPG (Scatter Plot)**

This minimal Charton example uses the built-in `mtcars` dataset to create a scatter plot of car weight (`wt`) versus miles per gallon (`mpg`).
```rust
use charton::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Data Preparation (Polars)
    let df = load_dataset("mtcars")?
        .lazy()
        .with_columns([col("gear").cast(DataType::String)]) // Cast 'gear' for categorical coloring
        .collect()?;

    // 2. Chart Declaration (Chart, Mark, Encoding)
    Chart::build(&df)?          // Chart: Binds the data source
        .mark_point()           // Mark: Specifies the visual primitive (dots)
        .encode((
            x("wt"),            // Encoding: Maps 'wt' (weight) to the X-axis
            y("mpg"),           // Encoding: Maps 'mpg' (fuel efficiency) to the Y-axis
        ))?
        // 3. Converted to Layered Chart
        .into_layered()
        // 4. Saving the Layered Chart to SVG
        .save("./scatter_chart.svg")?;

    println!("Chart saved to scatter_chart.svg");
    Ok(())
}
```
You can also display the result directly in your evcxr jupyter notebook using the `show()` method for quick iteration:
```rust
// ... (using the same 'df' DataFrame)
Chart::build(&df)?
    .mark_point()
    .encode((x("wt"), y("mpg")))?
    .into_layered()
    .show()?;
```

You can even save the chart object to a variable and use it later. For example:
```rust
// ... (using the same 'df' DataFrame)
let chart = Chart::build(&df)?
    .mark_point()
    .encode((x("wt"), y("mpg")))?
    .into_layered();

chart.save("./scatter_chart.svg")?; // or chart.show()?
```
This mirrors the **declarative style of Altair**, now in Rust.

**Explicit form**

The code above is equivalent to the following explicit construction using LayeredChart (see chapter 5).
```rust
use charton::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Data Preparation (Polars)
    let df = load_dataset("mtcars")?
        .lazy()
        .with_columns([col("gear").cast(DataType::String)]) // Cast 'gear' for categorical coloring
        .collect()?;

    // 2. Chart Declaration (Chart, Mark, Encoding)
    let scatter = Chart::build(&df)?    // Chart: Binds the data source
        .mark_point()                   // Mark: Specifies the visual primitive (dots)
        .encode((
            x("wt"),                    // Encoding: Maps 'wt' (weight) to the X-axis
            y("mpg"),                   // Encoding: Maps 'mpg' (fuel efficiency) to the Y-axis
        ))?;
    
    // 3. Create a layered chart
    LayeredChart::new() 
        .add_layer(scatter)             // Add the chart as a layer of the layered chart
        .save("./scatter_chart.svg")?;  // Save the layered chart

    println!("Chart saved to scatter_chart.svg");
    Ok(())
}
```

## 2.3 Loading and Preparing Data
Before creating visualizations, Charton requires your data to be stored in a Polars `DataFrame`. Charton itself does not impose restrictions on how data is loaded, so you can rely on Polars’ powerful I/O ecosystem.

### 2.3.1 Built-in Datasets
Charton provides a few built-in datasets for quick experimentation, demos, and tutorials.
```rust
let df = load_dataset("mtcars")?;
```
This returns a Polars DataFrame ready for visualization.

### 2.3.2 Loading CSV Files
CSV is the most common format for tabular data. Using Polars:
```rust
use polars::prelude::*;

let df = CsvReadOptions::default()
    .with_has_header(true)
    .try_into_reader_with_file_path(Some("./datasets/iris.csv".into()))?
    .finish()?;
```

### 2.3.3 Loading Parquet Files
Parquet is a high-performance, columnar storage format widely used in data engineering.
```rust
let file = std::fs::File::open("./datasets/foods.parquet")?;
let df = ParquetReader::new(file).finish()?;
```
Parquet is recommended for large datasets due to compression and fast loading.

### 2.3.4 Loading Data from Parquet Bytes (`Vec<u8>`) — Cross-Version Interoperability
One of the challenges when working with the Polars ecosystem is that **different crates may depend on different Polars versions**, which prevents passing `DataFrame` values directly between libraries. Charton solves this problem by offering a **version-agnostic data exchange format** based on **Parquet-serialized bytes**.

Charton provides an implementation of:
```rust
impl TryFrom<&Vec<u8>> for DataFrameSource
```
This allows you to:

- Serialize a Polars `DataFrame` into Parquet bytes (`Vec<u8>`)
- Pass those bytes to Charton
- Let Charton deserialize them internally using its Polars version
- Avoid Polars version conflicts entirely

This is especially useful when your application depends on a uncompatible Polars version with Charton. By using Parquet bytes as the intermediate format, **data can be exchanged safely across Polars versions**.

**Example: Passing a DataFrame to Charton Using Parquet Bytes**

Below is a full example demonstrating:

1. Creating a Polars `DataFrame`
2. Serializing it into Parquet bytes using your Polars version
3. Passing those bytes to Charton
4. Rendering a scatter plot

**Cargo.toml**
```toml
[dependencies]
polars = { version = "0.51", features = ["parquet"] }
charton = { version = "0.2.0" }
```
**Source Code Example**
```rust
use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create a Polars DataFrame using Polars 0.51
    let df = df![
        "length" => [5.1, 4.9, 4.7, 4.6, 5.0, 5.4, 4.6, 5.0, 4.4, 4.9],
        "width"  => [3.5, 3.0, 3.2, 3.1, 3.6, 3.9, 3.4, 3.4, 2.9, 3.1]
    ]?;

    // Serialize DataFrame into Parquet bytes
    let mut buf: Vec<u8> = Vec::new();
    ParquetWriter::new(&mut buf).finish(&mut df.clone())?;

    // Build a Chart using the serialized Parquet bytes
    Chart::build(&buf)?
        .mark_point()
        .encode((
            x("length"),
            y("width"),
        ))?
        .into_layered()
        .save("./scatter.svg")?;

    Ok(())
}
```

## 2.4 Simple Plotting Examples
This section introduces the most common chart types in Charton.

### 2.4.1 Line Chart
```rust
// Create a polars dataframe
let df = df![
    "length" => [4.4, 4.6, 4.7, 4.9, 5.0, 5.1, 5.4], // In ascending order
    "width" => [2.9, 3.1, 3.2, 3.0, 3.6, 3.5, 3.9]
]?;

// Create a line chart layer
Chart::build(&df)?
    .mark_line()               // Line chart
    .encode((
        x("length"),           // Map length column to X-axis
        y("width"),            // Map width column to Y-axis
    ))?
    .into_layered()
    .save("line.svg")?;
```
Useful for trends or ordered sequences.

### 2.4.2 Bar Chart
```rust
let df = df! [
    "type" => ["a", "b", "c", "d"],
    "value" => [4.9, 5.3, 5.5, 6.5],
]?;

Chart::build(&df)?
    .mark_bar()
    .encode((
        x("type"),
        y("value"),
    ))?
    .into_layered()
    .save("bar.svg")?;
```

### 2.4.3 Histogram
```rust
let df = load_dataset("iris")?;

Chart::build(&df)?
    .mark_hist()
    .encode((
        x("sepal_length"),
        // The number of data points (or Frequency) falls into the corresponding bin are named "count".
        // You can use any arbitray name for the y-axis, here we use "count".
        y("count")
    ))?
    .into_layered()
    .save("hist.svg")?;
```
Charton automatically computes bin counts when `y("count")` is specified.

### 2.4.4 Boxplot
```rust
let df = load_dataset("iris")?;

Chart::build(&df)?
    .mark_boxplot()
    .encode((x("species"), y("sepal_length")))?
    .into_layered()
    .save("boxplot.svg")?;
```
Boxplots summarize distributions using quartiles, medians, whiskers, and outliers.

### 2.4.5 Layered Charts
In Charton, complex visualizations are built by **layering multiple charts** on the same axes. Each layer defines a single mark type, and layers are composed to form a unified view with shared scales and coordinates.
```rust
use charton::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a polars dataframe
    let df = df![
        "length" => [4.4, 4.6, 4.7, 4.9, 5.0, 5.1, 5.4],
        "width" => [2.9, 3.1, 3.2, 3.0, 3.6, 3.5, 3.9]
    ]?;

    // Create a line chart layer
    let line = Chart::build(&df)?
        .mark_line()                        // Line chart
        .encode((
            x("length"),                    // Map length column to X-axis
            y("width"),                     // Map width column to Y-axis
        ))?;

    // Create a scatter point layer
    let scatter = Chart::build(&df)?
        .mark_point()                       // Scatter plot
        .encode((
            x("length"),                    // Map length column to X-axis
            y("width"),                     // Map width column to Y-axis
        ))?;

    LayeredChart::new()       
        .add_layer(line)                    // Add the line layer
        .add_layer(scatter)                 // Add the scatter point layer
        .save("./layeredchart.svg")?;

    Ok(())
}
```

## 2.5 Exporting Charts
Charton supports exporting charts to different file formats depending on the selected rendering backend. All backends share the same API:

```rust
chart.save("output.png")?;
```
The file format is inferred from the extension.

This section describes the supported formats and saving behavior for each backend.

### 2.5.1 Rust Native Backend

The Rust native backend is the default renderer and supports:

- **SVG** — vector graphics output
- **PNG** — rasterized SVG (using resvg with automatic system font loading)

**Saving SVG**
```rust
chart.save("chart.svg")?;
```
**Saving PNG**

PNG is generated by rasterizing the internal SVG at 2× resolution:
```rust
chart.save("chart.png")?;
```
This produces high-quality PNG output suitable for publication.

### 2.5.2 Altair Backend (Vega-Lite)

The Altair backend uses Vega-Lite as the rendering engine and supports:

- **SVG** — via Vega → SVG conversion
- **PNG** — SVG rasterized via resvg
- **JSON** — raw Vega-Lite specification

**Saving SVG**
```rust
chart.save("chart.svg")?;
```
**Saving PNG**
```rust
chart.save("chart.png")?;
```
**Saving Vega-Lite JSON**
```rust
chart.save("chart.json")?;
```
The JSON file can be opened directly in the online Vega-Lite editor.

### 2.5.3 Matplotlib Backend

The Matplotlib backend supports:

- **PNG** — returned as base64 from Python, decoded and saved

**Saving PNG**
```rust
chart.save("chart.png")?;
```
Other formats (SVG, JSON, PDF, etc.) are not currently supported by this backend.

### 2.5.4 Unsupported Formats & Errors

Charton will return an error if:

- The file extension is missing
- The extension is not supported by the selected backend
- SVG → PNG rasterization fails
- File write errors occur

Example:
```rust
if let Err(e) = chart.save("output.bmp") {
    eprintln!("Save error: {}", e);
}
```
### 2.5.5 Summary of Supported Formats
| Backend     | SVG | PNG | JSON |
| ----------- | :-: | :-: | :--: |
| Rust Native |  ✔️ |  ✔️ |   ❌  |
| Altair      |  ✔️ |  ✔️ |  ✔️  |
| Matplotlib  |  ❌  |  ✔️ |   ❌  |

### 2.5.6 Exporting Charts as Strings (SVG / JSON)
In addition to saving charts to files, Charton also supports exporting charts **directly as strings**.
This is useful in environments where writing to disk is undesirable or impossible, such as:

- Web servers returning chart data in API responses
- Browser/WASM applications
- Embedding charts into HTML templates
- Passing Vega-Lite specifications to front-end visualizers
- Testing and snapshot generation

Charton provides two kinds of in-memory exports depending on the backend.

#### 2.5.6.1 SVG Output (Rust Native Backend)
The Rust-native renderer can generate the complete SVG markup of a chart and return it as a `String`:
```rust
let svg_string = chart.to_svg()?;
```
This returns the full `<svg>...</svg>` element including:
- Layout
- Axes
- Marks
- Legends
- Background

The string can be:
- Embedded directly into HTML
- Returned from a web API
- Rendered inside a WASM application
- Passed to a templating engine such as Askama or Tera

**Example**
```rust
let svg = chart.to_svg()?;
```
#### 2.5.6.2 Vega-Lite JSON (Altair Backend)
When using the Altair backend, charts can be exported as raw **Vega-Lite JSON**:
```rust
let json = chart.to_json()?;
```
This produces the complete Vega-Lite specification generated by Altair. Typical usage scenarios include:
- Front-end rendering using Vega/Vega-Lite
- Sending the chart spec from a Rust API to a browser client
- Storing chart specifications in a database
- Generating reproducible visualization specs

**Example**
```rust
let json_spec = chart.to_json()?;
println!("{}", json_spec);
```
This JSON is fully compatible with the **official online Vega-Lite editor**.

#### 2.5.6.3 Summary: In-Memory Export Methods

| Backend     | `to_svg()`    | `to_json()`              |
| ----------- | ------------- | ------------------------ |
| Rust Native | ✔️ SVG string | ❌ unsupported            |
| Altair      | ❌ (file-only) | ✔️ Vega-Lite JSON string |
| Matplotlib  | ❌             | ❌                        |

String-based export complements file export by enabling fully in-memory rendering and programmatic integration.

## 2.7 Viewing Charts
Charton charts can be viewed directly inside **Evcxr Jupyter notebooks** using the `.show()` method.  

When running inside Evcxr Jupyter, Charton automatically prints the correct MIME content so that the chart appears inline.

Outside Jupyter (e.g., running a binary), `.show()` does nothing and simply returns `Ok(())`.

The rendering behavior differs depending on the selected backend.

## 2.6.1 Rust Native Backend
The Rust-native backend renders charts to **inline SVG**.  

When `.show()` is called inside Evcxr Jupyter, the SVG is printed using `text/html` MIME type.

### Example

```rust
use charton::prelude::*;
use polars::prelude::*;

let df = df![
    "x" => [1, 2, 3],
    "y" => [10, 20, 30]
]?;

let chart = Chart::build(&df)?
    .mark_point()
    .encode((x("x"), y("y")))?
    .into_layered();

chart.show()?;   // displays inline SVG in Jupyter
```
**Internal Behavior**
```text
EVCXR_BEGIN_CONTENT text/html
<svg>...</svg>
EVCXR_END_CONTENT
```
This enables rich inline SVG display in notebooks.

### 2.6.2 Altair Backend (Vega-Lite)
When using the Altair backend, `.show()` emits **Vega-Lite JSON** with the correct MIME type:
```bash
application/vnd.vegalite.v5+json
```
Jupyter then renders the chart using the built-in Vega-Lite renderer.

**Example**
```rust
chart.show()?;   // displays interactive Vega-Lite chart inside Jupyter
```
**Internal Behavior**
```text
EVCXR_BEGIN_CONTENT application/vnd.vegalite.v5+json
{ ... Vega-Lite JSON ... }
EVCXR_END_CONTENT
```
This produces interactive charts (tooltips, zooming, etc.) if supported by the notebook environment.

### Matplotlib Backend
The Matplotlib backend produces **base64-encoded PNG** images and sends them to the notebook using `image/png` MIME type.

**Example**
```rust
chart.show()?;   // displays inline PNG rendered by Matplotlib
```
**Internal Behavior**
```text
EVCXR_BEGIN_CONTENT image/png
<base64 image>
EVCXR_END_CONTENT
```
### 2.6.4 Summary: What `.show()` displays in Jupyter

| **Backend** | **Output Type** | **MIME Type**     |
| ----------- | ----------- | ---------------------------------- |
| Rust Native | SVG         | `text/html`                        |
| Altair      | Vega-Lite   | `application/vnd.vegalite.v5+json` |
| Matplotlib  | PNG         | `image/png`                        |

`.show()` is designed to behave naturally depending on the backend, giving the best viewing experience for each renderer.

## 2.7 Summary
In this chapter, you learned how to:
- Load datasets from CSV, Parquet, and built-in sources
- Create essential chart types: scatter, bar, line, histogram, boxplot, layered plots
- Export your charts to SVG, PNG, and Vega JSON
- Preview visualizations in the notebook

With these foundations, you now have everything you need to build **end-to-end data visualizations** quickly and reliably. The next chapters will introduce the building blocks of Charton, including marks and eocodings.

# Chapter 3 · Marks
Marks are the fundamental building blocks of Charton. A *mark* is any visible graphical primitive—points, lines, bars, areas, rectangles, arcs, text, boxplots, rules, histograms, and more.

Every chart in Charton is created by:
**1.** Constructing a base chart using `Chart::build()`.
**2.** Selecting a mark type (e.g., `mark_point()`, `mark_line()`).
**3.** Adding encodings that map data fields to visual properties.

Understanding marks is essential because **most visual expressiveness comes from combining marks with encodings.**

## 3.1 What Is a Mark?
In Charton, a mark is an object that implements the core trait:
```rust
pub trait Mark: Clone {
    fn mark_type(&self) -> &'static str;

    fn stroke(&self) -> Option<&SingleColor> { None }
    fn shape(&self) -> PointShape { PointShape::Circle }
    fn opacity(&self) -> f64 { 1.0 }
}
```
**Key Properties**
| **Property**| **Meaning**       | **Provided by Trait** |
| ----------- | ----------------- | ----------------- |
| `mark_type` | Unique identifier | required          |
| `stroke`    | Outline color     | default: none     |
| `shape`     | Point shape       | default: circle   |
| `opacity`   | Transparency      | default: 1.0      |

## 3.2 How Marks Work in Charton
A typical Charton chart:
```rust
Chart::build(&df)?
    .mark_point()
    .encode((
        x("x"),
        y("y")
    ))?
```
**Flow of Rendering**

**1.** `mark_point()` creates a `MarkPoint` object.

**2.** Encodings specify how data fields map to visual properties.

**3.** Renderer merges:
- mark defaults
- overriding encoding mappings
- automatic palettes

**4.** The final SVG/PNG is generated.

**Declarative Design Philosophy**

Charton follows an Altair-style declarative model:

> **If an encoding exists → encoding overrides mark defaults.**

> **If an encoding does not exist → use the mark’s own default appearance.**

This gives you:
- Short expressions for common charts
- Fine-grained control when needed

## 3.3 Point Mark
MarkPoint draws scattered points.

**Struct (simplified)**
```rust
pub struct MarkPoint {
    pub color: Option<SingleColor>,
    pub shape: PointShape,
    pub size: f64,
    pub opacity: f64,
    pub stroke: Option<SingleColor>,
    pub stroke_width: f64,
}
```
**Use Cases**
- Scatter plots
- Bubble charts
- Highlighting specific points
- Overlaying markers on other marks

**Correct Example**
```rust
Chart::build(&df)?
    .mark_point()
    .encode((
        x("sepal_length"),
        y("sepal_width"),
        color("species"),
        size("petal_length")
    ))?
```
## 3.4 Line Mark
MarkLine draws connected lines.

**Highlights**
- Supports LOESS smoothing
- Supports interpolation

**Struct**
```rust
pub struct MarkLine {
    pub color: Option<SingleColor>,
    pub stroke_width: f64,
    pub opacity: f64,
    pub use_loess: bool,
    pub loess_bandwidth: f64,
    pub interpolation: PathInterpolation,
}
```
**Example**
```rust
Chart::build(&df)?
    .mark_line().transform_loess(0.3)
    .encode((
        x("data"),
        y("value"),
        color("category")
    ))?
```
## 3.5 Bar Mark
A bar mark visualizes categorical comparisons.

**Struct**
```rust
pub struct MarkBar {
    pub color: Option<SingleColor>,
    pub opacity: f64,
    pub stroke: Option<SingleColor>,
    pub stroke_width: f64,
    pub width: f64,
    pub spacing: f64,
    pub span: f64,
}
```
**Use Cases**
- Vertical bars
- Grouped bars
- Stacked bars
- Horizontal bars

**Example**
```rust
Chart::build(&df)?
    .mark_bar()
    .encode((
        x("type"),
        y("value"),
    ))?
```
## 3.6 Area Mark
Area marks fill the area under a line.

**Example**
```rust
Chart::build(&df)?
    .mark_area()
    .encode((
        x("time"),
        y("value"),
        color("group")
    ))?
```
## 3.7 Arc Mark (Pie/Donut)
Arc marks draw circular segments.

**Example (donut)**
```rust
Chart::build(&df)?
    .mark_arc()  // Use arc mark for pie charts
    .encode((
        theta("value"),  // theta encoding for pie slices
        color("category"),  // color encoding for different segments
    ))?
    .with_inner_radius_ratio(0.5) // Creates a donut chart
```
## 3.8 Rect Mark (Heatmap)
Used for heatmaps and 2D densities.

**Example**
```rust
Chart::build(&df)?
    .mark_rect()
    .encode((
        x("x"),
        y("y"),
        color("value"),
    ))?
```
## 3.9 Boxplot Mark
Visualizes statistical distributions.

**Example**
```rust
Chart::build(&df_melted)?
    .mark_boxplot()
    .encode((
        x("variable"),
        y("value"),
        color("species")
    ))?
```
## 3.10 ErrorBar Mark
Represents uncertainty intervals.

**Example**
```rust
// Create error bar chart using transform_calculate to add min/max values
Chart::build(&df)?
    // Use transform_calculate to create ymin and ymax columns based on fixed std values
    .transform_calculate(
        (col("value") - col("value_std")).alias("value_min"),  // ymin = y - std
        (col("value") + col("value_std")).alias("value_max")   // ymax = y + std
    )?
    .mark_errorbar()
    .encode((
        x("type"),
        y("value_min"),
        y2("value_max")
    ))?
```
## 3.11 Histogram Mark
Internally used to draw histogram bins.

**Example**
```rust
Chart::build(&df)?
    .mark_hist()
    .encode((
        x("value"),
        y("count").with_normalize(true),
        color("variable")
    ))?
```
## 3.12 Rule Mark
Draws reference lines.

**Example**
```rust
Chart::build(&df)?
    .mark_rule()
    .encode((
        x("x"),
        y("y"),
        y2("y2"),
        color("color"),
    ))?
```
## 3.13 Text Mark
Places textual annotations.

**Example**
```rust
Chart::build(&df)?
    .mark_text().with_text_size(16.0)
    .encode((
        x("GDP"),
        y("Population"),
        text("Country"),
        color("Continent"),
    ))?
```
## 3.14 Summary
* Each mark defines a visual primitive.
* Marks are combined with *encodings* to bind data to graphics.
* Charton uses a declarative approach:
    * Encodings override mark defaults.
    * Palette and scales are automatically applied.
* By choosing the correct mark, you control how data is represented.

# Chapter 4 · Encodings
Encodings are the core of Charton’s declarative visualization system. They determine **how data fields map to visual properties** such as:
- Position (`x`, `y`, `y2`, `theta`)
- Color
- Shape
- Size
- Text labels

Every chart in Charton combines:
1. **A mark** (point, line, bar, arc, rect, etc.)
2. **Encodings** that map data fields to visual channels

This chapter explains all encoding channels, how they work, and provides complete code examples using **mtcars.**

## 4.1 What Are Encodings?
An encoding assigns a *data field* to a *visual* channel.
```rust
Chart::build(&df)?
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        color("cyl"),
    ))?;
```
This produces a scatter plot:

* **X axis** → horsepower
* **Y axis** → miles per gallon
* **Color** → number of cylinders

## 4.2 Encoding System Architecture
Every encoding implements the following trait:
```rust
pub trait IntoEncoding {
    fn apply(self, enc: &mut Encoding);
}
```

Users **never** interact with `Encoding` directly.
They simply write:
```rust
.encode((x("A"), y("B"), color("C")))
```

The API supports tuple-based composition of up to **9 encodings.**

## 4.3 Position Encodings
### 4.3.1 X – Horizontal Position

The **X** channel places data along the horizontal axis.

✔ **When to use** `X`

- Continuous values (e.g., `hp`, `mpg`, `disp`)
- Categorical values (`cyl`, `gear`, `carb`)
- Histogram binning
- Log scales

**API**
```rust
x("column_name")
```
**Optional settings**
```rust
x("hp")
    .with_bins(30)
    .with_scale(Scale::Log)
    .with_zero(true)
```
**Example: mtcars horsepower vs mpg**
```rust
let df = load_dataset("mtcars");

Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
    ));
```
Expected: Scatter plot showing `hp` vs `mpg`.

### 4.3.2 Y – Vertical Position

The **Y** channel has identical behavior to `X`.

**Example**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("wt"),
        y("mpg"),
    ));
```
Expected: Heavier cars generally have lower mpg.

### 4.3.3 Y2 – Second Vertical Coordinate

Used when a mark needs **two vertical positions:**

- Interval bands
- Confidence intervals
- Error bars
- Range rules

**Example: Upper & Lower MPG Bounds**
```rust
Chart::build(&df)
    .mark_area()
    .encode((
        x("hp"),
        y("mpg_low"),
        y2("mpg_high"),
    ));
```
## 4.4 Angular Position: θ (Theta)
Used in:

- Pie charts
- Donut charts
- Radial bar charts

**Example: Pie chart of cylinders**
```rust
Chart::build(&df)
    .mark_arc()
    .encode((
        theta("count"),
        color("cyl"),
    ));
```
## 4.5 Color Encoding
Color maps a field to the fill color of a mark.

✔ **When to use**

- Categorical grouping
- Continuous magnitude
- Heatmaps
- Parallel categories

**Example: Color by gears**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        color("gear"),
    ));
```
## 4.8 Shape Encoding
**Shape – Point Symbol Mapping**

Only applies to **point marks**.

**Available shapes include:**

- Circle
- Square
- Triangle
- Cross
- Diamond
- Star

**Example**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        shape("cyl"),
    ));
```
## 4.9 Size Encoding
**Size – Radius / Area Encoding**

Used for:

- Bubble plots
- Weighted scatter plots
- Emphasizing magnitude

**Example: Bubble plot with weight**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        size("wt"),
    ));
```
## 4.10 Opacity Encoding
**Opacity – Transparency**

Used for:

- Reducing overplotting
- Encoding density
- Showing relative uncertainty

**Example: Opacity mapped to horsepower**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("wt"),
        y("mpg"),
        opacity("hp"),
    ));
```
## 4.11 Text Encoding
**Text – Label Encoding**

Works with:

- Point labels
- Bar labels
- Annotation marks

**Example: Label each point with car model**
```rust
Chart::build(&df)
    .mark_text()
    .encode((
        x("hp"),
        y("mpg"),
        text("model"),
    ));
```
## 4.12 Stroke Encoding
**Stroke – Outline Color**

Useful when:

- Fill color is already used
- Emphasizing boundaries
- Donut chart outlines

**Example**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        stroke("gear"),
    ));
```
## 4.13 Stroke Width Encoding
**Stroke Width – Border Thickness**

Used for:

- Highlighting
- Encoding magnitude
- Interval charts

**Example**
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        stroke_width("wt"),
    ));
```
## 4.14 Combined Example: All Encodings

This chart uses eight encodings simultaneously:
```rust
Chart::build(&df)
    .mark_point()
    .encode((
        x("hp"),
        y("mpg"),
        color("cyl"),
        shape("gear"),
        size("wt"),
        opacity("qsec"),
        stroke("carb"),
        stroke_width("drat"),
    ));
```

Expected:
A rich multi-dimensional visualization of `mtcars`.

## 4.15 Tips & Best Practices

**✔ Use color for major categories**
Examples: `cyl`, `gear`, `carb`.

**✔ Use size sparingly**
Only when magnitude matters.

**✔ Avoid using both color & shape unless required**
Choose one main grouping.

**✔ Use opacity to reduce overplotting**
mtcars has many overlapping data points.

**✔ Avoid encoding more than 5 dimensions**
Human perception becomes overloaded.

## 4.16 Summary Table
| **Channel**    | **Purpose**          | **Works With** | **Example**         |
| -------------- | -------------------- | -------------- | ---------------------- |
| `x`            | horizontal position  | all marks      | `x("hp")`              |
| `y`            | vertical position    | all marks      | `y("mpg")`             |
| `y2`           | interval upper bound | area, rule     | `y2("high")`           |
| `theta`        | angle (pie/donut)    | arc            | `theta("count")`       |
| `color`        | fill color           | all            | `color("cyl")`         |
| `shape`        | symbol               | point          | `shape("gear")`        |
| `size`         | area/size            | point          | `size("wt")`           |
| `opacity`      | transparency         | point/area     | `opacity("hp")`        |
| `text`         | labels               | text mark      | `text("model")`        |
| `stroke`       | outline color        | point/rect/arc | `stroke("carb")`       |
| `stroke_width` | outline thickness    | all            | `stroke_width("drat")` |

# Chapter 5 · Layered Charts
Layered charts allow you to **stack multiple visual layers** on the same coordinate system. In Charton, a `LayeredChart` can combine any number of independent `Chart` layers—each with its own data, encodings, marks, transforms, and scale mappings. All layers automatically share the same **coordinate system, axes, scales, and overall styling**.

Layered charts are one of the most expressive visualization tools provided by Charton, enabling you to build statistical graphics, scientific plots, and analytical figures that combine multiple visual cues into a single coherent chart.

## 5.1 What Is a Layered Chart?

A `LayeredChart` is a container holding:
- global chart properties (width, height, margins, axes, theme, etc.)
- a list of layer objects (each a normal `Chart`)
- optional overrides for axis domains, labels, ticks, legends, and background

Conceptually, a layered chart is like **Photoshop layers** for data visualization:
- each layer can be a line, bar, point, area, rule, annotation, etc.
- layers are drawn in sequence (first layer at bottom, last layer on top)
- all layers share the same coordinate system

This model follows the design of **Vega-Lite’s layering, Altair’s layered charts**, and **ggplot2’s geoms**.

## 5.2 Why Layering Matters
Layering is essential for building analytical and scientific visualizations. Here are the key advantages.

### 5.2.1 Shared coordinate system
All layers automatically share:
- x/y axes
- axis labels
- axis domains
- grid lines
- coordinate transformation

This eliminates the need to manually align axis scales across layers.

### 5.2.2 Rich multi-factor visual expression
Layering is perfect for showing multiple aspects of a dataset:
- scatter points + smooth trend
- bars + error bars
- distributions + reference lines
- model predictions + raw data
- confidence intervals + regression lines

### 5.2.3 Ideal for scientific and statistical graphics
Most scientific plots are inherently layered:
- mean ± standard deviation
- line + shaded confidence interval
- theoretical vs. empirical distributions
- baseline rules
- highlight annotations

Charton makes these easy and declarative.

### 5.2.4 Fully modular
Each layer is independently constructed:
```rust
let layer1 = Chart::build(&df)?.mark_line().encode((x("time"), y("value")))?;
let layer2 = Chart::build(&df)?.mark_point().encode((x("time"), y("value")))?;
```
Then simply stack them.

### 5.2.5 Familiar to Altair/Vega-Lite users
Charton’s layering model is intentionally similar to existing declarative charting libraries, making knowledge transferable.

## 5.3 Creating a Layered Chart
The typical workflow:
```rust
LayeredChart::new()
    .add_layer(layer1)
    .add_layer(layer2)
    .save("output.svg")?;
```
Each `layer` is a complete, standalone `Chart` with its own:
- mark type
- encodings
- transforms

This modularity makes layering flexible and composable.

## 5.4 Example: Line + Scatter Overlay
A common pattern: points show raw measurements, the line shows the trend.
```rust
use charton::prelude::*;

let df = load_dataset("mtcars")?;
let df = df.sort(["hp"], SortMultipleOptions::default())?;

let line = Chart::build(&df)?
    .mark_line()
    .encode((x("hp"), y("mpg")))?;

let scatter = Chart::build(&df)?
    .mark_point()
    .encode((x("hp"), y("mpg")))?;

LayeredChart::new()
    .add_layer(line)
    .add_layer(scatter)
    .with_x_label("Horsepower")
    .with_y_label("Miles Per Gallon")
    .save("line_points.svg")?;
```
This combined visualization shows both the **trend** and the **raw variability**.

## 5.5 Example: Bar Chart + Error Bars
Charton supports statistical overlays such as error bars:
```rust
use charton::prelude::*;
use polars::prelude::*;

let df = {
    let mut df = load_dataset("mtcars")?.head(Some(3));
    df.with_column(Series::new("mpg_std", vec![1.0; df.height()]))?;
    df.lazy().with_column(col("qsec").cast(DataType::String)).collect()?
};

let errorbar = Chart::build(&df)?
    .transform_calculate(
        (col("mpg") - col("mpg_std")).alias("mpg_min"),
        (col("mpg") + col("mpg_std")).alias("mpg_max"),
    )?
    .mark_errorbar()
    .encode((x("qsec"), y("mpg_min"), y2("mpg_max")))?;

let bar = Chart::build(&df)?
    .mark_bar()
    .encode((x("qsec"), y("mpg")))?;

LayeredChart::new()
    .add_layer(bar)
    .add_layer(errorbar)
    .with_y_label("Miles Per Gallon")
    .save("bar_errorbar.svg")?;
```
This is a standard pattern in scientific and engineering reporting.

## 5.6 Example: Line + Error Bars (Time Series / Regression)
This layout is widely used for:
- regression mean ± CI
- temporal trends
- uncertainty visualization
```rust
let errorbar = Chart::build(&df)?
    .transform_calculate(
        (col("mpg") - col("mpg_std")).alias("mpg_min"),
        (col("mpg") + col("mpg_std")).alias("mpg_max"),
    )?
    .mark_errorbar()
    .encode((x("qsec"), y("mpg_min"), y2("mpg_max")))?;

let line = Chart::build(&df)?
    .mark_line()
    .encode((x("qsec"), y("mpg")))?;

LayeredChart::new()
    .add_layer(line)
    .add_layer(errorbar)
    .save("line_errorbar.svg")?;
```
## 5.7 Layering Reference Lines, Rules, and Annotations
You can combine data layers with semantic elements:
```rust
let df = load_dataset("mtcars")?;

let rule = Chart::build(&df)?
    .mark_rule()
    .encode((
        x("hp"),
        y("mpg"),
    ))?;

let scatter = Chart::build(&df)?
    .mark_point()
    .encode((
        x("hp"),
        y("mpg")
    ))?;

LayeredChart::new()
    .add_layer(scatter)
    .add_layer(rule)
    .save("reference_line.svg")?;
```

This is useful for:
- thresholds
- baselines
- highlighting important regions

## 5.8 Shared Axes, Scales, and Legends
`LayeredChart` intelligently merges:
- x/y domains, unless overridden
- tick values
- tick labels
- labels
- **legends** when layers use the same encoded fields

This provides consistent, aligned visual structure.

**Overriding axis domains**
```rust
LayeredChart::new()
    .with_x_domain(0.0, 350.0)
    .with_y_domain(0.0, 40.0)
```
**Overriding tick values or labels**
```rust
.with_x_tick_values(vec![0.0, 100.0, 200.0, 300.0])
.with_y_tick_labels(vec!["Low", "Medium", "High"])
```
**Controlling the legend**
```rust
.with_legend(true)
.with_legend_title("Engine Type")
```
## 5.9 Global Styling: Size, Margins, Theme, Background
Because `LayeredChart` owns global styling, you can adjust:

**Size**
```rust
.with_size(800, 500)
```
**Margins (proportional)**

Useful to accommodate long tick labels or large legends:
```rust
.with_left_margin(0.20)
.with_bottom_margin(0.18)
```
**Theme**
```rust
.with_theme(Theme::default())
```
**Background**
```rust
.with_background("#FAFAFA")
```
## 5.10 Advanced Example: Histogram + Reference Line + Density Curve
A classic triple-layer analytic plot.
```rust
let df = load_dataset("mtcars")?;

let hist = Chart::build(&df)?
    .mark_hist()
    .encode((x("mpg"), y("count")))?;

let rule = Chart::build(&df)?
    .mark_rule()
    .encode((x("mpg"), y("cyl")))?;

let density = Chart::build(&df)?
    .transform_density(
        DensityTransform::new("mpg")
            .with_as("mpg", "cumulative_density")
            .with_cumulative(true)
    )?
    .mark_line().with_line_color(Some(SingleColor::new("red")))
    .encode((x("mpg"), y("cumulative_density")))?;

LayeredChart::new()
    .add_layer(hist)
    .add_layer(rule)
    .add_layer(density)
    .save("hist_rule_density.svg")?;
```
## 5.11 Best Practices for Layered Charts
**✔ 1. Give each layer a clear conceptual purpose**

Examples:
- Layer 1: trend
- Layer 2: raw data
- Layer 3: uncertainty
- Layer 4: annotation

**✔ 2. Avoid too many layers**

More than 4–5 layers can cause visual clutter unless carefully styled.

**✔ 3. Avoid using opacity as the primary differentiator**

Instead prefer:
- color
- shape

**✔ 4. Think about rendering order**
- bottom layers: areas, bars, intervals
- top layers: lines, points, rules, annotations

**✔ 5. When in doubt, reduce complexity**

Layered plots are powerful, but simplicity is clarity.

## 5.12 Summary Table
| **Layer Type** | **Use Case**                             | **Example Fields** |
| -------------- | ---------------------------------------- | -------------- |
| **line**       | trends, regression, temporal analysis    | hp → mpg       |
| **point**      | raw data                                 | hp, mpg        |
| **bar**        | aggregated statistics                    | mpg by cyl     |
| **errorbar**   | uncertainty, variance                    | mpg ± sd       |
| **area**       | confidence intervals, distribution bands | lower–upper    |
| **rule**       | thresholds, baselines                    | fixed x or y   |
| **annotation** | labels, highlights                       | arbitrary      |

# Chapter 6 · Styling and Themes

A chart is not complete when it merely *works*.

A great chart communicates clearly, resonates visually, and fits seamlessly into its context—whether that context is a publication, dashboard, presentation slide, or internal report.

Charton provides a rich and structured styling system that allows you to control every aspect of chart appearance: themes, axes, fonts, sizes, colors, spacing, and layout.

Styling in Charton is **declarative, layered, and predictable**, following design principles familiar from Altair (Vega-Lite) and ggplot2.

## 6.1 Styling Model and Precedence
Charton follows a **three-level styling model**:

1. **Theme-level styling** (`Theme`)
Defines global defaults such as colors, fonts, paddings, and visual identity.
2. **Chart-level styling** (`LayeredChart` / common builders)
Adjusts layout, axes, domains, labels, and global chart properties.
3. **Mark-level styling** (`mark_*`)
Controls the appearance of individual visual elements such as color, size, and shape.

**Styling Precedence**

When multiple levels specify the same visual property, Charton resolves them in the following order:

> **Mark-level overrides** → **Chart-level overrides** → **Theme defaults**

This guarantees that:
- Themes establish a consistent visual baseline
- Charts can adapt styling to a specific figure
- Marks retain precise, local control when needed

## 6.2 Themes and Presets
Themes define the overall visual identity of a chart: colors, fonts, axis strokes, paddings, and spacing. In Charton, themes are represented by the `Theme` struct and applied globally:
```rust
chart.with_theme(Theme::default())
```
**Built-in Themes**

Charton provides several built-in themes:

- **Default** — light theme suitable for most use cases
- **Minimal** — reduced visual noise, thin strokes, no grid emphasis (To be implemented)
- **Classic** — thicker axes, Matplotlib-style appearance (To be implemented)
- **Dark** — optimized for dashboards and dark backgrounds (To be implemented)

Example:
```rust
let chart = LayeredChart::new()
    .with_theme(Theme::default());
```
**Customizing Theme Fields**

All theme fields are manually adjustable by overridding at the chart level.
```rust
let chart = LayeredChart::new()
    .with_theme(Theme::default())
    .with_label_font_size(10);
```

## 6.3 Chart-Level Styling: Axes and Layout
Chart-level styling is applied via shared builder methods and affects **all layers**.

**Axis Domains**

Override automatic domain inference:
```rust
chart
    .with_x_domain(0.0, 10.0)
    .with_y_domain_min(5.0);
```
**Axis Labels**
```
chart
    .with_x_label("Time (s)")
    .with_y_label("Intensity");
```
Padding and rotation:
```rust
chart
    .with_x_label_padding(25.0)
    .with_x_label_angle(-45.0);
```
**Tick Values and Labels**

**Continuous axis:**
```rust
chart.with_x_tick_values(vec![0.0, 2.0, 4.0, 6.0, 8.0]);
```
**Discrete axis:**
```rust
chart.with_x_tick_labels(vec!["A", "B", "C"]);
```
Rotate tick labels to avoid overlap:
```rust
chart.with_x_tick_label_angle(45.0);
```
Chart-level axis settings always override theme defaults.

## 6.4 Color Palettes and Colormaps
Charton supports multiple color control strategies.

**1. Mark-Level Colors**
```rust
mark_point().with_point_color("steelblue")
```
This always takes precedence.

**2. Encoded (Data-Driven) Colors**
```rust
.x("time")
.y("value")
.color("group")
```
Color scales are derived from the chart layer unless overridden.
```rust
// For discrete color scales, you can use a pre-defined palette.
mark_point().with_mark_palette(ColorPalette::Tab20)
```
or
```rust
// For continuous color scales, you can use a pre-defined colormap.
mark_point().with_color_map(ColorMap::Viridis)
```
## 6.5 Shapes and Sizes (Mark-Level Styling)
Shape and size are **mark-specific properties** and never affect other layers.

**Point Shape and Size**
```rust
let point = mark_point()
    .with_point_shape(PointShape::Circle)
    .with_point_size(60.0);
```
**Data-Driven Shape and Size**
```rust
mark_point()
    .encode((
        x("time"),
        y("value"),
        shape("group")
    ))?;
```
or
```rust
mark_point()
    .encode((
        x("wt"),
        y("mpg"),
        size("cyl")
    ))?;
```
Encoding-driven shape and size always use the chart defaults.

## 6.6 Font and Text Styling
Typography plays a major role in readability. Font and text styles currently inherit theme settings but can be overridden at the chart level.

**Chart Title**
```rust
chart
    .with_title("Gene Expression Overview")
    .with_title_font_family("Arial")
    .with_title_font_size(24)
    .with_title_color("red");
```
## 6.7 Chart Dimensions, Margins, and Background
**Dimensions**
```rust
chart.with_size(800, 600);
```
Larger sizes improve readability for dense charts.

**Margins**

Margins are expressed as **fractions of total size**:
```rust
chart
    .with_left_margin(0.15)
    .with_right_margin(0.10)
    .with_top_margin(0.10)
    .with_bottom_margin(0.15);
```

**Background and Legend**
```rust
chart.with_background("#fafafa");

chart
    .with_legend(true)
    .with_legend_title("Experimental Groups");
```
Legend appearance is influenced by the active theme.

## 6.8 Complete Example: Before and After Styling
**Basic Chart (Default Styling)**
```rust
let chart = Chart::build(&df)?
    .mark_point()
    .encode((x("x"), y("y")))?
    .into_layered();
```
**Styled Chart**
```rust
let chart = Chart::build(&df)?
    .mark_point()
    .encode((x("x"), y("y")))?
    .into_layered();

chart
    .with_theme(Theme::default())
    .with_title("Styled Scatter Plot")
    .with_x_label("X Value")
    .with_y_label("Y Value")
    .with_size(800, 600)
    .with_background("#ffffff")
    .save("chart.svg")?;
```
This demonstrates how themes, chart-level settings, and mark-level styling compose naturally.

**Style Resolution Summary**

| **Level** | **Scope** | **Typical Usage**            |
| ----- | --------- | -------------------------------- |
| Theme | Global    | Visual identity, fonts           |
| Chart | Per chart | Axes, layout, labels, domains    |
| Mark  | Per layer | Color, size, shape               |

Charton’s styling system is designed to be:
- **Declarative** — no imperative styling logic
- **Layer-aware** — global defaults with local overrides
- **Consistent** — predictable resolution rules

This allows users to create publication-quality visualizations with minimal effort, while still enabling deep customization when required.

# Chapter 7 · Advanced Charts
Basic marks such as points, lines, and bars are sufficient for many exploratory tasks, but real-world data analysis often requires **statistical summaries, distribution analysis, and structured comparisons**.

Charton provides a set of *advanced chart types* that are built on top of the same declarative grammar introduced in earlier chapters:

- Data is expressed as a `DataFrame`
- Visual structure is defined by **marks**
- Semantics are declared via **encodings**
- Rendering is handled automatically by the engine

What changes in this chapter is **what the marks compute internally**.

Advanced charts in Charton are *computation-aware*:

they derive statistical summaries (quantiles, counts, densities, bins) directly from the input data and render the results in a visually consistent way.

This chapter covers:

- Box plots and grouped statistical summaries
- Error bars and uncertainty visualization
- Density plots and kernel density estimation (KDE)
- Cumulative distributions (CDF / ECDF)
- Histograms (1D and 2D)
- Heatmaps and rect-based charts
- Pie and donut charts
- Rule-based overlays for annotations and thresholds

By the end of this chapter, you will be able to construct **publication-grade analytical visualizations** while keeping your code concise and expressive.

## 7.1 Box Plots and Grouped Statistical Summaries
Box plots (also called *box-and-whisker plots*) provide a compact summary of a distribution using its **five-number summary**:

- Minimum
- First quartile (Q1)
- Median
- Third quartile (Q3)
- Maximum

In addition, values outside the range

`[Q1 − 1.5 × IQR, Q3 + 1.5 × IQR]`

are treated as **outliers** and rendered explicitly.

Box plots are particularly useful for:

- Comparing distributions across categories
- Identifying skewness and spread
- Detecting outliers
- Visualizing grouped experimental results

Charton supports **grouped box plots out of the box**, allowing multiple distributions to be compared side-by-side within each category.

### 7.1.1 Grouped Box Plot
The most common box plot layout maps:

- a **categorical field** to the x-axis
- a **numeric field** to the y-axis
- an optional **grouping field** to color

Example (Iris dataset):
```rust
use charton::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = load_dataset("iris")?;

    // Convert wide-format measurements into long format
    let df_melted = df.unpivot(["sepal_length", "sepal_width", "petal_length", "petal_width"], ["species"])?;

    Chart::build(&df_melted)?
        .mark_boxplot()
        .encode((
            x("variable"),
            y("value"),
            color("species")
        ))?
        .into_layered()
        .save("./grouped_boxplot.svg")?;

    Ok(())
}
```
This produces a grouped box plot where:

- Each **measurement type** (`variable`) forms a group on the x-axis
- Each **species** is rendered as a separate box within that group
- Colors are automatically assigned from the active palette

### 7.1.2 How Grouped Box Plots Work Internally

Understanding the internal model helps explain Charton’s behavior and defaults.

For box plots, Charton:

1. Identifies the **grouping axis** (usually `x`)
2. Identifies the **value axis** (`y`)
3. Uses the **color encoding** (if present) as a secondary grouping dimension
4. Computes statistics *per (group × color) combination*

This means that:

- `x("variable")` defines where boxes are placed
- `color("species")` defines *how many boxes appear within each group*
- Missing combinations are handled gracefully and do not break layout

The order of categories is preserved from the input data, rather than being sorted alphabetically.

### 7.1.3 Visual Styling of Box Plots
Box plots are rendered using the `MarkBoxplot` mark type, which exposes a number of visual controls.

All of the following methods apply **only to box plot marks** and do not affect other layers.

**Box Fill and Opacity**
```rust
chart
    .with_box_color(Some(SingleColor::new("steelblue")))
    .with_box_opacity(0.7);
```
Note: If a color encoding exists, colors are taken from the palette.

**Stroke and Outline**
```rust
chart
    .with_box_stroke(Some(SingleColor::new("black")))
    .with_box_stroke_width(1.0);
```
- Stroke controls both the box outline and whiskers
- Set stroke to `None` to remove outlines entirely

**Outlier Appearance**
```rust
chart
    .with_outlier_color(Some(SingleColor::new("black")))
    .with_outlier_size(3.0);
```
- Outliers are rendered as point markers
- By default, they inherit a visible but unobtrusive style
- Increasing size is helpful for dense distributions

### 7.1.4 Layout Control for Grouped Boxes
When multiple box plots appear within the same category, Charton uses a **dodged layout**.

Three parameters control this layout:
```rust
chart
    .with_box_width(0.5)
    .with_box_spacing(0.2)
    .with_box_span(0.7);
```
- `width`: Base width of a single box (in data units)
- `spacing`: Gap between boxes within the same group, expressed as a ratio of box width
- `span`: Total horizontal space reserved for all boxes in a group

These values are automatically converted into pixel units during rendering, ensuring consistent appearance across output sizes.

The defaults are chosen to produce visually balanced grouped box plots without manual tuning.

### 7.1.5 Orientation and Axis Swapping
Box plots automatically respect axis orientation:

- Standard vertical boxes when `x` is categorical and `y` is numeric
- Horizontal boxes when axes are swapped

No additional configuration is required—orientation is inferred from encodings.

### 7.1.6 When to Use Box Plots
- Use box plots when you want to:
- Compare distributions across many categories
- Emphasize medians and quartiles instead of raw values
- Identify outliers quickly
- Summarize large datasets without overplotting

## 7.2 Error Bars and Uncertainty Visualization
Error bars are a fundamental tool for communicating **uncertainty, variability, and statistical confidence**.

Instead of showing individual data points, error bars summarize a group of observations by:

- A **central value** (typically the mean)
- A **lower bound** (e.g. mean − standard deviation)
- An **upper bound** (e.g. mean + standard deviation)

They are most commonly used to:
- Visualize measurement uncertainty
- Compare variability across categories
- Overlay statistical context on bar charts or point charts

In Charton, error bars are implemented as a dedicated mark type: `MarkErrorBar`.

### 7.2.1 Basic Error Bar Chart
At the API level, creating an error bar chart looks very similar to other marks:
```rust
use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df = df![
        "x" => ["a", "a", "a", "b", "b", "b", "c", "c", "c", "d", "d", "d"],
        "y" => [5.1, 5.3, 5.7, 6.5, 6.9, 6.2, 4.0, 4.2, 4.4, 7.6, 8.0, 7.8],
    ]?;

    let errorbar_chart = Chart::build(&df)?
        .mark_errorbar()
        .with_errorbar_color(Some(SingleColor::new("blue")))
        .with_errorbar_stroke_width(2.0)
        .with_errorbar_cap_length(5.0)
        .with_errorbar_center(true)
        .encode((
            x("x"),
            y("y"),
        ))?;

    LayeredChart::new()
        .with_size(500, 400)
        .with_title("Error Bar Chart with Mean and Std Dev")
        .add_layer(errorbar_chart)
        .save("./examples/errorbar.svg")?;

    Ok(())
}
```
This produces a chart where:
- Each unique `x` category forms one error bar
- The **center point** is the mean of `y`
- The **lower and upper bounds** are computed automatically
- Caps and line styles are rendered consistently across categories

### 7.2.2 Automatic Statistical Aggregation
A key design principle of Charton error bars is that **users declare intent, not computation**.

When you write:
```rust
.mark_errorbar()
.encode((x("x"), y("y")))
```
Charton automatically:
1. Groups data by the x encoding
2. Computes:
    - mean(`y`)
    - standard deviation(`y`)
3. Generates three internal fields:
    - `y` (mean)
    - `__charton_temp_y_min`
    - `__charton_temp_y_max`
This transformation happens transparently via the internal `transform_errorbar_data` step.

As a result:
- You do **not** need to pre-aggregate your data
- You do **not** need to manually compute error bounds
- The same chart definition works for raw observations

### 7.2.3 Error Bars with Explicit Bounds
In some cases, you may already have explicit min/max values (e.g. confidence intervals computed upstream).

Charton supports this via a secondary encoding:
```rust
.encode((
    x("x"),
    y("y_min"),
    y2("y_max"),
))
```
When `y2` is present:
- Charton **skips automatic aggregation**
- The provided fields are used directly
- This allows full control over uncertainty semantics

This design mirrors Vega-Lite’s `y` / `y2` pattern while remaining Rust-native.

### 7.2.4 Visual Styling of Error Bars
All visual styling is controlled via `MarkErrorBar`.

These methods affect only error bar marks and can be safely combined with other layers.

**Color and Opacity**
```rust
chart
    .with_errorbar_color(Some(SingleColor::new("black")))
    .with_errorbar_opacity(0.8);
```
- If no color encoding is present, a single color is used for all error bars
- Opacity is useful when overlaying error bars on dense charts

**Stroke Width**
```rust
chart.with_errorbar_stroke_width(1.5);
```
Controls the thickness of:
- The main error bar line
- The cap lines at both ends

**Cap Length**
```rust
chart.with_errorbar_cap_length(4.0);
```
Caps help visually distinguish the extent of uncertainty.
- Larger caps improve readability for sparse charts
- Smaller caps produce a cleaner, minimalist look

**Center Point Visibility**
```rust
chart.with_errorbar_center(true);
```
When enabled, a small marker is drawn at the center (mean) position.

This is especially useful when:
- Error bars are not overlaid on another mark
- The central tendency needs emphasis

### 7.2.5 Orientation and Axis Behavior
Error bars always represent **variation in the continuous axis**.

Internally:
- Bounds are transformed using the **y-axis scale**
- This remains true even when axes are swapped

As a result:
- Vertical error bars appear by default
- Horizontal error bars are rendered automatically when axes are swapped
- Log scales are handled correctly by transforming bounds before rendering

No additional configuration is required.

### 7.2.6 Layering Error Bars with Other Marks
Error bars are most powerful when combined with other chart types.

Typical patterns include:
- Bars + error bars
- Points + error bars
- Lines + error bars

Because error bars are standard `Chart` layers, you can simply add them:
```rust
LayeredChart::new()
    .add_layer(bar_chart)
    .add_layer(errorbar_chart);
```
Legend rendering is intentionally omitted for error bars, since they typically encode uncertainty rather than categories.

### 7.2.7 When to Use Error Bars
Use error bars when you want to:
- Communicate uncertainty or variability
- Summarize repeated measurements
- Compare stability across groups
- Avoid visual clutter from raw points

## 7.3 Density and Kernel Density Estimation (KDE)
While box plots and error bars summarize distributions using a small number of statistics, **density plots aim to visualize the full shape of a distribution**.

A density plot estimates a smooth probability distribution from raw samples, allowing you to:
- Identify multimodality
- Compare distribution shapes across groups
- Visualize skewness and tails
- Produce cumulative distribution curves (CDF)

In Charton, density estimation is implemented as a **data transformation**, not a mark.

This design cleanly separates:

- **Statistical computation** (`transform_density`)
- **Visual encoding** (`mark_line`, `mark_area`)

### 7.3.1 Density as a Data Transformation
Unlike histograms or box plots, density estimation is not a primitive mark.

Instead, you explicitly request a transformation:
```rust
chart.transform_density(DensityTransform::new("values"))?
```
This transformation:
1. Takes raw observations
2. Performs kernel density estimation (KDE)
3. Produces a new DataFrame with:
    - Evaluation points (x-axis)
    - Density values (y-axis)

You then visualize the result using any continuous mark, typically:
- `mark_line()` for density curves
- `mark_area()` for filled densities
- `mark_line().with_interpolation(StepAfter)` for ECDFs

This mirrors Charton’s philosophy:
> **statistics transform data; marks only draw geometry**

### 7.3.2 Basic Density Plot
A minimal density plot looks like this:
```rust
let chart = Chart::build(&df)?
    .transform_density(
        DensityTransform::new("IMDB_Rating")
    )?
    .mark_line()
    .encode((
        x("value"),
        y("density"),
    ))?;
```
By default:
- The kernel is **Gaussian (Normal)**
- The bandwidth is chosen via **Scott’s rule**
- The output columns are:
    - `"value"` (x-axis)
    - `"density"` (y-axis)

These defaults are chosen to produce reasonable results for most datasets.

### 7.3.3 Cumulative Density (CDF)
Density estimation can also produce **cumulative distributions**.

This is especially useful for:
- Comparing percentiles
- Visualizing probabilities (“what fraction is below x?”)
- ECDF-style plots
```rust
let chart = Chart::build(&df)?
    .transform_density(
        DensityTransform::new("IMDB_Rating")
            .with_as("IMDB_Rating", "cumulative_density")
            .with_cumulative(true)
    )?
    .mark_area()
    .encode((
        x("IMDB_Rating"),
        y("cumulative_density"),
    ))?
    .with_area_opacity(0.3);
```
Internally:
    - `pdf()` is replaced with `cdf()`
    - The output remains continuous and monotonic
    - Any mark capable of drawing a curve can be used

This design makes **PDF and CDF just configuration differences**, not separate chart types.

### 7.3.4 Kernel Functions
Charton supports multiple kernel functions via `KernelType`:
```rust
pub enum KernelType {
    Normal,
    Epanechnikov,
    Uniform,
}
```
You can select the kernel explicitly:
```rust
DensityTransform::new("values")
    .with_kernel(KernelType::Epanechnikov)
```
Conceptually:
- **Normal**: smooth, infinitely supported (default)
- **Epanechnikov**: compact support, optimal MSE
- **Uniform**: box kernel, step-like smoothing

The choice affects smoothness and boundary behavior but not API structure.

### 7.3.5 Bandwidth Selection
Bandwidth controls how smooth the density curve is.

Charton supports three bandwidth strategies:
```rust
pub enum BandwidthType {
    Scott,
    Silverman,
    Fixed(f64),
}
```
Examples:
```rust
DensityTransform::new("values")
    .with_bandwidth(BandwidthType::Silverman)
```
```rust
DensityTransform::new("values")
    .with_bandwidth(BandwidthType::Fixed(0.2))
```
Internally:
- Scott and Silverman rules are delegated to the KDE backend
- Fixed bandwidth is wrapped as a closure
- Bandwidth choice is applied per group

This makes bandwidth **explicit, reproducible, and inspectable**.

### 7.3.6 Grouped Density Estimation
One of the most powerful features of Charton density estimation is **grouped KDE**.
```rust
DensityTransform::new("IMDB_Rating")
    .with_groupby("Genre".to_string())
```
This causes Charton to:
1. Split the data by group
2. Perform KDE **independently for each group**
3. Concatenate the results into a single DataFrame
4. Preserve group labels for encoding

You can then encode color:
```rust
.mark_line()
.encode((
    x("value"),
    y("density"),
    color("Genre"),
))
```
Each group becomes a separate density curve, colored consistently.

### 7.3.7 Counts vs Probability Density
By default, density values integrate to 1 (probability density).

If you want **smoothed counts instead**, enable `counts`:
```rust
DensityTransform::new("values")
    .with_counts(true)
```
Internally:
- Density values are multiplied by group size
- The resulting curve approximates histogram counts
- Useful when comparing absolute frequencies

This allows density plots to function as **continuous histograms**.

### 7.3.8 Evaluation Grid and Numerical Stability
Internally, Charton:
- Computes global min/max across all data
- Expands degenerate ranges automatically
- Evaluates KDE on a fixed grid (default: 200 points)

This ensures:
- Consistent x-ranges across groups
- Stable rendering
- Comparable densities in layered charts

The evaluation grid is intentionally abstracted away from the API.

### 7.3.9 Rendering Density Curves
Once transformed, density data behaves like any other continuous dataset.

Typical rendering choices:

**Line Density**
```rust
.mark_line()
.with_line_stroke_width(2.0)
```
**Filled Density**
```rust
.mark_area()
.with_area_opacity(0.4)
```

### 7.3.10 Density vs Histogram vs Box Plot

| **Chart Type** | **Purpose**              | **Strength**       |
| ------------- | ------------------------- | ------------------ |
| Histogram     | Frequency approximation   | Simple, intuitive  |
| Density (KDE) | Smooth distribution shape | Reveals structure  |
| Box Plot      | Robust summary            | Compact comparison |
| ECDF          | Cumulative probability    | Exact percentiles  |

Charton intentionally supports **all four**, each optimized for a different analytical task.

**Design Philosophy Recap**

Density estimation in Charton is:
- **Explicit**: no hidden magic
- **Composable**: works with any continuous mark
- **Grouped by design**: first-class support
- **Statistically honest**: kernel and bandwidth are visible choices

Rather than introducing a dedicated `density` mark, Charton treats KDE as what it truly is:

> **a statistical transformation, not a geometric primitive**

## 7.4 Empirical Cumulative Distribution Function (ECDF)
The **Empirical Cumulative Distribution Function (ECDF)** describes how data accumulates over its range.

For a value 𝑥, the ECDF gives:

> the number (or proportion) of observations less than or equal to 𝑥

Unlike density plots, ECDFs:
- Do **not require bandwidth selection**
- Are **exact** representations of the data
- Preserve all distributional information
- Are especially useful for percentile-based comparisons

In Charton, ECDF is implemented as a **window transformation**, not as a dedicated mark.

This design reflects a core principle:

> ECDF is a *cumulative statistic*, not a geometric primitive.

### 7.4.1 ECDF as a Window Transformation
Charton computes ECDF using the **window transform system**, specifically the `CumeDist` operation.
```rust
WindowOnlyOp::CumeDist
```
Conceptually, this corresponds to SQL-style cumulative distribution:
```sql
CUME_DIST() OVER (PARTITION BY group ORDER BY value)
```
The result is a new column that increases monotonically within each group.

### 7.4.2 Minimal ECDF Example
A basic ECDF chart consists of three parts:
1. A window transformation
2. A line mark
3. Step interpolation
```rust
let chart = Chart::build(&df)?
    .transform_window(
        WindowTransform::new(
            WindowFieldDef::new("value", WindowOnlyOp::CumeDist, "ecdf")
        )
    )?
    .mark_line()
        .with_interpolation(PathInterpolation::StepAfter)
    .encode((
        x("value"),
        y("ecdf"),
    ))?;
```
Here:
- `"value"` is the sorted variable
- `"ecdf"` is the cumulative frequency
- `StepAfter` ensures a mathematically correct ECDF shape

### 7.4.3 Why Step Interpolation Is Required
An ECDF is a **right-continuous step function**.

Charton enforces this visually by requiring:
```rust
.with_interpolation(PathInterpolation::StepAfter)
```
This interpolation means:
- The function holds its value *after* each observation
- Jumps occur exactly at data points
- No artificial smoothing is introduced

Internally, this choice also triggers ECDF-specific rendering logic, described later.

### 7.4.4 Grouped ECDFs
ECDFs are often compared across categories.

Charton supports grouped ECDF computation via `with_groupby`:
```rust
.transform_window(
    WindowTransform::new(
        WindowFieldDef::new("sepal_length", WindowOnlyOp::CumeDist, "ecdf")
    )
    .with_groupby("species")
)
```
This causes Charton to:
**1.** Partition data by group
**2.** Sort values within each group
**3.** Compute cumulative distributions independently
**4.** Preserve group identity for encoding

You can then encode color:
```rust
.encode((
    x("sepal_length"),
    y("ecdf"),
    color("species"),
))
```
Each group becomes a separate ECDF curve.

### 7.4.5 Normalized vs Raw ECDF
By default, `CumeDist` returns **raw cumulative counts**:
- The maximum value equals the group size

You can normalize ECDFs to the [0, 1] range:
```rust
.with_normalize(true)
```
This transforms ECDFs into **cumulative probability functions**, making them directly comparable across groups with different sample sizes.

Internally, normalization is applied as:
```rust
cumulative / total_count
```
This logic is implemented entirely within the lazy Polars pipeline.

### 7.4.6 WindowTransform API Overview
The ECDF behavior is controlled by `WindowTransform`:
```rust
pub struct WindowTransform {
    pub window: WindowFieldDef,
    pub frame: [Option<f64>; 2],
    pub groupby: Option<String>,
    pub ignore_peers: bool,
    pub normalize: bool,
}
```
For ECDF:
- `window.op = CumeDist`
- `frame = [None, Some(0.0)]` (unbounded preceding → current row)
- `ignore_peers = false` (ties handled correctly)
- `normalize` is optional

The frame definition ensures cumulative behavior rather than sliding windows.

### 7.4.7 How ECDF Is Computed Internally
When `CumeDist` is selected, Charton performs the following steps:

**1. Create a working DataFrame**
Ensures a group column exists (real or temporary).

**2. Compute rank-based cumulative frequency**

Uses Polars’ `rank()` with:
* `RankMethod::Max`
* Ascending order

**3. Preserve group appearance order**

Groups are rendered in the order they appear in the data.

**4. Deduplicate ECDF steps**

Only unique `(group, cumulative_value)` pairs are kept.

**5. Optionally normalize**

Divides by total group size if requested.

**6. Clean up temporary columns**

The result is a minimal, clean dataset suitable for plotting.

### 7.4.8 ECDF Rendering Details

ECDF rendering is handled by the line mark renderer with ECDF-aware logic.

**Global X Range Alignment**

Before rendering, Charton computes:
```rust
global_x_min
global_x_max
```
These values ensure that **all ECDF curves share the same horizontal extent**, which is critical for correct comparison.

**Automatic Start and End Points**

For `StepAfter` interpolation, Charton automatically:
- Prepends a starting point `(global_x_min, 0)`
- Appends an ending point `(global_x_max, max_y)`

This guarantees:
- ECDF starts at zero
- ECDF extends horizontally to the full data range
- Visual completeness even with sparse data

This behavior is ECDF-specific and does **not** apply to ordinary line charts.

### 7.4.9 Axis Swapping Support

ECDF rendering fully respects coordinate transformations.

When axes are swapped:
```rust
(x, y) → (y, x)
```
This allows ECDFs to be rendered horizontally without any change to statistical logic.

### 7.4.10 Complete ECDF Example
```rust
let chart = Chart::build(&df.select(["species", "sepal_length"])?)?
    .transform_window(
        WindowTransform::new(
            WindowFieldDef::new(
                "sepal_length",
                WindowOnlyOp::CumeDist,
                "ecdf"
            )
        )
        .with_groupby("species")
        .with_normalize(false)
    )?
    .mark_line()
        .with_interpolation(PathInterpolation::StepAfter)
    .encode((
        x("sepal_length"),
        y("ecdf"),
        color("species")
    ))?;
```
This produces a grouped ECDF with exact cumulative counts.

### 7.4.11 ECDF vs Density vs Histogram

| **Method**    | **Exact**   | **Smooth** | **Requires Parameters** | **Best Use**   |
| ------------- | ----------- | ------ | ------------------- | ------------------------ |
| Histogram     | Approximate | No     | Bin size            | Frequency overview       |
| Density (KDE) | Approximate | Yes    | Kernel, bandwidth   | Shape analysis           |
| ECDF          | Exact       | No     | None                | Percentiles, comparisons |

Charton intentionally supports all three, recognizing that **no single method fits all analytical goals**

**Design Philosophy Recap**

ECDF in Charton is:
**- Statistically exact**
**- Explicitly computed**
**- Composable with marks**
**- Faithful to mathematical definition**

Rather than introducing a special ECDF mark, Charton models ECDF as what it truly is:
> a cumulative window statistic rendered with step geometry..

## 7.5 Histograms (1D & Grouped)

Histograms are one of the most fundamental tools for understanding data distributions.

They approximate an underlying distribution by:

**1. Partitioning continuous values into bins**
**2. Counting observations within each bin**
**3. Rendering counts (or normalized frequencies) as adjacent bars**

Unlike bar charts, histogram bars represent **continuous intervals**, not discrete categories.
This distinction drives both the API design and the internal implementation in Charton.

### 7.5.1 Histogram as a Data Transformation
In Charton, a histogram is **not a primitive mark alone**.

It is defined by the combination of:
- a **binning transform** on the x-axis
- an **aggregation** (count)
- an optional **normalization**
- a **rectangular mark renderer**

This separation allows histograms to integrate naturally with:
- color grouping
- layered charts
- axis swapping
- shared legends

### 7.5.2 Basic 1D Histogram Example
```rust
let chart = Chart::build(&df)?
    .mark_hist()
    .encode((
        x("value"),
        y("count"),
    ))?;
```
At the API level:
- `x("value")` indicates the continuous variable to be binned
- `y("count")` signals an aggregation rather than raw data
- `mark_hist()` selects the histogram renderer

No explicit binning step is required from the user.

### 7.5.3 Normalized Histograms
Histograms are often normalized to show **relative frequencies** instead of raw counts.

Charton exposes normalization through the y encoding:
```rust
y("count").with_normalize(true)
```
This mirrors the mental model used in density plots and ECDFs:

> normalization is a property of aggregation, not geometry

Internally, normalization is applied *after grouping* but *before rendering*.

### 7.5.4 Grouped Histograms via Color Encoding
Histograms can be grouped using color encoding:
```rust
.encode((
    x("value"),
    y("count").with_normalize(true),
    color("variable"),
))
```
This produces a **grouped histogram**, where each group:
- shares the same bin boundaries
- is normalized independently
- is rendered with a distinct palette color

Charton ensures that **empty bins are preserved** across all groups, avoiding misleading shapes.

### 7.5.5 Histogram Data Transformation Pipeline
Histogram computation is implemented in:
```rust
Chart<T>::transform_histogram_data
```
This method is invoked automatically when `mark_hist()` is active.

The pipeline consists of six stages.

### 7.5.5.1 Determining the Number of Bins
Charton determines the number of bins as follows:
```rust
let n_bins = x_encoding.bins.unwrap_or_else(|| {
    ((unique_count as f64).sqrt() as usize).max(5).min(50)
});
```
Rules:
- If all values are identical → 1 bin
- Otherwise:
    - default: √n rule
    - minimum: 5 bins
    - maximum: 50 bins
- Users may override this explicitly via encoding

This strikes a balance between robustness and simplicity.

#### 7.5.5.2 Computing Bin Boundaries
Once `n_bins` is known:
```rust
let bin_width = (max_val - min_val) / (n_bins as f64);
```
Charton constructs:
- bin edges
- bin labels (`bin_0`, `bin_1`, …)
- bin midpoints (used later for rendering)

The midpoints ensure histogram bars align naturally with continuous axes.

#### 7.5.5.3 Assigning Observations to Bins
Binning is performed using a dedicated statistical utility:
```rust
crate::stats::stat_binning::cut
```
This produces a categorical bin label for each observation, which is then:
- added as a temporary column
- used as a grouping key

This design keeps binning logic reusable and testable.

#### 7.5.5.4 Grouping and Counting
Aggregation depends on whether color encoding is present.

**Without color encoding:**
```rust
group_by_stable([bin_field])
```
**With color encoding:**
```rust
group_by_stable([bin_field, color_field])
```
In both cases, counts are computed using:
```rust
col(&bin_field).count()
```
Stable grouping preserves the visual order of categories.

#### 7.5.5.5 Filling Empty Bins
A critical detail: **empty bins must be rendered explicitly**.

Charton guarantees this by:
- generating all possible bin labels
- creating all bin × color combinations (if grouped)
- left-joining aggregated results
- filling missing counts with zero

This avoids gaps and misleading shapes in grouped histograms.

#### 7.5.5.6 Normalization
If normalization is enabled:
- **without color**: all bins sum to 1
- **with color**: each color group sums to 1 independently

This mirrors the behavior of statistical plotting systems such as Vega-Lite and ggplot2.

#### 7.5.5.7 From Transformed Data to Geometry
After transformation, the histogram renderer takes over.

Rendering is implemented in:
```rust
Chart<MarkHist>::render_histogram
```
#### 7.5.5.8 Bar Width Calculation
Histogram bins are equally spaced, so bar width is inferred from data:
```rust
bar_width = (max - min) / (n_bins - 1) * 0.95
```
This:
- prevents bars from touching exactly
- preserves the continuous nature of the axis
- adapts automatically to axis scaling

A fallback width is used if spacing cannot be inferred safely.

#### 7.5.5.9 Color Handling Strategy
Color is resolved per group:
- If `color` encoding exists:
    - colors are assigned from the active palette
- Otherwise:
    - `MarkHist.color` is used as a uniform fill

This matches the behavior of other marks and keeps styling orthogonal to data semantics.

#### 7.5.5.10
Vertical and Horizontal Histograms

Histogram rendering respects axis swapping automatically.
- **Vertical histogram**
    - x → bin midpoint
    - y → count

- **Horizontal histogram**
    - y → bin midpoint
    - x → count

The same transformed data supports both layouts without recomputation.

#### 7.5.5.11 MarkHist: Visual Properties
The `MarkHist` struct controls appearance only:
```rust
pub struct MarkHist {
    color: Option<SingleColor>,
    opacity: f64,
    stroke: Option<SingleColor>,
    stroke_width: f64,
}
```
This strict separation ensures:
- no statistical logic leaks into rendering
- consistent behavior across themes and backends

#### 7.5.5.12 Layering and Legends
Histograms integrate naturally with layered charts:
```rust
LayeredChart::new()
    .add_layer(histogram_chart)
    .with_legend(true)
```
Legend rendering is delegated to the shared color legend renderer, ensuring consistency across all mark types.

### 7.5.6 A Complete Histogram Example
The following example demonstrates a complete workflow for creating a grouped, normalized histogram in Charton:
- reading real-world data
- reshaping it for visualization
- binning continuous values
- grouping by category
- rendering a layered histogram with a legend

```rust
use charton::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = load_dataset("iris")?;

    let df_melted = df.unpivot(
        ["sepal_length", "sepal_width", "petal_length", "petal_width"],
        ["species"]
    )?;

    let histogram_chart = Chart::build(&df_melted.head(Some(200)))?
        .mark_hist()
        .encode((
            x("value"),
            y("count").with_normalize(true),
            color("variable")
        ))?
        .with_hist_color(Some(SingleColor::new("steelblue")))
        .with_hist_opacity(0.5)
        .with_hist_stroke(Some(SingleColor::new("black")))
        .with_hist_stroke_width(0.0)
        .with_color_palette(ColorPalette::Tab10);

    LayeredChart::new()
        .with_size(500, 400)
        .with_title("Histogram Example")
        .with_x_label("Value")
        .with_y_label("Frequency")
        .add_layer(histogram_chart)
        .with_legend(true)
        .save("histogram.svg")?;

    Ok(())
}
```

### 7.5.7 Design Philosophy Recap
Charton’s histogram implementation emphasizes:
- **Explicit statistical transformation**
- **Stable grouping and binning**
- **Correct handling of empty bins**
- **Clear separation of data, geometry, and style**

Rather than treating histograms as “fat bar charts,” Charton models them as what they are:
> a structured approximation of continuous distributions.

## 7.6 Pie & donut charts
Pie and donut charts represent **categorical proportions** using angular spans. Each category is mapped to a slice whose angle is proportional to its value.

In Charton, pie and donut charts are implemented using the `arc` **mark** together with **theta encoding**, rather than Cartesian x/y axes.

### 7.6.1 A Complete Pie / Donut Chart Example
The following example demonstrates how to create a **donut chart** using Charton. It covers:
- categorical aggregation
- angular (theta) encoding
- color-based grouping
- converting a pie chart into a donut chart
```rust
use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data frame
    let df = df![
        "category" => ["A", "B", "C", "D", "E"],
        "value" => [25.0, 30.0, 15.0, 20.0, 10.0]
    ]?;

    // Create a pie / donut chart
    let pie_chart = Chart::build(&df)?
        .mark_arc()
        .encode((
            theta("value"),      // angular size of each slice
            color("category"),   // category for each slice
        ))?
        .with_inner_radius_ratio(0.5); // turn pie into donut

    // Render and save
    LayeredChart::new()
        .with_size(400, 400)
        .with_title("Donut Chart Example")
        .add_layer(pie_chart)
        .with_legend(true)
        .save("./examples/donut.svg")?;

    Ok(())
}
```
This example produces a donut chart where each category occupies a slice whose angle is proportional to its value.

The remainder of this section explains how this behavior is implemented.

### 7.6.2 Arc Marks and Polar Geometry
Pie and donut charts in Charton are rendered using the `MarkArc` mark type.

Unlike Cartesian marks (points, bars, lines), arc marks:
- do **not** use x/y scales
- are rendered in **polar coordinates**
- rely on angular accumulation around a center point

Internally, the chart computes:
- a fixed center position
- a shared outer radius
- a cumulative start and end angle for each slice

```rust
let slice_angle = 2.0 * PI * value / total;
```
All slices together sum to a full circle.

### 7.6.3 Theta Encoding
The `theta()` encoding maps a numeric field to **angular magnitude**.
```rust
.encode((
    theta("value"),
    color("category"),
))
```
Key properties:
- theta values **must be non-negative**
- values are summed by their category label and automatically normalized by the total sum of all values
- the order of slices follows the **original data order**, not sorted order

Internally, Charton aggregates data by category and sums the theta field before rendering.

### 7.6.4 Color Encoding and Legends
Pie and donut charts almost always rely on color to distinguish categories.

When `color()` encoding is present:
- each slice is assigned a color from the active palette
- legends are rendered automatically
- slice order matches the first appearance of each category
```rust
color("category")
```
If no color encoding is provided, the mark’s default color is used.

### 7.6.5 Donut Charts via Inner Radius Ratio
Donut charts are created by specifying an inner radius ratio:
```rust
.with_inner_radius_ratio(0.5)
```
- `0.0` → full pie chart
- `(0.0, 1.0)` → donut chart
- `1.0` → fully hollow (not useful)

The inner radius is computed as:
```rust
inner_radius = outer_radius × inner_radius_ratio
```
This allows smooth transitions between pie and donut representations without changing data or encodings.

### 7.6.6 Rendering Model
Each slice is rendered as an SVG path representing an annular sector:
- start angle
- end angle
- outer radius
- optional inner radius

Stroke and opacity are applied uniformly across slices:
```rust
.with_arc_opacity(0.9)
.with_arc_stroke(SingleColor::new("white"))
.with_arc_stroke_width(1.0)
```

### 7.6.7 Design Considerations
Pie and donut charts are best suited for:
- small numbers of categories
- part-to-whole comparisons

emphasizing proportions rather than precise values.

For larger category counts or precise comparisons, bar charts or stacked charts are often more effective.

Charton intentionally keeps pie/donut charts:
- simple
- declarative
- aggregation-driven

to encourage correct and intentional usage.

## 7.7 Heatmaps & rect-based charts
**Grid-based scalar maps and intensity fields**

Heatmaps and other rectangle-based charts visualize scalar values over a 2D grid.

In Charton, this family of charts is implemented using the `rect` **mark**, where each data point is rendered as a colored rectangle whose position is determined by *(x, y)* encodings and whose color represents a numeric intensity.

Typical use cases include:
- Categorical × categorical heatmaps
- Continuous 2D density / binned heatmaps
- Correlation matrices
- Time × category intensity maps

### 7.7.1 API Overview
Rect-based charts are enabled by calling `mark_rect()` on a `Chart`.
```rust
Chart::build(&df)?
    .mark_rect()
    .encode((
        x("x"),
        y("y"),
        color("value"),
    ))?;
```
**Required encodings**

| **Encoding** | **Type**                   | **Description**                    |
| -------- | ---------------------- | ------------------------------ |
| `x`      | discrete or continuous | Horizontal grid coordinate     |
| `y`      | discrete or continuous | Vertical grid coordinate       |
| `color`  | numeric                | Scalar value mapped to a color |

> **Note**
> Unlike point or bar charts, `rect` charts **require** a `color` encoding. The rectangle fill color is the primary visual channel.

### 7.7.2 Rect Mark Configuration
The `rect` mark controls the appearance of each grid cell.
```rust
pub struct MarkRect {
    color: Option<SingleColor>,
    opacity: f64,
    stroke: Option<SingleColor>,
    stroke_width: f64,
}
```
**Configuration methods**
```rust
.mark_rect()
.with_rect_color(Some(SingleColor::new("black")))
.with_rect_opacity(0.8)
.with_rect_stroke(Some(SingleColor::new("white")))
.with_rect_stroke_width(1.0)
```
| **Method**               | **Effect**                                           |
| ------------------------ | ---------------------------------------------------- |
| `with_rect_color`        | Fallback fill color (used only if no color encoding) |
| `with_rect_opacity`      | Rectangle transparency                               |
| `with_rect_stroke`       | Border color                                         |
| `with_rect_stroke_width` | Border thickness in pixels                           |

In typical heatmap usage, **the fill color comes from the colormap**, not from `with_rect_color`.

### 7.7.3 Data Processing Pipeline (Source-level Explanation)
Rect-based charts have the most complex data pipeline in Charton because they support **both discrete and continuous axes**.

This logic lives in:
```rust
Chart<T>::transform_rect_data()
```
**Step 1: Detect axis scale types**
```rust
let x_is_discrete = matches!(x_scale, Scale::Discrete);
let y_is_discrete = matches!(y_scale, Scale::Discrete);
```
- Discrete × discrete → categorical heatmap
- Continuous × continuous → 2D binned heatmap
- Mixed → hybrid binning

**Step 2: Automatic binning (continuous axes)**

If an axis is continuous:
- Values are **binned**
- Bin count is determined by:
    - Explicit `bins` setting, or
    - √N heuristic (bounded to 5–50)
- Bin labels are generated (`bin_0`, `bin_1`, …)
- Bin **midpoints** are stored for rendering
```rust
cut(&series, &bins, &labels)
```
This ensures **rectangles occupy equal spatial extents**, which is essential for heatmaps.

**Step 3: Aggregation**

After binning (if needed), data is aggregated by `(x, y)`:
```rust
.group_by_stable([x, y])
.agg([color.sum()])
```
If multiple records fall into the same grid cell, their values are **summed**.

**Step 4: Fill missing grid cells**

For binned data, Charton generates **all possible x × y combinations** and fills missing cells with `0`.

This guarantees:
- Rectangles form a complete grid
- No visual gaps appear due to missing data

**Step 5: Replace bin labels with numeric midpoints**

For continuous axes:
- Bin labels → numeric midpoints
- Enables numeric coordinate mapping during rendering

### 7.7.4 Rendering Logic
Rendering is handled by `render_rects()`.

**Rectangle size computation**
```rust
rect_width  = Δx_pixels / (unique_x_count - 1)
rect_height = Δy_pixels / (unique_y_count - 1)
```
This ensures:

- Uniform rectangle sizes
- Correct alignment in both discrete and continuous modes

**Color mapping**
```rust
let color = self.mark_cmap.get_color(normalized_value);
```
- Values are normalized in `ProcessedChartData`
- Colors are drawn from the active `ColorMap`
- A continuous colorbar legend is automatically rendered

### 7.7.5 Legend Behavior
Rect charts always use a **continuous color legend**:
```rust
render_colorbar(svg, self, theme, context)
```
This produces a vertical color scale showing:
- Min → max value mapping
- Associated numeric ticks

### 7.7.6 Complete Examples
**Example 1: Categorical Heatmap**
```rust
use charton::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! [
        "x" => ["A", "B", "C", "A", "B", "C", "A", "B", "C"],
        "y" => ["X", "X", "X", "Y", "Y", "Y", "Z", "Z", "Z"],
        "value" => [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
    ]?;

    let heatmap = Chart::build(&df)?
        .mark_rect()
        .encode((
            x("x"),
            y("y"),
            color("value"),
        ))?;

    LayeredChart::new()
        .add_layer(heatmap)
        .save("heatmap.svg")?;

    Ok(())
}
```

**Characteristics**
- Discrete × discrete grid
- One rectangle per category pair
- Continuous color legend

**Example 2: Continuous 2D Density / Binned Heatmap**
```rust
use charton::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! [
        "x" => [1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.8, 2.05, 2.2, 2.5, 2.6, 2.7],
        "y" => [1.2, 1.3, 1.4, 1.5, 1.8, 1.83, 2.0, 1.9, 2.2, 2.3, 2.4, 2.5],
        "value" => [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
    ]?;

    let density = Chart::build(&df)?
        .mark_rect()
        .encode((
            x("x"),
            y("y"),
            color("value"),
        ))?
        .with_color_map(ColorMap::GnBu);

    LayeredChart::new()
        .add_layer(density)
        .save("2d_density.svg")?;

    Ok(())
}
```

**Characteristics**
- Continuous × continuous
- Automatic binning
- Grid filled with aggregated values
- Smooth perceptual color mapping

### 7.7.7 Design Notes
- Rect charts are **data-dense** and scale well to large datasets
- Automatic binning keeps the API minimal while remaining flexible
- Separation of:
    - **Data transformation** (`transform_rect_data`)
    - **Rendering** (`render_rects`) ensures extensibility (e.g. treemaps, calendar heatmaps)


## 7.8 Rule-based Charts
**Thresholds, reference markers, and annotation overlays**

Rule-based charts are used to draw **reference lines** rather than data glyphs. They are typically overlaid on other charts to indicate:
- Thresholds (e.g. mean, target, warning line)
- Ranges (from `y` to `y2`)
- Event markers or annotations
- Baselines and guides

In Charton, rule charts are implemented via the `rule` **mark**, which renders horizontal or vertical lines aligned to data coordinates.

Unlike bars or points, rule marks do not encode magnitude through area or position alone — instead, they **annotate the coordinate space itself**.

### 7.8.1 API Overview
A rule chart is created by calling `mark_rule()` on a `Chart`.
```rust
Chart::build(&df)?
    .mark_rule()
    .encode((
        x("x"),
        y("y"),
    ))?;
```
**Supported encodings**

| **Encoding** | **Required** | **Description**                       |
| -------- | -------- | ----------------------------------------------- |
| `x`      | yes      | Position of the rule along the x-axis           |
| `y`      | yes      | Starting position along the y-axis              |
| `y2`     | no       | Optional end position (defines a segment/range) |
| `color`  | no       | Category or value-based coloring                |

### 7.8.2 MarkRule Configuration
The visual appearance of rule lines is controlled by `MarkRule`.
```rust
pub struct MarkRule {
    color: Option<SingleColor>,
    opacity: f64,
    stroke_width: f64,
}
```
**Configuration methods**
```rust
.mark_rule()
.with_rule_color(Some(SingleColor::new("red")))
.with_rule_opacity(0.6)
.with_rule_stroke_width(2.0)
```
| **Method**               | **Effect**                                            |
| ------------------------ | ----------------------------------------------------- |
| `with_rule_color`        | Default stroke color (when no color encoding is used) |
| `with_rule_opacity`      | Line transparency                                     |
| `with_rule_stroke_width` | Line thickness in pixels                              |

If a `color` encoding is present, colors are automatically assigned using:
- **Palette** for discrete scales
- **Colormap** for continuous scales

### 7.8.3 Conceptual Model
Rule charts support **two conceptual forms**:

**1. Infinite reference rules**

If `y2` is **not** provided:
- Each rule spans the entire plotting region
- Used for baselines or global thresholds
```rust
x = constant
y = plot_min → plot_max
```
**2. Finite range rules**

If `y2` **is** provided:
- Each rule spans from `y` to `y2`
- Used for intervals, confidence bands, or annotations
```rust
x = constant
y → y2
```
This dual behavior is handled automatically at render time.

### 7.8.4 Data Processing Pipeline
Rule charts reuse the shared processing logic via:
```rust
ProcessedChartData::new(self, coord_system)
```
This provides:
- Transformed x / y values (linear, log, discrete)
- Optional color normalization

**Handling** `y2`

If a `y2` encoding exists:
```rust
let y2_series = self.data.column(&y2_encoding.field)?;
```
- Values are transformed according to the y-axis scale
- Log scales apply `log10`
- Linear and discrete scales pass through unchanged

This ensures **consistent geometry across coordinate systems**.

### 7.8.5 Rendering Logic (Source-Level Explanation)
Rendering is implemented in `render_rules()`.

**Step 1: Resolve stroke color**
```rust
if let Some(color_info) = processed_data.color_info {
    // palette or colormap
} else {
    mark.color.clone()
}
```
| **Color scale** | **Behavior**          |
| ----------- | ------------------------- |
| Discrete    | Uses categorical palette  |
| Continuous  | Uses colormap             |
| None        | Uses mark’s default color |

**Step 2: Map data → pixel space**
```rust
let x_pos = (context.x_mapper)(x_vals[i]);
let y_pos = (context.y_mapper)(y_vals[i]);
```
Axis swapping is handled transparently via `context.swapped_axes`.

**Step 3: Render rules**
**Without** `y2`
```rust
render_vertical_rule(
    x_pos,
    plot_top,
    plot_bottom,
)
```
Draws a full-height rule.

**With** `y2`
```rust
render_vertical_rule(
    x_pos,
    y_pos,
    y2_pos,
)
```
Draws a bounded segment.

If axes are swapped, horizontal rules are drawn instead.

### 7.8.6 Legend Behavior
Rule charts support **both legend types**:
- **Discrete color legend** (categorical rules)
- **Continuous colorbar** (value-driven rules)

Legend rendering is delegated to:
```rust
colorbar_renderer::render_colorbar(...)
color_legend_renderer::render_color_legend(...)
```
Only the appropriate legend is displayed, depending on the encoding scale.

### 7.8.7 Complete Example: Rule Chart with Y and Y2
The following example demonstrates:

- Vertical rules positioned at x
- Finite rule segments using y → y2
- Discrete color encoding
- Full chart metadata (title, labels)
```rust
use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data with x, y, y2, and color columns
    let df = df![
        "x" => [1.0, 2.0, 3.0, 4.0, 5.0],
        "y" => [2.0, 3.0, 1.0, 4.0, 2.0],
        "y2" => [4.0, 5.0, 3.0, 6.0, 4.0],
        "color" => ["A", "B", "A", "B", "A"]
    ]?;

    // Create rule chart
    let chart = Chart::build(&df)?
        .mark_rule()
        .encode((
            x("x"),
            y("y"),
            y2("y2"),
            color("color"),
        ))?
        .into_layered()
        .with_title("Rule Chart with Y and Y2")
        .with_x_label("X Values")
        .with_y_label("Y Values");

    // Save to SVG
    chart.save("rule.svg")?;

    Ok(())
}
```
**Resulting Visualization**
- Each rule is a vertical line at a given `x`
- The visible segment spans from `y` to `y2`
- Colors distinguish categories
- Rules can be layered on top of other marks

### 7.8.8 Design Notes
- Rule marks are **annotation-first**, not data-first
- They are designed to be layered with bars, lines, or heatmaps
- Explicit support for `y2` avoids special-case “range” marks
- Shared rendering infrastructure keeps rule logic minimal and composable

# Chapter 8 · External Backend Integration
Charton can render charts using native Rust rendering, but it also integrates seamlessly with external visualization backends such as Altair  and Matplotlib.  

This chapter explains how backend switching works and why external backends can be useful for leveraging established visualization ecosystems. You will also learn how to run raw Python plotting code from Rust, allowing complete flexibility.

This is especially useful when mixing Rust data pipelines with existing Python workflows.

## 8.1 Why external backends?
Rust visualization ecosystem — including Charton — is still relatively young, it may not always meet all user requirements. In contrast, other languages have mature and feature-rich visualization tools, such as Altair and Matplotlib. Therefore, in situations where Charton’s native capabilities are not sufficient, it is necessary to rely on these external visualization tools as complementary backends.

## 8.2 Altair backend
Charton provides first-class integration with the Altair visualization ecosystem through the Altair backend. This backend allows Rust programs to generate Altair charts, render them using Python, and output either SVG images or Vega-Lite JSON specifications. This enables seamless interoperability between Rust data pipelines and any existing Python-based visualization workflow.

Internally, Charton sends data to Python using an IPC (Apache Arrow) buffer, executes user-provided Altair code, and returns either SVG or Vega-Lite JSON back to Rust.

**🔧 Requirements**

Before using the Altair backend, ensure Python and required packages are installed:
```shell
pip install altair vl-convert-python polars pyarrow
```
### 8.2.1 Loading the Example Dataset (`mtcars`)
Below is a built-in function to load `mtcars` into a Polars `DataFrame`:
```rust
let df = load_dataset("mtcars")?;
```

### 8.2.2 Basic Usage: Executing Altair Code
This example shows the minimal usage of the Altair backend:
- Load `mtcars`
- Send it to the Altair backend
- Execute a small Altair script
- Display result (in Jupyter) or do nothing (CLI)

```rust
use charton::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = load_dataset("mtcars")?;

    let code = r#"
import altair as alt
chart = alt.Chart(df).mark_point().encode(
    x='mpg',
    y='hp',
    color='cyl:O'
)
"#;

    Plot::<Altair>::build(data!(&df)?)?
        .with_exe_path("python")?
        .with_plotting_code(code)
        .show()?;   // Works in evcxr notebook

    Ok(())
}
```
### 8.2.3 Saving Altair Charts as SVG
The Altair backend supports exporting the chart as **SVG** by calling `.save("chart.svg")`.
```rust
use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = load_mtcars();

    let code = r#"
import altair as alt
chart = alt.Chart(df).mark_circle(size=80).encode(
    x='wt',
    y='mpg',
    color='cyl:O',
    tooltip=['mpg','cyl','wt']
)
"#;

    Plot::<Altair>::build(data!(&df)?)?
        .with_exe_path("python")?
        .with_plotting_code(code)
        .save("scatter.svg")?;

    println!("Saved to scatter.svg");
    Ok(())
}
```
### 8.2.4 Export as Vega-Lite JSON
To get a **Vega-Lite JSON specification**, call `.to_json()` or save with `.json` extension:

**Method 1 — get JSON string in Rust:**
```rust
let json: String = Plot::<Altair>::build(data!(&df)?)?
    .with_exe_path("python")?
    .with_plotting_code(code)
    .to_json()?;

println!("{}", json);
```
**Method 2 — save JSON file:**
```rust
Plot::<Altair>::build(data!(&df)?)?
    .with_exe_path("python")?
    .with_plotting_code(code)
    .save("chart.json")?;
```
### 8.2.5 Example: Converting to Vega-Lite JSON and Rendering in the Browser
You can embed the exported JSON into an HTML file and render it directly in the browser using Vega-Lite.

**Step 1 — generate JSON from Rust:**
```rust
Plot::<Altair>::build(data!(&df)?)?
    .with_exe_path("python")?
    .with_plotting_code(code)
    .save("mtcars.json")?;
```

**Step 2 — embed JSON in HTML:**
```html
<!DOCTYPE html>
<html>
<head>
  <script src="https://cdn.jsdelivr.net/npm/vega@5"></script>
  <script src="https://cdn.jsdelivr.net/npm/vega-lite@5"></script>
  <script src="https://cdn.jsdelivr.net/npm/vega-embed@6"></script>
</head>

<body>
<div id="vis"></div>

<script>
fetch("mtcars.json")
  .then(r => r.json())
  .then(spec => vegaEmbed("#vis", spec));
</script>

</body>
</html>
```
Open in browser → you get an Altair-rendered visualization displayed via Vega-Lite.

### 8.2.6 Full Example: A More Complete Altair Chart
```rust
let df = load_dataset("mtcars")?;

let code = r#"
import altair as alt

chart = alt.Chart(df).mark_point(filled=True).encode(
    x=alt.X('hp', title='Horsepower'),
    y=alt.Y('mpg', title='Miles/Gallon'),
    color=alt.Color('cyl:O', title='Cylinders'),
    size='wt',
    tooltip=['mpg','hp','wt','cyl']
)
"#;

Plot::<Altair>::build(data!(&df)?)?
    .with_exe_path("python")?
    .with_plotting_code(code)
    .save("full.svg")?;
```
## 8.3 Matplotlib backend
The Matplotlib backend enables Charton to generate high-quality static visualizations using Python’s Matplotlib library. This backend is ideal when users need:
- Scientific publication–grade plots
- Fine-grained control over rendering
- Access to the mature, feature-rich Matplotlib ecosystem
- Compatibility with existing Python visualization workflows

Just like the Altair backend, Charton transfers data to Python through an IPC buffer (Apache Arrow), executes user-provided Matplotlib code, and returns the resulting SVG image back to Rust.

**🔧 Requirements**

Before using the Matplotlib backend, ensure the required Python packages are installed:
```bash
pip install matplotlib polars pyarrow
```
### 8.3.1 Basic Usage: Executing Matplotlib Code
The minimal workflow for the Matplotlib backend is similar to Altair:
**1.** Load data
**2.** Provide a snippet of Python Matplotlib code
**3.** Charton runs it in Python and captures the SVG output

```rust
use charton::prelude::*;
use polars::prelude::*;

let df = load_dataset("mtcars")?;

let code = r#"
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4))
ax.scatter(df['mpg'], df['hp'], c=df['cyl'], cmap='viridis')
ax.set_xlabel('MPG')
ax.set_ylabel('Horsepower')
"#;

Plot::<Matplotlib>::build(data!(&df))?
    .with_exe_path("python")?
    .with_plotting_code(code)
    .show()?;   // Works in evcxr notebook
```
### 8.3.2 Saving Matplotlib Output as SVG
You can export Matplotlib-rendered figures to SVG files using `.save("file.svg")`.

```rust
use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = load_dataset("mtcars")?;

    let code = r#"
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4))
scatter = ax.scatter(df['wt'], df['mpg'], c=df['cyl'], cmap='tab10')
ax.set_xlabel('Weight')
ax.set_ylabel('MPG')
"#;

    Plot::<Matplotlib>::build(data!(&df))?
        .with_exe_path("python")?
        .with_plotting_code(code)
        .save("mat_mtcars.svg")?;

    println!("Saved to mat_mtcars.svg");
    Ok(())
}
```
The saved SVG can be embedded in Markdown, LaTeX, HTML, Jupyter notebooks, or included in publication figures.

### 8.3.3 Using Subplots
Matplotlib excels at multi-panel scientific figures. Here is an example showing two subplots:

```rust
let code = r#"
import matplotlib.pyplot as plt

fig, axes = plt.subplots(1, 2, figsize=(8, 4))

axes[0].hist(df['mpg'], bins=8, color='steelblue')
axes[0].set_title('MPG Distribution')

axes[1].scatter(df['hp'], df['wt'], c=df['cyl'], cmap='viridis')
axes[1].set_xlabel('Horsepower')
axes[1].set_ylabel('Weight')

fig.tight_layout()
"#;

Plot::<Matplotlib>::build(data!(&df)?)?
    .with_exe_path("python")?
    .with_plotting_code(code)
    .save("mat_subplots.svg")?;
```
### 8.3.4 Adding Titles, Legends, and Styles
Matplotlib’s flexibility allows full customization:

```rust
let code = r#"
import matplotlib.pyplot as plt
plt.style.use('ggplot')

fig, ax = plt.subplots(figsize=(6, 4))

scatter = ax.scatter(
    df['hp'], df['mpg'],
    c=df['cyl'],
    cmap='viridis',
    s=df['wt'] * 40,
    alpha=0.8
)

fig.colorbar(scatter, ax=ax, label='Cylinders')
ax.set_title('HP vs MPG (Sized by Weight)')
ax.set_xlabel('Horsepower')
ax.set_ylabel('MPG')
"#;

Plot::<Matplotlib>::build(data!(&df)?)?
    .with_exe_path("python")?
    .with_plotting_code(code)
    .save("mat_custom.svg")?;
```

### 8.3.5 Using Python Libraries with Matplotlib (not tested)
Because the backend runs arbitrary Python code, you can integrate any Python library, for example:
* seaborn for statistical plots
* scipy for fitting or curves
* sklearn for clustering overlay
* pandas for quick plotting

Example with seaborn:

```rust
let code = r#"
import seaborn as sns
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
sns.regplot(data=df, x='hp', y='mpg', ax=ax)
ax.set_title('Regression Line of HP vs MPG')
"#;
```

This freedom allows Rust code to leverage the entire Python scientific visualization stack.

### 8.3.6 Full Example: Multi-encoding Scatterplot
This example uses color mapping, different marker sizes, and labels:
```rust
let code = r#"
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))

sc = ax.scatter(
    df['hp'], df['mpg'],
    c=df['cyl'], cmap='tab10',
    s=df['wt'] * 35,
    alpha=0.85, edgecolor='black'
)

ax.set_title('HP vs MPG (Colored by Cylinders, Sized by Weight)')
ax.set_xlabel('Horsepower')
ax.set_ylabel('Miles/Gallon')

cbar = fig.colorbar(sc, ax=ax)
cbar.set_label('Cylinders')

fig.tight_layout()
"#;

Plot::<Matplotlib>::build(data!(&df))?
    .with_exe_path("python")?
    .with_plotting_code(code)
    .save("mat_full.svg")?;
```

# Chapter 9: Interactive Workflows & WebAssembly Integration
Charton provides two categories of visualization output:

1. **Static rendering** (native Charton SVG)
2. **Interactive rendering** (via the Altair backend and Vega-Lite)

Although Charton’s native renderer produces static SVG files, it can still participate in interactive workflows in several environments (e.g., Jupyter), and Charton can also generate *fully interactive* visualizations by delegating to the Altair/Vega-Lite ecosystem.

This chapter explains these modes and clarifies the underlying architecture.

## 9.1 Static interactive-style display in Jupyter (via `evcxr`)
Charton integrates with `evcxr` to display static charts *inline* inside Jupyter notebooks. This mode is “static” because the output is a fixed SVG, but it behaves “interactive-style” because:
- Each execution immediately re-renders the chart inside the notebook  
- Any changes to code/data result in instant visual updates  
- Ideal for exploration, education, and iterative refinement

This is similar to how Plotters or PlotPy integrate with `evcxr`.

### Example: Displaying a Charton chart inline in Jupyter
```rust
:dep charton = { version="0.2.0" }
:dep polars = { version="0.49.1" }

use charton::prelude::*;
use polars::prelude::*;

// Create sample data
let df = df![
    "length" => [5.1, 4.9, 4.7, 4.6, 5.0],
    "width"  => [3.5, 3.0, 3.2, 3.1, 3.6]
]?;

// Build a simple scatter plot
Chart::build(&df)?
    .mark_point()
    .encode((x("length"), y("width")))?
    .into_layered()
    .show()?;   // <-- Displays directly inside the Jupyter cell
```
Even though the chart itself is static, the *workflow* feels interactive due to the rapid feedback loop.

## 9.2 Static SVG vs. Interactive Rendering in WebAssembly
Although Charton’s native output is a **static SVG**, this does *not* prevent it from supporting interactive rendering when compiled to Wasm. In fact, the combination of **Charton + Rust + Wasm** enables a high-performance interaction model that is often *faster* than traditional JavaScript visualization libraries.

To understand this correctly, we must distinguish two different concepts:
- **Static** — refers to the file format: SVG is a declarative XML graphics format.
- **Dynamic** — refers to the rendering and update pipeline: how a chart is recomputed and replaced in response to user input.

**🔑 Key Idea: Charton SVGs Are Not “Immutable”**

The SVG that Charton produces is a static file format, but this does **not** mean the visualization must remain static in the browser. The core principle of the Charton + Wasm model is:

> Interactions do not modify the SVG in-place.
> Instead, Charton’s Rust/Wasm runtime dynamically recomputes and regenerates a new SVG whenever needed.

Thus, the browser simply re-renders the updated SVG structure.

This architecture provides both simplicity (SVG is easy to embed, style, and display) and performance (Wasm + Polars + Rust for fast recomputation).

### 9.2.1 Interaction Does Not Require Canvas
Interactive visualization is *not* exclusive to Canvas or WebGL.
SVG supports two fundamentally different interaction models:
| **Interaction Model**                           | **Description**                                                                                          | **Suitable For**                                              |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| **DOM-driven interactions (CSS/JS)**            | Browser handles hover, click, and small style changes by directly modifying SVG element attributes.      | Tooltips, highlighting, simple UI responses.                  |
| **Wasm-driven interactions (high-performance)** | Rust/Wasm computes a completely new SVG (or a DOM patch) on each interaction and replaces the old chart. | Zooming, panning, filtering, re-aggregating, re-scaling axes. |

Charton’s design focuses on *the second model*, where Rust/Wasm performs the heavy lifting.

### 9.2.2 Wasm-Driven Interactive Rendering Pipeline
When a user interacts with a Charton chart compiled to Wasm, the pipeline works as follows:
- The browser captures a user event—e.g., a drag event for zooming or a brush gesture for selecting a range.
- Using `wasm-bindgen`, the event details are passed into the Charton Rust core.
- The Rust engine performs full or partial chart recomputation. These operations run at native-like speed inside Wasm.
- Charton generates a new SVG string or structured DOM patch representing the new view.
- The browser replaces the old SVG node with the new one.

Charton’s Wasm-driven model has several performance advantages:

**1. Polars performance inside Wasm**
Traditional JS libraries rely on JavaScript arrays, D3 computations, or slower JS-based DataFrame libraries.
Charton instead executes **Polars** in Wasm—offering:
- zero-copy columnar data
- vectorized operations
- multi-threaded execution (where supported)

**2. Rust efficiency**
All chart logic—scales, encodings, transforms, layouts—is executed in **compiled Rust**, not interpreted JS.

**3. SVG rendering advantages**
SVG is declarative; modern browsers:
- batch DOM updates
- optimize SVG rendering paths
- offload rendering to GPU when possible

This drastically reduces UI-thread blocking compared to manual JS DOM manipulation.

### 9.2.3 Charton + Polars + wasm-bindgen — step-by-step example
> Goal: expose a `draw_chart()` function from Rust → returns an SVG string → JavaScript inserts that SVG into the DOM.

**0) Prerequisites**
- Rust toolchain (stable), with `rustup`.
- `wasm-pack` (recommended) OR `wasm-bindgen-cli` + `cargo build --target wasm32-unknown-unknown`.
    - Install `wasm-pack` (recommended):

      `cargo install wasm-pack`
- `clang` (required)
    - **Linux**: `apt install clang`
    - **Windows**: Download and run the **LLVM installer** from [LLVM Releases](https://github.com/llvm/llvm-project/releases). During installation, select **"Add LLVM to the system PATH"**.
- A simple static file server (e.g. `basic-http-server` from cargo, `python -m http.server`, or `serve` via npm).
- Node/ npm only if you want to integrate into an NPM workflow; not required for the simple demo.

> **Important compatibility note (read before you start):**

Many crates (especially heavy ones like `polars` or visualization crates) may have limited or no support for `wasm32-unknown-unknown` out of the box. If Polars and Charton compile to wasm in your environment, the steps below will work. If they don't, read the **Caveats & alternatives** section at the end.

**1) Project layout**

Assume you created a project:
```text
web
├── Cargo.toml
├── index.html
├── pkg
│   ├── package.json
│   ├── web_bg.wasm
│   ├── web_bg.wasm.d.ts
│   ├── web.d.ts
│   └── web.js
└── src
    └── lib.rs
```
We will build a `cdylib` wasm package that `wasm-pack` will wrap into `pkg/`.

**2)** `Cargo.toml`**(example)**

Put this into `web/Cargo.toml`.
```toml
[package]
name = "web"
version = "0.1.0"
edition = "2021" # Important: Stable standard for Wasm/Polars. Don't upgrade to 2024 yet to avoid toolchain conflicts.

# Produce a cdylib for wasm
[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
polars = { version = "0.49", default-features = false }
# Avoids transitive mio dependency to ensure Wasm compatibility.
polars-io = { version = "0.49", default-features = false, features = ["parquet"] }
charton = { version = "0.2" }

[profile.release]
opt-level = "z"  # or "s" to speed up
lto = true
codegen-units = 1
panic = "abort"
```

**3)** `src/lib.rs`**-Rust (wasm entry points)**

Create `web/src/lib.rs`.
```rust
use wasm_bindgen::prelude::*;
use polars::prelude::*;
use charton::prelude::*;

// Build a small scatter plot and return the SVG string.
#[wasm_bindgen]
pub fn draw_chart() -> Result<String, JsValue> {
    // Create a tiny DataFrame
    let df = df![
        "length" => [5.1, 4.9, 4.7, 4.6, 5.0, 5.4, 4.6, 5.0, 4.4, 4.9],
        "width" => [3.5, 3.0, 3.2, 3.1, 3.6, 3.9, 3.4, 3.4, 2.9, 3.1]
    ].map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Build a Charton Chart
    let scatter = Chart::build(&df)
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .mark_point()
        .encode((x("length"), y("width")))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let chart = LayeredChart::new().add_layer(scatter);

    let svg = chart.to_svg()
        .map_err(|e| JsValue::from_str(&e.to_string()))?; // Returns SVG string

    Ok(svg)
}
```
Key points:

- `#[wasm_bindgen]` exposes functions to JS.
- We return `Result<String, JsValue>` so JS receives errors as exceptions.

**4) Build with** `wasm-pack` **(recommended)**

From project root (`web/`):
```bash
wasm-pack build --release --target web --out-dir pkg
```
`wasm-pack` will:
- compile to `wasm32-unknown-unknown`,
- run `wasm-bindgen` to generate JS wrapper(s),
- produce a `pkg/` folder containing:

    - `web_bg.wasm`
    - `web_bg.wasm.d.ts`
    - `web.d.ts`
    - `web.js` (ES module bootstrap)
> 💡**Optimization Note: Binary Size**

> After building in `--release` mode, the resulting `web_bg.wasm` is approximately **4MB**. However, for web production:
> - **Gzip compression** reduces it to about **900KB**.
> - **Brotli compression** can shrink it even further.
> This compact footprint makes it highly suitable for browser-side data processing without long loading times.

**5) Creating `index.html` (Client-Side Loader)**

The final step is to create a minimal HTML file (`web/index.html`) that loads the generated WASM module and renders the SVG chart into the page.
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Charton WASM Demo</title>
</head>
<body>
    <div id="chart-container"></div>

    <script type="module">
        import init, { draw_chart } from './pkg/web.js';

        async function run() {
            // Initialize and load the WebAssembly module
            await init();

            // Call the Rust function that returns an SVG string
            const svg = draw_chart();

            // Insert the SVG into the page
            document.getElementById("chart-container").innerHTML = svg;
        }

        run();
    </script>
</body>
</html>
```
This minimal version:
- Loads the WASM module generated by `wasm-pack`
- Calls the Rust function `draw_chart()` to generate the SVG string
- Injects the SVG directly into the DOM
- Contains no additional CSS, error handling, or panic hooks — keeping the example simple and focused

This is the recommended simplest setup for demonstrating Charton rendering through WebAssembly.

**6) Serve the folder**

Browsers enforce CORS for WASM; open the page via HTTP server rather than `file://`.

Minimal options:
```bash
cd web
python3 -m http.server 8080
```
Then open http://localhost:8080/index.html and you'll see the chart in the browser:
![wasm](../assets/wasm.png)

### 9.2.4 Conclusion
The combination of *static* SVG and *dynamic* Rust/Wasm computation forms a powerful model for interactive visualization:
- SVG provides simple, portable output for embedding and styling.
- Rust/Wasm enables high-performance chart recomputation.
- Polars accelerates data transformations dramatically.
- Browser handles final rendering efficiently.

**Charton does not attempt to patch SVGs with JavaScript like traditional libraries. Instead, it regenerates a complete static SVG—fast enough to support real-time interactivity.**

This architecture makes high-performance, browser-based interaction not only possible but highly efficient.

## 9.3 True interactive visualization via the Altair backend
Charton can generate fully interactive charts by delegating to **Altair**, which compiles to Vega-Lite specifications capable of:
- Hover tooltips
- Selections
- Brush interactions
- Zoom and pan
- Linked views
- Filtering and conditional styling
- Rich UI semantics

**Charton’s role in this workflow**

Charton does:
1. Run Rust-side preprocessing (Polars)
2. Transfer data to Python
3. Embed user-provided Altair plotting code
4. Invoke Python to generate Vega-Lite JSON
5. Display the result (browser/Jupyter) or export JSON

All *actual* interactivity comes from **Altair/Vega-Lite**, not from Charton.

**Example: interactive Altair chart via Charton**
```rust
:dep charton = { version="0.2.0" }
:dep polars = { version="0.49.1" }

use charton::prelude::*;
use polars::prelude::df;

let exe_path = r"D:\Programs\miniconda3\envs\cellpy\python.exe";

let df1 = df![
    "Model" => ["S1", "M1", "R2", "P8", "M4", "T5", "V1"],
    "Price" => [2430, 3550, 5700, 8750, 2315, 3560, 980],
    "Discount" => [Some(0.65), Some(0.73), Some(0.82), None, Some(0.51), None, Some(0.26)],
].unwrap();

// Any valid Altair code can be placed here.
let raw_plotting_code = r#"
import altair as alt

chart = alt.Chart(df1).mark_point().encode(
    x='Price',
    y='Discount',
    color='Model',
    tooltip=['Model', 'Price', 'Discount']
).interactive()        # <-- zoom + pan + scroll
"#;

Plot::<Altair>::build(data!(&df1)?)?
    .with_exe_path(exe_path)?
    .with_plotting_code(raw_plotting_code)
    .show()?;  // Jupyter or browser
```

This provides **real interactivity** entirely through Altair.

## 9.4 Exporting Vega-Lite JSON for browser/Web app usage
Since Altair compiles to Vega-Lite, Charton can generate the JSON specification directly.

This is ideal for:
- Web dashboards
- React / Vue / Svelte components
- Embedding charts in HTML
- APIs returning visualization specs
- Reproducible visualization pipelines

**Example: Export to JSON**
```rust
let chart_json: String = Plot::<Altair>::build(data!(&df1)?)?
    .with_exe_path(exe_path)?
    .with_plotting_code(raw_plotting_code)
    .to_json()?;

// save, embed, or send via API
println!("{}", chart_json);
```

**Embedding in a webpage**:
```html
<div id="vis"></div>
<script>
  var spec = /* paste JSON here */;
  vegaEmbed('#vis', spec);
</script>
```

## 9.5 Summary: What kinds of interactivity does Charton support?
| **Feature**                                          | **Supported?** | **Provided by**         |
| ---------------------------------------------------- | ----------     | ----------------------- |
| Hover tooltips                                       | ✔ Yes         | Altair/Vega-Lite        |
| Selection / brushing                                 | ✔ Yes         | Vega-Lite               |
| Zoom / pan                                           | ✔ Yes         | Altair `.interactive()` |
| Dynamic UI-driven filtering                          | ✔ Yes         | Vega-Lite               |
| Inline static charts in Jupyter                      | ✔ Yes         | Charton SVG via `evcxr`  |
| True reactive Rust-side charts (recompute on events) | ❌ No         | —                       |
| Charton-native browser interactivity                  | ❌ No         | —                       |

**When to use which mode?**
| **Use Case**                    | **Recommended Mode**                           |
| ------------------------------- | ----------------------------------------------- |
| Fast feedback in Rust           | Jupyter + `evcxr` static SVG                    |
| Publication-quality plots       | Native Charton SVG                               |
| Hover/tooltip/zoom              | Altair backend                                  |
| Web dashboards or JS frameworks | Export Vega-Lite JSON                           |
| Rust/WASM interactive apps      | Use Charton as SVG generator + custom WASM logic |

# Chapter 10 · Performance Optimization

Charton is optimized for large datasets and high-performance workflows, especially when paired with Polars.  
This chapter explains best practices for handling large data, minimizing memory usage, improving rendering speed, and leveraging lazy evaluation.  
It offers guidelines for scaling your visualizations to millions of data points without sacrificing responsiveness.  
By applying these techniques, you can confidently use charton in production-scale environments.

### 10.1 Polars + charton performance pipeline
How Polars lazy engine and charton complement each other.

### 10.2 Handling large datasets
Sampling, binning, aggregation strategies.

### 10.3 Rendering performance
When to use native vs external backends.

### 10.4 Memory management
Strategies to avoid unnecessary copies and overhead.

### 10.5 Parallel or multi-stage pipelines
Leverage Rust concurrency for performance gains.

---

# Chapter 11 · Extensibility, Troubleshooting & Community

Charton is designed to be extensible.  
This final chapter shows how to create custom marks, define new encodings, extend palettes, and contribute to the project.  
It also includes a comprehensive troubleshooting guide, FAQs, and links to community resources.  
After reading this chapter, you will be fully equipped not only to use charton, but also to extend and improve it—joining the ecosystem as a contributor.

## 11.1 Creating custom marks
Implementing new mark types.

## 11.2 Adding new encodings
Defining custom encoding channels.

## 11.3 Extending themes and palettes
Building reusable visual themes.

## 11.4 Troubleshooting common issues
**Q: Why do I get this error?**
`the trait bound `InputData: From<(&str, &DataFrame)>` is not satisfied`
or:
`the trait bound `InputData: From<(&str, &LazyFrame)>` is not satisfied`

**A**: Because your Polars version is not compatible with the Polars version used inside Charton, so the types cannot be converted. You have two options:
- Use a Polars version that matches the version required by Charton.
- If you don’t want to change versions, write your DataFrame to a Parquet file and let Charton read it (see Section 2.4.4).

## 11.5 FAQ
Answers to common user questions.

## 11.6 Contributing to charton
Code structure, contribution guidelines, and development workflow.

## 11.7 Community & resources
Links to docs, repo, issues, examples, and community channels.
