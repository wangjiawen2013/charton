### True interactive visualization via the Altair backend
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

### Exporting Vega-Lite JSON for browser/Web app usage
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

The generated Vega-Lite JSON specification will look like this:
```json
{
  "$schema": "https://vega.github.io/schema/vega-lite/v5.20.1.json",
  "data": {
    "name": "data-8572dbb2f2fe2e54e92fc99f68a5f076"
  },
  "datasets": {
    "data-8572dbb2f2fe2e54e92fc99f68a5f076": [
      {
        "Discount": 0.65,
        "Model": "S1",
        "Price": 2430
      },
      // ... more data rows ...
    ]
  },
  "encoding": {
    "color": {
      "field": "Model",
      "type": "nominal"
    },
    "x": {
      "field": "Price",
      "type": "quantitative"
    },
    // ... other encoding and properties ...
  },
  "mark": {
    "type": "point"
  }
}
```

**Embedding in a webpage**:

To render the visualization, simply embed the generated JSON into your HTML using the `vega-embed` library:

```html
<div id="vis"></div>
<script>
  var spec = /* paste JSON here */;
  vegaEmbed('#vis', spec);
</script>
```

### Summary: Hybrid Power
By leveraging Altair as a backend, **Charton** offers a unique "hybrid" workflow that combines the best of two worlds:

1. **Rust Efficiency**: Handle heavy data crunching and complex Polars transformations with type safety and maximum performance.
2. **Python Ecosystem**: Access the vast, mature visualization capabilities of Altair/Vega-Lite without leaving your Rust development environment.

Whether you are performing rapid **Exploratory Data Analysis** in a Jupyter notebook or shipping high-fidelity **interactive dashboards** to a web frontend, this bridge ensures you never have to choose between performance and features.