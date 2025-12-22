**Subreddit**: `r/rust` or `r/dataisbeautiful` **Title:** [Show & Tell] Charton: A Polars-native, Altair-style Declarative Plotting Library for Rust

**Body:**

Hi everyone,

I’m excited to announce the first public release of **Charton**, a plotting library designed to bring the ergonomics of Python’s **Altair/Vega-Lite** to the Rust and **Polars** ecosystem.

**GitHub:** [Charton](https://github.com/wangjiawen2013/charton) Crates.io: `charton = "0.2.0"`

🦀 **Why another plotting library?**

We have great low-level tools like `plotters`, but for exploratory data analysis (EDA), we often miss the declarative "Grammar of Graphics" approach. Charton aims to bridge the gap between high-performance data processing in Polars and beautiful, rapid visualization.

✨ **Key Features:**
- **Polars-First:** Deeply integrated with Polars. Plot directly from your DataFrames without cumbersome conversions.
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
🛣️ **What's next?**
- Full WASM testing (currently partial support).
- LazyFrame optimization for extremely large datasets.
- More statistical marks (Violin plots, Beeswarm, etc.).

I’d love to get your feedback! Whether you are a data scientist moving to Rust or a systems engineer needing quick dashboards, I hope Charton makes your life easier.

**Check out the [Examples](https://github.com/wangjiawen2013/charton/tree/main/examples) folder in the repo for more!**