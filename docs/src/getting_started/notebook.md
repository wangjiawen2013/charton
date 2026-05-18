# Interactive Notebooks

In modern data engineering and scientific exploration, immediate feedback loops are essential. Charton natively implements runtime hooks for the Rust Jupyter kernel (`evcxr_jupyter`). By replacing disk-bound `.save()` sequences with the specialized `.show()` terminal API, you can render high-fidelity, inline SVG vector visuals instantly within individual notebook cells.

## Prerequisites

Before initiating plotting routines inside a notebook, ensure the underlying EVCXR kernel is initialized globally within your local system architecture. See the [Jupyter/evcxr article](https://depth-first.com/articles/2020/09/21/interactive-rust-in-a-repl-and-jupyter-notebook-with-evcxr).

## Notebook Inline Execution Blueprint

Create a fresh cell inside your Jupyter Notebook using the Rust (evcxr) kernel, and input the following configuration:

```rust
:dep charton = { version = "0.5" }
:dep polars = { version = "0.53", features = ["lazy"] }

use charton::prelude::*;
use polars::prelude::*;

// 1. Initialize evaluation dataframe
let df = df!["x" => [1, 2, 3], "y" => [10, 20, 30]].unwrap();
let ds = load_polars_df!(df).unwrap();

// 2. Compose the declarative graph layer
let chart = Chart::build(ds).unwrap()
    .mark_point().unwrap()
    .encode((alt::x("x"), alt::y("y"))).unwrap();

// 3. Execute inline visualization: renders high-fidelity vector graphics right into the notebook cell
chart.show().unwrap();
```

## Deep Dive: Seamless Environment Self-Adaptation
The `.show()` method can seamlessly adapt its output behavior whether it is executed inside a live, interactive Jupyter workspace or run within a traditional console application. This resilience is achieved via Charton’s internal runtime environment probing mechanism.

Take a look at how `.show()` is engineered under the hood:

```rust
pub fn show(&self) -> Result<(), ChartonError> {
    // 1. Core Execution: Serialize the in-memory chart nodes and abstract layers into an SVG string
    let svg_content = self.to_svg()?;

    // 2. Probing Environment: Query the active process context for the EVCXR runtime signature
    if std::env::var("EVCXR_IS_RUNTIME").is_ok() {
        // 3. Protocol Handshake: When explicitly run within a Jupyter container,
        // intercept standard output and stream the tailored HTML payload wrapper
        println!(
            "EVCXR_BEGIN_CONTENT text/html\n{}\nEVCXR_END_CONTENT",
            svg_content
        );
    }

    // 4. Fallback Boundary Safety: If triggered within a regular CLI, microservice, 
    // or CI/CD test harness, this method safely concludes with an Ok(()) without
    // polluting standard error or panicking.
    Ok(())
}
```