# Quick Start
Welcome to **Charton Quick Start**! 

This chapter will guide you through creating charts in Rust using Charton from scratch. By the end of this chapter, you'll know how to:

- Initialize a Rust project and add Charton dependencies
- Load and preprocess data using Polars
- Build charts using Chart, Mark, and Encoding
- Render charts in multiple formats and environments
- Avoid common pitfalls and errors

The goal is to make you productive **within minutes**.

## Project Setup
First, create a new Rust project:

```bash
cargo new demo
cd demo
```
Edit your `Cargo.toml` to add Charton and Polars dependencies:
```toml
[dependencies]
charton = "0.3"
polars = { version = "0.49", features = ["lazy", "csv", "parquet"] }
```
Run `cargo build` to ensure everything compiles.

## Creating Your First Chart
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
        .mark_point()?          // Mark: Specifies the visual primitive (dots)
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
    .mark_point()?
    .encode((x("wt"), y("mpg")))?
    .into_layered()
    .show()?;
```

You can even save the chart object to a variable and use it later. For example:
```rust
// ... (using the same 'df' DataFrame)
let chart = Chart::build(&df)?
    .mark_point()?
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
        .mark_point()?                  // Mark: Specifies the visual primitive (dots)
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

## Loading and Preparing Data
Before creating visualizations, Charton requires your data to be stored in a Polars `DataFrame`. Charton itself does not impose restrictions on how data is loaded, so you can rely on Polars’ powerful I/O ecosystem.

### Built-in Datasets
Charton provides a few built-in datasets for quick experimentation, demos, and tutorials.
```rust
let df = load_dataset("mtcars")?;
```
This returns a Polars DataFrame ready for visualization.

### Loading CSV Files
CSV is the most common format for tabular data. Using Polars:
```rust
use polars::prelude::*;

let df = CsvReadOptions::default()
    .with_has_header(true)
    .try_into_reader_with_file_path(Some("./datasets/iris.csv".into()))?
    .finish()?;
```

### Loading Parquet Files
Parquet is a high-performance, columnar storage format widely used in data engineering.
```rust
let file = std::fs::File::open("./datasets/foods.parquet")?;
let df = ParquetReader::new(file).finish()?;
```
Parquet is recommended for large datasets due to compression and fast loading.

### Loading Data from Parquet Bytes (`Vec<u8>`) — Cross-Version Interoperability
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
charton = { version = "0.3" }
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
        .mark_point()?
        .encode((
            x("length"),
            y("width"),
        ))?
        .into_layered()
        .save("./scatter.svg")?;

    Ok(())
}
```

## Simple Plotting Examples
This section introduces the most common chart types in Charton.

### Line Chart
```rust
// Create a polars dataframe
let df = df![
    "length" => [4.4, 4.6, 4.7, 4.9, 5.0, 5.1, 5.4], // In ascending order
    "width" => [2.9, 3.1, 3.2, 3.0, 3.6, 3.5, 3.9]
]?;

// Create a line chart layer
Chart::build(&df)?
    .mark_line()?              // Line chart
    .encode((
        x("length"),           // Map length column to X-axis
        y("width"),            // Map width column to Y-axis
    ))?
    .into_layered()
    .save("line.svg")?;
```
Useful for trends or ordered sequences.

### Bar Chart
```rust
let df = df! [
    "type" => ["a", "b", "c", "d"],
    "value" => [4.9, 5.3, 5.5, 6.5],
]?;

Chart::build(&df)?
    .mark_bar()?
    .encode((
        x("type"),
        y("value"),
    ))?
    .into_layered()
    .save("bar.svg")?;
```

### Histogram
```rust
let df = load_dataset("iris")?;

Chart::build(&df)?
    .mark_hist()?
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

### Boxplot
```rust
let df = load_dataset("iris")?;

Chart::build(&df)?
    .mark_boxplot()?
    .encode((x("species"), y("sepal_length")))?
    .into_layered()
    .save("boxplot.svg")?;
```
Boxplots summarize distributions using quartiles, medians, whiskers, and outliers.

### Layered Charts
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
        .mark_line()?                       // Line chart
        .encode((
            x("length"),                    // Map length column to X-axis
            y("width"),                     // Map width column to Y-axis
        ))?;

    // Create a scatter point layer
    let scatter = Chart::build(&df)?
        .mark_point()?                      // Scatter plot
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

## Exporting Charts
Charton supports exporting charts to different file formats depending on the selected rendering backend. All backends share the same API:

```rust
chart.save("output.png")?;
```
The file format is inferred from the extension.

This section describes the supported formats and saving behavior for each backend.

### Rust Native Backend

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

### Altair Backend (Vega-Lite)

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

### Matplotlib Backend

The Matplotlib backend supports:

- **PNG** — returned as base64 from Python, decoded and saved

**Saving PNG**
```rust
chart.save("chart.png")?;
```
Other formats (SVG, JSON, PDF, etc.) are not currently supported by this backend.

### Unsupported Formats & Errors

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
### Summary of Supported Formats
| Backend     | SVG | PNG | JSON |
| ----------- | :-: | :-: | :--: |
| Rust Native |  ✔️ |  ✔️ |   ❌  |
| Altair      |  ✔️ |  ✔️ |  ✔️  |
| Matplotlib  |  ❌  |  ✔️ |   ❌  |

### Exporting Charts as Strings (SVG / JSON)
In addition to saving charts to files, Charton also supports exporting charts **directly as strings**.
This is useful in environments where writing to disk is undesirable or impossible, such as:

- Web servers returning chart data in API responses
- Browser/WASM applications
- Embedding charts into HTML templates
- Passing Vega-Lite specifications to front-end visualizers
- Testing and snapshot generation

Charton provides two kinds of in-memory exports depending on the backend.

**SVG Output (Rust Native Backend)**
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

*Example*
```rust
let svg = chart.to_svg()?;
```
**Vega-Lite JSON (Altair Backend)**
When using the Altair backend, charts can be exported as raw *Vega-Lite JSON*:
```rust
let json = chart.to_json()?;
```
This produces the complete Vega-Lite specification generated by Altair. Typical usage scenarios include:
- Front-end rendering using Vega/Vega-Lite
- Sending the chart spec from a Rust API to a browser client
- Storing chart specifications in a database
- Generating reproducible visualization specs

*Example*
```rust
let json_spec = chart.to_json()?;
println!("{}", json_spec);
```
This JSON is fully compatible with the *official online Vega-Lite editor*.

**Summary: In-Memory Export Methods**

| Backend     | `to_svg()`    | `to_json()`              |
| ----------- | ------------- | ------------------------ |
| Rust Native | ✔️ SVG string | ❌ unsupported            |
| Altair      | ❌ (file-only) | ✔️ Vega-Lite JSON string |
| Matplotlib  | ❌             | ❌                        |

String-based export complements file export by enabling fully in-memory rendering and programmatic integration.

## Viewing Charts
Charton charts can be viewed directly inside *Evcxr Jupyter notebooks* using the `.show()` method.  

When running inside Evcxr Jupyter, Charton automatically prints the correct MIME content so that the chart appears inline.

Outside Jupyter (e.g., running a binary), `.show()` does nothing and simply returns `Ok(())`.

The rendering behavior differs depending on the selected backend.

## Rust Native Backend
The Rust-native backend renders charts to *inline SVG*.  

When `.show()` is called inside Evcxr Jupyter, the SVG is printed using `text/html` MIME type.

*Example*

```rust
use charton::prelude::*;
use polars::prelude::*;

let df = df![
    "x" => [1, 2, 3],
    "y" => [10, 20, 30]
]?;

let chart = Chart::build(&df)?
    .mark_point()?
    .encode((x("x"), y("y")))?
    .into_layered();

chart.show()?;   // displays inline SVG in Jupyter
```
*Internal Behavior*
```text
EVCXR_BEGIN_CONTENT text/html
<svg>...</svg>
EVCXR_END_CONTENT
```
This enables rich inline SVG display in notebooks.

### Altair Backend (Vega-Lite)
When using the Altair backend, `.show()` emits *Vega-Lite JSON* with the correct MIME type:
```bash
application/vnd.vegalite.v5+json
```
Jupyter then renders the chart using the built-in Vega-Lite renderer.

*Example*
```rust
chart.show()?;   // displays interactive Vega-Lite chart inside Jupyter
```
*Internal Behavior*
```text
EVCXR_BEGIN_CONTENT application/vnd.vegalite.v5+json
{ ... Vega-Lite JSON ... }
EVCXR_END_CONTENT
```
This produces interactive charts (tooltips, zooming, etc.) if supported by the notebook environment.

### Matplotlib Backend
The Matplotlib backend produces *base64-encoded PNG* images and sends them to the notebook using `image/png` MIME type.

*Example*

```rust
chart.show()?;   // displays inline PNG rendered by Matplotlib
```

*Internal Behavior*

```text
EVCXR_BEGIN_CONTENT image/png
<base64 image>
EVCXR_END_CONTENT
```
### Summary: What `.show()` displays in Jupyter

| *Backend* | *Output Type* | *MIME Type*     |
| ----------- | ----------- | ---------------------------------- |
| Rust Native | SVG         | `text/html`                        |
| Altair      | Vega-Lite   | `application/vnd.vegalite.v5+json` |
| Matplotlib  | PNG         | `image/png`                        |

`.show()` is designed to behave naturally depending on the backend, giving the best viewing experience for each renderer.

## Summary
In this chapter, you learned how to:
- Load datasets from CSV, Parquet, and built-in sources
- Create essential chart types: scatter, bar, line, histogram, boxplot, layered plots
- Export your charts to SVG, PNG, and Vega JSON
- Preview visualizations in the notebook

With these foundations, you now have everything you need to build *end-to-end data visualizations* quickly and reliably. The next chapters will introduce the building blocks of Charton, including marks and eocodings.
