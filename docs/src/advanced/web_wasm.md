# Interactive Workflows & WebAssembly Integration
Charton provides two categories of visualization output:

1. **Static rendering** (native Charton SVG)
2. **Interactive rendering** (via the Altair backend and Vega-Lite)

Although Charton’s native renderer produces static SVG files, it can still participate in interactive workflows in several environments (e.g., Jupyter), and Charton can also generate *fully interactive* visualizations by delegating to the Altair/Vega-Lite ecosystem.

This chapter explains these modes and clarifies the underlying architecture.

## Static interactive-style display in Jupyter (via `evcxr`)
Charton integrates with `evcxr` to display static charts *inline* inside Jupyter notebooks. This mode is “static” because the output is a fixed SVG, but it behaves “interactive-style” because:
- Each execution immediately re-renders the chart inside the notebook  
- Any changes to code/data result in instant visual updates  
- Ideal for exploration, education, and iterative refinement

This is similar to how Plotters or PlotPy integrate with `evcxr`.

### Example: Displaying a Charton chart inline in Jupyter
```rust
:dep charton = { version="0.3.0" }
:dep polars = { version="0.49" }

use charton::prelude::*;
use polars::prelude::*;

// Create sample data
let df = df![
    "length" => [5.1, 4.9, 4.7, 4.6, 5.0],
    "width"  => [3.5, 3.0, 3.2, 3.1, 3.6]
]?;

// Build a simple scatter plot
Chart::build(&df)?
    .mark_point()?
    .encode((x("length"), y("width")))?
    .into_layered()
    .show()?;   // <-- Displays directly inside the Jupyter cell
```
Even though the chart itself is static, the *workflow* feels interactive due to the rapid feedback loop.

## Static SVG vs. Interactive Rendering in WebAssembly
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

### Interaction Does Not Require Canvas
Interactive visualization is *not* exclusive to Canvas or WebGL.
SVG supports two fundamentally different interaction models:
| **Interaction Model**                           | **Description**                                                                                          | **Suitable For**                                              |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| **DOM-driven interactions (CSS/JS)**            | Browser handles hover, click, and small style changes by directly modifying SVG element attributes.      | Tooltips, highlighting, simple UI responses.                  |
| **Wasm-driven interactions (high-performance)** | Rust/Wasm computes a completely new SVG (or a DOM patch) on each interaction and replaces the old chart. | Zooming, panning, filtering, re-aggregating, re-scaling axes. |

Charton’s design focuses on *the second model*, where Rust/Wasm performs the heavy lifting.

### Wasm-Driven Interactive Rendering Pipeline
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

### Charton + Polars + wasm-bindgen — step-by-step example
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
charton = { version = "0.3" }

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
        .mark_point()?
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

> After building in `--release` mode, the resulting `web_bg.wasm` is approximately **4 MB**. However, for web production:
> - **Gzip compression** reduces it to about **900 KB**.
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
python -m http.server 8080
```
Then open http://localhost:8080/index.html and you'll see the chart in the browser:
![wasm](../assets/wasm1.png)

**7) Troubleshooting**

Processing heavy libraries like Polars in WASM can strain your system. Here is how to handle common bottlenecks:
- **Compilation Hangs/Freezes:** Building Polars for WASM is extremely CPU and RAM intensive. If your computer "freezes" during the `Optimizing with wasm-opt` stage, you can manually stop the process. The compiled `.wasm` file in `pkg/` is usually already functional; it will simply be larger in size without the final optimization. For a smooth experience, a machine with high-core counts and 16GB+ RAM is recommended.
- **wasm-opt Errors:** If `wasm-pack` fails because it cannot install or run `wasm-opt`, you can simply ignore the error if the `pkg/` folder was already populated. The unoptimized WASM file will still run in the browser.
- **Polars Version Incompatibility:** If your project requires a Polars version uncompatible with the one used by Charton, passing a DataFrame directly will cause a compilation error. In this case, you can use the Parquet Interoperability method described in Section 2.3.4.

### Charton + Polars + wasm-bindgen — advanced example: dynamic CSV visualization
**Goal:** Beyond a static demo, we now build a functional tool: users upload a local CSV file (e.g., `iris.csv`, which can be found and downloaded from the `datasets/` folder in this project) → JavaScript reads it as a string → Rust/Polars parses the data in-browser → Charton generates a multi-colored scatter plot → The resulting SVG is rendered instantly.

**1) Updated** `Cargo.toml`
**Update Note:** This file updates the dependencies from 9.2.2 by enabling the `csv` feature in polars-io (to handle user uploads) and switching to the `charton` crate for more advanced encoding.
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
polars-io = { version = "0.49", default-features = false, features = ["parquet", "csv"] }
charton = { version = 0.3 }

[profile.release]
opt-level = "z"  # or "s" to speed up
lto = true
codegen-units = 1
panic = "abort"
```
**2) Updated** `src/lib.rs`
**Update Note:** This replaces the hard-coded `draw_chart` from 9.2.2. The new `draw_chart_from_csv` function accepts a `String` from JavaScript and uses `std::io::Cursor` to treat that string as a readable file stream for Polars.
```rust
use wasm_bindgen::prelude::*;
use polars::prelude::*;
use charton::prelude::*;
use std::io::Cursor;

#[wasm_bindgen]
pub fn draw_chart_from_csv(csv_content: String) -> Result<String, JsValue> {
    /* * 1. Parse CSV data from String.
     * We use a Cursor to treat the String as a readable stream for Polars.
     */
    let cursor = Cursor::new(csv_content);

    /* * 2. Initialize the Polars DataFrame.
     * CsvReader is highly optimized but runs in a single thread in standard WASM.
     */
    let df = CsvReader::new(cursor)
        .finish()
        .map_err(|e| JsValue::from_str(&format!("Polars Error: {}", e)))?;

    /* * 3. Construct the Scatter Plot.
     * Ensure that the columns "length" and "width" exist in your CSV file.
     */
    let scatter = Chart::build(&df)
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .mark_point()
        .encode((x("sepal_length"), y("sepal_width"), color("species")))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    /* * 4. Wrap the scatter plot into a LayeredChart and generate SVG.
     * The to_svg() method returns a raw XML string representing the vector graphic.
     */
    let chart = LayeredChart::new().add_layer(scatter);

    let svg = chart.to_svg()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(svg)
}
```
**3) Updated** `index.html`
**Update Note:** This expands the simple loader from 9.2.2 by adding a File Input UI and a `FileReader` event loop. This allows the WASM module to process "live" data provided by the user.
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>WASM CSV Visualizer</title>
    <style>
        #chart-container { margin-top: 20px; border: 1px solid #ccc; }
    </style>
</head>
<body>
    <h2>Upload CSV to Generate Chart</h2>
    <input type="file" id="csv-upload" accept=".csv" />
    
    <div id="chart-container"></div>

    <script type="module">
        import init, { draw_chart_from_csv } from './pkg/web.js';

        async function run() {
            // Initialize the WASM module
            await init();

            const fileInput = document.getElementById("csv-upload");
            const container = document.getElementById("chart-container");

            // Event listener for file selection
            fileInput.addEventListener("change", async (event) => {
                const file = event.target.files[0];
                if (!file) return;

                /* * Use FileReader to read the file content as text.
                 * This text is then passed across the JS-WASM boundary.
                 */
                const reader = new FileReader();
                reader.onload = (e) => {
                    const csvContent = e.target.result;

                    try {
                        // Call the Rust function with the CSV string
                        const svg = draw_chart_from_csv(csvContent);
                        
                        // Inject the returned SVG string directly into the DOM
                        container.innerHTML = svg;
                    } catch (err) {
                        console.error("Computation Error:", err);
                        alert("Error: Make sure CSV has 'length' and 'width' columns.");
                    }
                };
                
                // Trigger the file read
                reader.readAsText(file);
            });
        }

        run();
    </script>
</body>
</html>
```
**4) Build and Serve Update Note:** The build command remains the same as 9.2.3, but the compilation time may increase due to the added CSV and color encoding features.
```bash
# Build the package
wasm-pack build --release --target web --out-dir pkg

# Serve the files
python -m http.server 8080
```
**Summary of Improvements over 9.2.3**
- **Data Handling:** Shifted from static `df!` macros to dynamic `CsvReader` parsing.
- **Complexity:** Added `color` encoding in Charton to demonstrate multi-dimensional data mapping.
- **User Interaction:** Introduced the `FileReader` API to bridge the gap between the local file system and WASM linear memory.

### Conclusion
The combination of *static* SVG and *dynamic* Rust/Wasm computation forms a powerful model for interactive visualization:
- SVG provides simple, portable output for embedding and styling.
- Rust/Wasm enables high-performance chart recomputation.
- Polars accelerates data transformations dramatically.
- Browser handles final rendering efficiently.

**Charton does not attempt to patch SVGs with JavaScript like traditional libraries. Instead, it regenerates a complete static SVG—fast enough to support real-time interactivity.**

This architecture makes high-performance, browser-based interaction not only possible but highly efficient.

## True interactive visualization via the Altair backend
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
:dep charton = { version="0.3" }
:dep polars = { version="0.49" }

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

## Exporting Vega-Lite JSON for browser/Web app usage
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

## Summary: What kinds of interactivity does Charton support?
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
