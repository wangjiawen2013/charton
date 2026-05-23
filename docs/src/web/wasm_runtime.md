# WASM Runtime Rendering

This chapter walks a complete beginner from zero to a real-time, color-gradient scatter plot running in the browser, powered by Rust and WebAssembly. The goal is to expose a `draw_wave()` function from Rust → returns an SVG string with a color gradient → JavaScript drives a smooth animation by replacing the SVG in the DOM at ~20–25 FPS.

> Every frame is a brand‑new SVG computed by Rust in WebAssembly.

## 0) Prerequisites

* Rust toolchain (stable) – [install via rustup](https://rustup.rs/)
* wasm-pack – install with:
`cargo install wasm-pack`
* A static file server – choose one:
    - Python: `python -m http.server 8080` (make sure Python has been installed)
    - Node.js: `npx serve .`
    - Or any other HTTP server (browsers require HTTP for WASM)
* clang (may be required on some systems)
    - Linux: `sudo apt install clang`
    - Windows: Download LLVM from [releases.llvm.org](https://github.com/llvm/llvm-project/releases) and select *Add LLVM to the system PATH*
    - macOS: usually pre‑installed with Xcode command line tools

> Important compatibility note:
> `charton` v0.5 depends on `getrandom`, which needs special configuration for `wasm32-unknown-unknown`. This tutorial includes all required settings.

## 1) Project Layout

Create a new project (e.g., `cargo new wave --lib`) and set up the following structure:

```text
wave
├── Cargo.toml
├── index.html
├── pkg
└── src
    └── lib.rs
```

We will build a `cdylib` wasm package that wasm-pack will wrap into `pkg/`.

## 2) `Cargo.toml`

Put this into `wave/Cargo.toml`:

```toml
[package]
name = "wave"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]     # Produces a dynamic library for WASM

[dependencies]
wasm-bindgen = "0.2"        # JS ↔ Rust bridge
charton = "0.5"             # Declarative plotting library

# getrandom must be explicitly added with the "wasm_js" feature flag
# for wasm32-unknown-unknown target support.
getrandom = { version = "0.3", features = ["wasm_js"] }

[profile.release]
opt-level = "s"             # Optimize for size
lto = true                  # Link-time optimization
codegen-units = 1           # Better optimization
panic = "abort"             # Smaller panic handler
```

## 3) `src/lib.rs`- Rust (wasm entry points)

Create a `lib.rs` file in the `src` directory and add the following code: 

```rust
//! Charton WASM demo: real-time animated line chart with color gradient.
//!
//! This module exposes a single function, `draw_wave`, which takes three
//! numeric arrays and returns an SVG string. The color channel is mapped
//! directly to the y-value, producing a continuous color gradient along the line.

use wasm_bindgen::prelude::*;
use charton::prelude::*;

/// Generate an SVG line chart with a color gradient.
///
/// # Arguments
/// * `xs` - X-axis values (e.g., time steps)
/// * `ys` - Y-axis values (e.g., amplitude)
/// * `colors` - Values for the continuous color scale (can be the same as `ys`)
///
/// # Returns
/// A `Result` containing the SVG string or a JavaScript error.
#[wasm_bindgen]
pub fn draw_wave(
    xs: Vec<f64>,
    ys: Vec<f64>,
    colors: Vec<f64>,
) -> Result<String, JsValue> {
    // Build a Charton Dataset from the three columns
    let ds = Dataset::new()
        .with_column("x", xs)
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .with_column("y", ys)
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .with_column("color", colors)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Build a chart using the declarative API
    let chart = Chart::build(ds)
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .mark_point()                                       // Use a line mark
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .encode((                                           // Map columns to visual channels
            alt::x("x"),
            alt::y("y"),
            alt::color("color"),                            // Continuous color scale
        ))
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .with_size(800, 400)
        .configure_theme(|t| t.with_left_margin(0.01).with_top_margin(0.12).with_bottom_margin(0.05));

    // Render the chart to a static SVG string
    let svg = chart
        .to_svg()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(svg)
}
```

## 4) Build with `wasm-pack`

From the project root (`wave/`):

```bash
wasm-pack build --release --target web --out-dir pkg
```

`wasm-pack` will:

- Compile to `wasm32-unknown-unknown`
- Run `wasm-bindgen` to generate JavaScript bindings
- Output everything into `pkg/`:
    - `wave_bg.wasm` – the compiled WebAssembly binary
    - `wave.js` – ES module bootstrap
    - `wave.d.ts` – TypeScript declarations (optional)

> The .wasm file is roughly 300 kb in release mode. Gzip or Brotli compression can bring it down further, perfectly fine for web delivery.

## 5) `index.html` – Animated Frontend

Create `index.html` in the project root. The JavaScript:

* Initialises the WASM module
* Runs an animation loop with requestAnimationFrame
* Pushes a new data point (sine wave + noise) every ~40 ms
* Passes the arrays to draw_wave() and replaces the SVG in the DOM

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Charton WASM — Gradient Wave</title>
    <style>
        /* Dark background to make the gradient pop */
        body {
            font-family: system-ui, sans-serif;
            display: flex;
            flex-direction: column;
            align-items: center;
            margin: 0;
            padding-top: 2rem;
            background: #0d1117;
            color: #c9d1d9;
        }
        #chart {
            width: 800px;
            height: 400px;
            border-radius: 12px;
            background: #161b22;
            border: 1px solid #30363d;
            box-shadow: 0 4px 16px rgba(0,0,0,0.6);
        }
        .tag {
            margin-top: 1rem;
            font-size: 0.9rem;
            color: #8b949e;
        }
    </style>
</head>
<body>
    <h2>🌈 Charton + WASM — Gradient Wave</h2>
    <div id="chart"></div>
    <div class="tag">Every frame is a brand‑new SVG computed by Rust in WebAssembly</div>

    <script type="module">
        // Import the generated JS glue and the Rust function
        import init, { draw_wave } from './pkg/wave.js';

        async function run() {
            // Boot the WASM module
            await init();

            const container = document.getElementById('chart');
            const WINDOW_SIZE = 200;          // Show the latest 200 data points
            const ADD_INTERVAL_MS = 40;       // Add a new point every 50ms
            let xs = [];
            let ys = [];
            let t = 0;                        // Time counter
            let lastAdd = 0;                  // Timestamp of the last data addition

            // Animation loop driven by requestAnimationFrame
            function loop(timestamp) {
                // Only append a new point if enough time has elapsed
                if (timestamp - lastAdd >= ADD_INTERVAL_MS) {
                    // Sine wave with a little noise for a more organic look
                    const y = Math.sin(t * 0.3) + (Math.random() - 0.5) * 0.2;
                    xs.push(t);
                    ys.push(y);

                    // Keep only the latest WINDOW_SIZE points
                    if (xs.length > WINDOW_SIZE) {
                        xs.shift();
                        ys.shift();
                    }
                    t += 0.5;
                    lastAdd = timestamp;

                    try {
                        // The color column is just a copy of the y-values,
                        // which gives a nice blue‑to‑orange gradient.
                        const svg = draw_wave(xs, ys, [...ys]);
                        container.innerHTML = svg;
                    } catch (e) {
                        console.error(e);
                    }
                }
                requestAnimationFrame(loop);
            }

            requestAnimationFrame(loop);
        }

        run();
    </script>
</body>
</html>
```

## 6) Serve and View

Open a terminal in the project directory and start a local server:

```bash
python -m http.server 8080
```

Then open http://localhost:8080 in your browser.

You will see a dark-themed page with a flowing stream of coloured dots – the colour changes smoothly from cool (trough) to warm (peak), and the entire chart is re‑rendered from scratch by Rust on every frame.

## 7) Troubleshooting
* Compilation freezes / high RAM usage – Building for WASM can be heavy. If the process hangs during wasm-opt, you can stop it manually; the unoptimised `.wasm` is already functional and will run in the browser.
* `wasm-opt` errors – If `wasm-pack` fails to install or run `wasm-opt`, ignore the error as long as `pkg/` has been populated.
* Port already in use – Try a different port: `python -m http.server 8000`.
* Chart appears but no colour gradient – Make sure you are passing three vectors to `draw_wave` and that the third one is a numeric array (not all the same value). Check the browser console for any Rust panics.
* Blank page or CORS errors – Always use an HTTP server, never open the HTML file directly with `file://`.

## What's Next?

* Adjust the animation speed by changing `t += 0.5` and `ADD_INTERVAL_MS` and `WINDOW_SIZE` in `index.html`.
* Replace the sine wave with real‑time data fetched from an API.
* Explore Polars integration to pre‑process large datasets in the browser before plotting.