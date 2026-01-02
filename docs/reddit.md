**Subreddit**: `r/rust` or `r/dataisbeautiful` **Title:** [Show & Tell] Charton: A Polars-native, Altair-style Declarative Plotting Library for Rust

**Body:**

Hi everyone,

I’m excited to announce the first public release of **Charton**, a plotting library designed to bring the ergonomics of Python’s **Altair/Vega-Lite** to the Rust and **Polars** ecosystem.

**GitHub:** [Charton](https://github.com/wangjiawen2013/charton) Crates.io: `charton = "0.2.0"`

🦀 **Why another plotting library?**

**As a Rust developer tired of context-switching to Python just for plotting, I spent much of my spare time building Charton to solve my own frustration.** We have great low-level tools like `plotters`, but for exploratory data analysis (EDA), we often miss the declarative "Grammar of Graphics" approach. Charton aims to bridge the gap between high-performance data processing in Polars and beautiful, rapid visualization.

✨ **Key Features:**
- **Polars-First & Wasm Ready:** Deeply integrated with Polars. Thanks to Rust's unique strengths, Charton is being optimized for WebAssembly, bringing high-performance, interactive data viz directly to the browser.
- **Declarative API:** If you've used Altair or ggplot2, you'll feel at home. Define *what* to plot, not how to draw lines.
```rust
Chart::build(&df)?
    .mark_point()
    .encode((x("length"), y("width"), color("species")))?
    .into_layered()
    .save("scatter.svg")?;
```
- **Version-Agnostic Data Exchange:** This is the "secret sauce." To avoid the common `orphan rule` issues and version mismatches between different Polars versions, Charton can exchange data via **Parquet-serialized bytes**. It's fast, safe, and avoids dependency hell.
- **Dual-Backend Strategy:**
    - **Native:** A pure-Rust SVG renderer (zero external dependencies, perfect for WASM/Server-side).
    - **External Bridge:** Seamlessly delegate complex plots to **Altair** or **Matplotlib** via a high-speed IPC mechanism—no slow temporary files involved.
- **Jupyter/evcxr Integration:** First-class support for interactive data science in Rust notebooks.

🏗️ **Architecture & Performance**

Charton is built to be a "Visualization Infrastructure":
1. **Core Engine:** Handles statistical transforms (binning, loess, etc.) and encoding logic.
2. **IPC Module:** Efficiently pipes data to Python if you need specific Altair features, returning PNG/SVG/JSON.
3. **Frontend Ready:** It can output standard **Vega-Lite JSON**, making it trivial to embed charts in React/Vue apps using `vega-embed`.

🛠️ **Usage Example (Layered Chart)**
```rust
let line = Chart::build(&df)?.mark_line().encode((x("x"), y("y")))?;
let scatter = Chart::build(&df)?.mark_point().encode((x("x"), y("y")))?;

LayeredChart::new()
    .add_layer(line)
    .add_layer(scatter)
    .show()?; // Renders inline in Jupyter!
```

I’d love to get your feedback! Whether you are a data scientist moving to Rust or a systems engineer needing quick dashboards, I hope Charton makes your life easier.

**Check out the [Examples](https://github.com/wangjiawen2013/charton/tree/main/examples) folder in the repo for more!**




The first comment to reddit:
**Why I built Charton:**

I’ve been working in the Rust data ecosystem for a while, and like many of you, I’ve often felt the "visualization gap." Here’s why I decided to build another plotting library:

**Plotters is too low-level:** While `plotters` is incredibly powerful, it often feels like drawing on a canvas rather than analyzing data. I wanted something where I could describe what to plot, not how to draw every line and pixel.

**The "Native" Problem:** Many existing solutions like `plotly` or `charming` are essentially wrappers around JavaScript libraries. They are great for the web, but they don't feel "Rust-native." I wanted a library that talks directly to **Polars** and leverages Rust’s type system without being a black box for JS.

**API Complexity:** Many Vega-lite implementations in Rust are either too verbose or strictly follow a JSON-like structure that feels clunky in IDEs. Charton aims for an **Altair-inspired API**—concise, chainable, and intuitive.

**Maintenance Concerns:** Let’s be honest—several promising Rust plotting crates haven't seen an update in years. I built Charton to be a modern, actively maintained alternative specifically optimized for the current `Polars` (0.49+) ecosystem and `Wasm` requirements.

**Current Status & Goals:** Charton is currently in its early stages. It has a pure-Rust SVG renderer, but also allows you to "drop down" to Altair/Matplotlib if you need a feature that isn't native yet.

I’d love to hear your thoughts on the API. What’s the biggest "missing piece" in your Rust data workflow?
