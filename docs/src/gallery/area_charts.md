Area represent multiple data element as a single area shape. Area marks are often used to show change over time, using either a single area or stacked areas.

### Simple Stacked Area Chart
Adding a color field to area chart creates stacked area chart by default. For example, here we split the area chart by country by setting `stack` to `"stacked"`.

```rust
{{#include ../../../examples/simple_stacked_area_chart.rs}}
```

<img src="../images/simple_stacked_area.svg" width="500">

### Normalized Stacked Area Chart
You can also create a normalized stacked area chart by setting `stack` to `"normalize"` in the encoding channel. Here we can easily see the percentage of unemployment across countries.

```rust
{{#include ../../../examples/normalized_stacked_area_chart.rs}}
```

<img src="../images/normalized_stacked_area.svg" width="500">

### Steamgraph
We can also shift the stacked area chart’s baseline to center and produces a streamgraph by setting `stack` to `"center"` in the encoding channel.

```rust
{{#include ../../../examples/steamgraph.rs}}
```

<img src="../images/steamgraph.svg" width="500">