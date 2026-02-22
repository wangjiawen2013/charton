# External Backend Integration
Charton can render charts using native Rust rendering, but it also integrates seamlessly with external visualization backends such as Altair  and Matplotlib.  

This chapter explains how backend switching works and why external backends can be useful for leveraging established visualization ecosystems. You will also learn how to run raw Python plotting code from Rust, allowing complete flexibility.

This is especially useful when mixing Rust data pipelines with existing Python workflows.

## Why external backends?
Rust visualization ecosystem — including Charton — is still relatively young, it may not always meet all user requirements. In contrast, other languages have mature and feature-rich visualization tools, such as Altair and Matplotlib. Therefore, in situations where Charton’s native capabilities are not sufficient, it is necessary to rely on these external visualization tools as complementary backends.

## Altair backend
Charton provides first-class integration with the Altair visualization ecosystem through the Altair backend. This backend allows Rust programs to generate Altair charts, render them using Python, and output either SVG images or Vega-Lite JSON specifications. This enables seamless interoperability between Rust data pipelines and any existing Python-based visualization workflow.

Internally, Charton sends data to Python using an IPC (Apache Arrow) buffer, executes user-provided Altair code, and returns either SVG or Vega-Lite JSON back to Rust.

**Requirements**

Before using the Altair backend, ensure Python and required packages are installed:
```shell
pip install altair vl-convert-python polars pyarrow
```
### Loading the Example Dataset (`mtcars`)
Below is a built-in function to load `mtcars` into a Polars `DataFrame`:
```rust
let df = load_dataset("mtcars")?;
```

### Basic Usage: Executing Altair Code
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
### Saving Altair Charts as SVG
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
### Export as Vega-Lite JSON
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
### Example: Converting to Vega-Lite JSON and Rendering in the Browser
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

### Full Example: A More Complete Altair Chart
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
## Matplotlib backend
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
### Basic Usage: Executing Matplotlib Code
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
### Saving Matplotlib Output as SVG
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

### Using Subplots
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
### Adding Titles, Legends, and Styles
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

### Using Python Libraries with Matplotlib (not tested)
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

### Full Example: Multi-encoding Scatterplot
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
