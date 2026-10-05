# The Nushell Plugin

Nushell treats every pipeline as a table (`list<record>`), which makes it a
natural front-end for a grammar-of-graphics library: instead of serializing data
to CSV and re-loading it into a plotting tool, you can pipe a table straight
into a chart. `nu_plugin_charton` is the bridge that turns Charton into a
first-class citizen of the shell.

```nu
open sales.csv | charton -g bar -x region -y revenue
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
  `... | charton -g point -x a -y b --raw | save chart.svg`.
- **Terminal-native output.** The plugin inherits the shell's `stdout`, so it
  can draw the chart *inline* in the terminal — the shell equivalent of
  `plt.show()`.

## The data path

```text
Nushell Value (list<record>)
        │  converter.rs  (per-column type inference over all rows)
        ▼
      Table
        │  command.rs    (geom → mark, layers, facets, styling)
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
ls | charton -g bar -x name -y size

# export by extension: SVG is a vector, PNG is a raster
open data.csv | charton -g line -x date -y value -o chart.svg
open data.csv | charton -g beeswarm -x group -y score -o chart.png

# violin: a density outline with an inner quartile box; -c dodges one per group
open data.csv | charton -g violin -x category -y score -c group -o violin.svg

# contour: iso-lines of a regular x/y/z grid; --z is the value column
open grid.csv | charton -g contour -x x -y y --z z -o contour.svg

# group by a column with a color channel, then overlay a second mark
open data.csv | charton -g line -x t -y v -c series \
    --layer {geom: point}

# cookbook recipes, all compositions of the marks above
open data.csv | charton -g ridge -x category -y score -o ridge.svg
open data.csv | charton -g dumbbell -x category -y a --y2 b -o dumbbell.svg
open data.csv | charton -g waterfall -x category -y amount -o waterfall.svg
open ohlc.csv | charton -g candlestick -x date -y open --y2 close \
    --low low --high high -o candles.svg

# geographic choropleth from GeoJSON
charton -g geo --geojson world.geojson -c POP_EST -o world.png
```

The `-g` names cover every Charton mark (`point`, `line`, `area`, `bar`,
`boxplot`, `errorbar`, `rule`, `tick`, `text`, `rect`/`heatmap`, `hist`,
`density`/`kde`, `density_2d`, `ecdf`, `geo`) plus the composition names
`beeswarm`, `violin`, `contour`, `ridge`, `dumbbell`, `lollipop`, `range`,
`slope`, `bump`, `waterfall`, and `candlestick`; aliases such as `scatter`,
`label`, and `heatmap` map onto the marks. The recipes are the same
compositions as the [cookbook](../gallery/primitives.md) — no new marks. The
full flag table is in the plugin README.

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
