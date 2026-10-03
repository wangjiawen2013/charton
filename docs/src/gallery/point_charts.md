# Scatter

`mark_point` is the base point geometry; shape, size and colour are ordinary
channels.

## Shape, size and a reference grid

<img src="../images/grid_line.svg" width="500">

The grid is a **theme** feature (`with_grid(true)`), not a mark — turn it on for
any chart.

```rust
{{#include ../../../examples/grid_line.rs}}
```

## Point layouts

When points overlap, a layout spreads them without changing their x position:

- **Jitter** — small random offsets; `tests/test_scatter.rs`.
- **Beeswarm / quasirandom** — deterministic packing; see
  [Distributions](statistics.md#beeswarm-and-quasirandom).
