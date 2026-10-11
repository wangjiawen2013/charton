# The Nushell Plugin

Nushell treats every pipeline as a table (`list<record>`), which makes it a
natural front-end for a grammar-of-graphics library: instead of serializing data
to CSV and re-loading it into a plotting tool, you can pipe a table straight
into a chart. `nu_plugin_charton` is the bridge that turns Charton into a
first-class citizen of the shell.

```nu
open sales.csv | charton bar -x region -y revenue
```

That single line reads a file, infers column types, builds a Charton `Dataset`,
renders a bar chart, and draws it inline in the terminal.

![charton drawing a bubble chart inline in the terminal](../images/nushell-demo.gif)

> This chapter is the conceptual overview. The complete flag/config reference
> lives in the plugin's own
> [`README`](https://github.com/wangjiawen2013/charton/blob/main/nu_plugin_charton/README.md).
> The non-obvious engineering — process model, inline protocols, resolution
> fitting, protocol versioning — is covered in
> [Nushell Plugin Internals](nushell_internals.md).

## Why a plugin, not a binary

A Nushell *plugin* is a long-lived child process that speaks Nushell's plugin
protocol (MessagePack over stdio or a local socket). Packaging Charton as a
plugin rather than a standalone CLI buys three things:

- **No serialization round-trip.** Data arrives as Nushell `Value`s and is
  converted directly into a Charton `Dataset`. There is no CSV/JSON detour and
  no second parse.
- **Composability.** The command can hand values *back* to the pipeline
  (`--raw`), so a chart is just another stage:
  `... | charton point -x a -y b --raw | save chart.svg`.
- **Terminal-native output.** The plugin inherits the shell's `stdout`, so it
  can draw the chart *inline* in the terminal — the shell equivalent of
  `plt.show()`.

## The data path

```text
Nushell Value (list<record>)
        │  converter.rs  (per-column type inference over all rows)
        ▼
      Table
        │  command.rs    (mark subcommand → mark, --transform, --layer, facets)
        ▼
   Charton Dataset / LayeredChart
        │  render.rs
        ├─► SVG string      (vector — export / pipe)
        └─► PNG bytes       (raster — inline / export)
                │  render.rs
                ▼
     Terminal escape sequence (Kitty / iTerm2 / Sixel / half-block)
```

Type inference is done over **all rows**, not just the first: a column that
starts with integers but contains a decimal later is promoted to float, and a
column containing nulls/`NaN`s stays null-aware. This mirrors the integrity
rules described in [The Dataset Struct](../engine/dataset_core.md).

## Quick start

### Install

```sh
cargo install --path nu_plugin_charton --locked
```

```nu
plugin add '~/.cargo/bin/nu_plugin_charton.exe'   # Windows: full path is most reliable
plugin use charton
```

The full, platform-specific walk-through (including uninstall) is in the
[plugin README](https://github.com/wangjiawen2013/charton/blob/main/nu_plugin_charton/README.md#install).

### Everyday use

```nu
# inline, auto-detected terminal protocol
ls | charton bar -x name -y size

# export by extension: SVG is a vector, PNG is a raster
open data.csv | charton line -x date -y value -o chart.svg
open data.csv | charton point -x group -y score --layout beeswarm -o chart.png

# violin: a density outline with an inner quartile box; -c dodges one per group
open data.csv | charton violin -x category -y score -c group -o violin.svg

# contour: iso-lines of a regular x/y/z grid; --z is the value column
open grid.csv | charton contour -x x -y y --z z -o contour.svg

# group by a column with a color channel, then overlay a second mark
open data.csv | charton line -x t -y v -c series --layer point

# recipes are compositions, not marks: --transform runs Charton's public
# transforms in order and the mark draws the columns they emit
# ridgeline = density + one-sided band + polygon
let ridge = [
    {op: density, field: score, as: [value, density], group-by: [category], trim: true}
    {op: band, value: value, width: density, center: category, side: right,
     scale: per-group, overlap: 2.5, as: [x, y, pg]}
]
open assets/data.csv
| charton polygon -x x -y y --path-group pg -c category --flip --transform $ridge -o ridge.svg
# ECDF = the CumeDist window transform + a step line
let ecdf = {op: window, field: score, kind: cume-dist, as: ecdf, group-by: [group]}
open assets/data.csv
| charton line -x score -y ecdf -c group --interpolation step --transform $ecdf -o ecdf.svg
# candlestick = a low--high rule + a floating bar
let body = {geom: bar, y: open, y2: close}
open assets/ohlc.csv
| charton rule -x date -y low --y2 high -c direction --layer $body --x-angle -45 -o candles.svg
# dumbbell = a rule with a dot at each end
let ends = [{geom: point, y: y2011}, {geom: point, y: y2021}]
open assets/dumbbell.csv
| charton rule -x country -y y2011 --y2 y2021 --flip --layer $ends -o dumbbell.svg
# range = a single rule; slope = a line with its points
open assets/range.csv | charton rule -x month -y low --y2 high --flip -o range.svg
open assets/slope.csv | charton line -x period -y rate -c country --layer point -o slope.svg

# geographic choropleth from GeoJSON
charton geo --geojson world.geojson -c POP_EST -o world.png
```

The plugin is built around four levels, from "just draw my rows" to writing a
full recipe: **marks** (geometry primitives and statistical atoms),
**composition** (`--layer`), **transforms** (`--transform`), and both together.
The README's [four-level guide](https://github.com/wangjiawen2013/charton/blob/main/nu_plugin_charton/README.md#the-four-levels)
walks through them.

The mark subcommands cover every Charton mark (`charton point`, `charton line`,
`charton path`, `charton polygon`, `charton area`, `charton bar`,
`charton boxplot`, `charton violin`, `charton errorbar`, `charton rule`,
`charton tick`, `charton text`, `charton rect`, `charton hist`,
`charton density`, `charton density-2d`, `charton contour`, `charton geo`).
Chart types that Charton builds from primitives — ridgeline, ECDF, candlestick,
dumbbell, range, slope, bump, waterfall, raincloud — are **not** commands: you
write them with `--transform` and `--layer`, exactly as the
[cookbook](../gallery/primitives.md) shows. The `--transform` vocabulary is
closed: `density`, `density-2d`, `contour`, `band`, `quantile-box`, `window`,
`calculate`. `calculate` runs a Nushell closure per row (`{op: calculate,
as: ratio, expr: {|r| $r.a / $r.b}}`), so it can read an earlier transform's
output; a plain `insert` before `charton` is faster when the value only needs the
original columns.
The full flag table is in the plugin README.

## Configuration

Defaults live in `$env.config.plugins.charton`. Command-line flags win over
config, and config wins over built-in defaults:

```nu
$env.config.plugins.charton = {
    width: 1000
    height: 700
    scale: 2.0
    cell_width: 9                  # terminal cell size in device px
    cell_height: 20                # (tune if inline charts look soft)
    inline_style: kitty            # auto | halfblock | iterm2 | kitty | sixel
    grid: true
    palette: tab10
    legend: bottom
    x_angle: -45
    background: "#ffffff"
}
```

`cell_width`/`cell_height` are the one non-obvious knob; they are explained in
[Nushell Plugin Internals](nushell_internals.md).
