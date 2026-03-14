### Wasm-Driven Interactive Rendering Pipeline
Charton can be compiled to **WebAssembly (WASM)**, bringing Rust's near-native performance to the browser. This enables a high-performance interaction model that handles large-scale datasets with lower latency than traditional JavaScript-based visualization libraries.

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
charton = { version = "0.4" }

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
    let chart = Chart::build(&df)
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .mark_point()?
        .encode((x("length"), y("width")))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

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
charton = { version = 0.4 }

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
    let chart = Chart::build(&df)
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .mark_point()
        .encode((x("sepal_length"), y("sepal_width"), color("species")))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    /* * 4. Generate SVG.
     * The to_svg() method returns a raw XML string representing the vector graphic.
     */
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


