# Area Charts

An area fills the region between a line and its baseline. Adding a colour field
stacks the series; changing the stack baseline yields the streamgraph.

Use `mark_area` when the region is defined by a series and a baseline. Use
`mark_polygon` when you have an explicit list of ring vertices (a band, a map,
a violin outline); an area is ordered and baseline-oriented, while a polygon is
arbitrary row-order geometry. Use `mark_line` when you only want the boundary,
not the filled interior.

## Area chart

<img src="../images/area.svg" width="500">

```rust
{{#include ../../../examples/area.rs}}
```

## Gaps

A missing `x` or `y` opens the area at that point instead of bridging it. See
[Missing Values & Gaps](../concepts/missing_values.md).

## Simple Stacked Area Chart
Adding a color field to area chart creates stacked area chart by default. For example, here we split the area chart by country by setting `stack` to `"stacked"`.

```rust
{{#include ../../../examples/simple_stacked_area_chart.rs}}
```

<img src="../images/simple_stacked_area.svg" width="500">

## Normalized Stacked Area Chart
You can also create a normalized stacked area chart by setting `stack` to `"normalize"` in the encoding channel. Here we can easily see the percentage of unemployment across countries.

```rust
{{#include ../../../examples/normalized_stacked_area_chart.rs}}
```

<img src="../images/normalized_stacked_area.svg" width="500">

## Steamgraph
We can also shift the stacked area chart’s baseline to center and produces a streamgraph by setting `stack` to `"center"` in the encoding channel.

```rust
{{#include ../../../examples/steamgraph.rs}}
```

<img src="../images/steamgraph.svg" width="500">