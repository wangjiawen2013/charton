# nu_plugin_charton

Turn any Nushell pipeline table into a chart — inline in the terminal, or saved
as SVG/PNG. Built on the [charton](https://github.com/wangjiawen2013/charton)
plotting library.

## Demo

![charton drawing a bubble chart inline in the terminal](assets/nushell-demo.gif)

```nu
open assets/demo.csv | charton -g point -x x -y y -c group --size-by size --theme dark
```

That one line reads the table, infers the column types, builds the chart and
draws it in the terminal. The Nushell command is `charton` (the plugin is
registered under the name without the `nu_plugin_` prefix).

> **Compatibility:** `nu_plugin_charton` targets **Nushell 0.116** and shares
> the `charton` version number (both `0.7.x`), so one release covers the
> matching pair. See [Compatibility](#compatibility).

## Install

```sh
cargo install nu_plugin_charton
```

This puts the `nu_plugin_charton` executable in `~/.cargo/bin`, which `rustup`
already adds to your `PATH`.

### Register with Nushell

`plugin add` records the plugin in Nushell's registry. Pass the **executable**
name or path, not the `charton` command name:

```nu
plugin add nu_plugin_charton
```

On Windows, the full path is the most reliable form:

```nu
plugin add 'C:/Users/you/.cargo/bin/nu_plugin_charton.exe'
```

### Load it

In Nushell 0.116 a registered plugin is not loaded automatically; its commands
enter scope after `plugin use`:

```nu
plugin use charton
```

To load it in every session, add that line to your `config.nu`. `plugin use` is
a parser keyword, so it must be on its own line and cannot share a script with
the `plugin add` that registers the plugin.

### Verify

```nu
plugin list | where name == charton
[[g, v]; [a, 1] [b, 2]] | charton -g bar -x g -y v
```

In an interactive terminal the last command draws the chart inline; in a
non-interactive context (`nu -c`, output redirected) it returns the SVG text
instead.

## Quick start

```nu
# Inline in the terminal (Kitty / iTerm2 / Sixel / truecolor half-blocks)
ls | charton -g bar -x name -y size

# Save to a file — format comes from the extension (.svg or .png)
open assets/data.csv | charton -g line -x date -y value -o chart.svg
open assets/data.csv | charton -g beeswarm -x group -y score -o chart.png

# Return the image to the pipeline instead of drawing it
open assets/data.csv | charton -g point -x a -y b --raw | save chart.svg
open assets/data.csv | charton -g point -x a -y b --raw --png | save chart.png

# Group by a column with color, stack, and aggregate
open assets/sales.csv | charton -g bar -x region -y revenue -c quarter --stack stacked
open assets/sales.csv | charton -g bar -x region -y revenue --aggregate mean

# Distribution and trend
open assets/data.csv | charton -g density -x score -c group
open assets/data.csv | charton -g line -x t -y v --loess

# Pie / donut (bar mark in polar coordinates)
open assets/data.csv | charton -g bar -x category -y amount --coord polar
open assets/data.csv | charton -g bar -x category -y amount --coord polar --inner-radius 0.5

# Geographic choropleth from a GeoJSON file
charton -g geo --geojson assets/world.geojson -c POP_EST -o world.png
```

The examples read the small sample files shipped in `assets/`: `data.csv` has
columns `date,value,group,score,a,b,t,v,category,amount`, `sales.csv` has
`region,quarter,revenue`, and `world.geojson` is a Natural Earth country map.
Run the commands from the `nu_plugin_charton/` directory (or prefix `assets/`
with its path), and swap in your own file and column names to chart your data.

## Output modes

| You write | Where the result goes |
|---|---|
| `charton ...` | Drawn inline when the terminal supports it; otherwise an SVG string is returned to the pipeline |
| `charton ... -o chart.svg` / `.png` | Written to that file (can be combined with inline drawing) |
| `charton ... --raw` | The SVG text is returned instead of drawing |
| `charton ... --raw --png` | PNG bytes are returned instead of drawing |
| `charton ... --no-inline` | Never draws inline |

Output format is chosen by the `-o` extension, case-insensitively; only `.svg`
and `.png` are accepted. For the inline drawing path, see
[Inline rendering](#inline-rendering).

## Usage reference

Flags are grouped by what you want to do. `[]` in the type column marks a list.

### Chart type and data

| Flag | Meaning |
|---|---|
| `-g, --geom` | Chart type: `point` (default) \| `line` \| `area` \| `bar` \| `boxplot` \| `errorbar` \| `rule` \| `tick` \| `text` \| `rect`/`heatmap` \| `hist` \| `density`/`kde` \| `ecdf` \| `beeswarm` \| `geo` |
| `-x, --x` | Column for the x axis (the value column for `-g density`/`-g ecdf`) |
| `-y, --y` | Column for the y axis (`hist` uses a generated `count`; `density` uses a generated `density`) |
| `-c, --color` | Column mapped to color / grouping (required for `rect`; the group column for `density`/`ecdf`) |
| `--y2` | Upper-bound column for `errorbar`/`rule` (errorbar aggregates mean ± std when omitted) |
| `--text` | Label column for `-g text` |
| `--geojson` | GeoJSON file to render with `-g geo` |

`beeswarm` is the `point` mark with a beeswarm layout, not a separate mark.
`scatter`, `box`, `label`, `heatmap`, `histogram`, and `geoshape` are aliases.

### Encodings

| Flag | Meaning |
|---|---|
| `--size-by` | Column mapped to point size (bubble charts; `-g point`/`beeswarm`) |
| `--shape-by` | Column mapped to point shape (`-g point`/`beeswarm`) |
| `--size-label`, `--shape-label` | Legend labels for the size / shape scales |

### Scales and axes

| Flag | Meaning |
|---|---|
| `--x-scale`, `--y-scale` | Axis scale: `linear` \| `log` \| `discrete` \| `temporal` |
| `--x-min`, `--x-max`, `--y-min`, `--y-max` | Fix an axis bound (min and max must be given together) |
| `--x-expand`, `--y-expand` | Padding added to both ends, as a fraction of the data range |
| `--x-ticks`, `--y-ticks` | Explicit tick values, e.g. `[0 2 4 6]` |
| `--x-format`, `--y-format`, `--legend-format` | Tick/legend label format: the preset `compact`, or a record `{prefix, suffix, precision, compact, thousands, multiplier}` |
| `--x-label`, `--y-label`, `--color-label` | Axis and legend titles |
| `--x-angle` | X tick label angle in degrees |
| `--flip` | Swap the x and y axes |
| `--bins` | Number of bins for a continuous x axis |
| `--aggregate` | Aggregate y per x group: `sum` \| `mean` \| `median` \| `min` \| `max` \| `count` |

### Data transforms

| Flag | Meaning |
|---|---|
| `--stack` | Bar/area stacking: `none` \| `stacked` \| `normalize` \| `center` |
| `--normalize` | Normalize y values (histogram counts / bar values) to proportions |
| `--loess`, `--loess-bandwidth` | Smooth a `-g line` with LOESS (bandwidth 0.0–1.0) |
| `--density-bandwidth`, `--density-kernel` | KDE options for `-g density`: bandwidth in data units, kernel `normal` \| `epanechnikov` \| `uniform` |
| `--cumulative`, `--counts` | `-g density`: cumulative density, or smoothed counts instead of probabilities |

### Layout and composition

| Flag | Meaning |
|---|---|
| `--layer` | Overlay layer(s): a record or list of records with `geom`/`x`/`y`/`y2`/`color`/`text`/`size_by`/`shape_by`, plus optional `style`, `stack`, `aggregate`, `bins`, `x_scale`/`y_scale` |
| `--facet-wrap` | Wrap panels by this column |
| `--facet-columns` | Number of columns for `--facet-wrap` |
| `--facet-row`, `--facet-col` | Two-field facet grid |
| `--facet-strategy` | `fixed` (default) \| `free` |
| `--coord` | Coordinate system: `cartesian` (default) \| `polar` (geographic charts use `-g geo`) |
| `--inner-radius` | Polar inner radius ratio 0.0–1.0 (donut charts) |
| `--start-angle`, `--end-angle` | Polar angular span in degrees (rose / nightingale charts) |
| `--margins` | Canvas margins as `top,right,bottom,left` (fractions 0.0–1.0) |

### Appearance

| Flag | Meaning |
|---|---|
| `-t, --title` | Chart title |
| `--theme` | Color theme: `auto` (default) \| `light` \| `dark` |
| `--background` | Chart background color (overrides `--theme`) |
| `--color-map` | Continuous color map for `rect`/heatmap, e.g. `viridis`, `magma`, `ylgnbu` |
| `--legend` | Legend position: `left` \| `right` \| `top` \| `bottom` \| `none` |
| `--grid`, `--no-grid` | Force grid lines on / off |
| `--width`, `--height` | Pixel canvas size (default 800×600) |

### Mark styling

Mark-level options apply to every layer whose mark supports them; unsupported
combinations are ignored (for example `--size` does nothing on a bar).

| Flag | Meaning |
|---|---|
| `--mark-color`, `--opacity`, `--size`, `--stroke`, `--stroke-width` | Color, opacity, size, stroke color and width |
| `--mark-width` | Band width as a fraction 0.0–1.0 (bar / box / point / errorbar) |
| `--shape` | Point shape: `circle` \| `square` \| `triangle` \| `star` \| `diamond` \| `pentagon` \| `hexagon` \| `octagon` |
| `--layout`, `--quasirandom-method` | Point layout (`standard`/`jitter`/`beeswarm`/`quasirandom`) and its pairing method |
| `--dash` | Line dash pattern, e.g. `6,4` |
| `--interpolation` | Line interpolation: `linear` \| `step` \| `step-before` |
| `--outlier-color`, `--no-outliers`, `--outlier-size` | Boxplot outliers |
| `--cap-length`, `--no-center` | Errorbar cap length and center dot |
| `--anchor`, `--weight` | Text anchor (`start`/`middle`/`end`) and font weight (`normal`/`bold`/`100..900`) |

### Output and inline rendering

| Flag | Meaning |
|---|---|
| `-o, --output` | Save to `.svg` or `.png` (relative paths resolve against the shell's working directory) |
| `--raw` | Return the image instead of drawing it inline |
| `--png` | With `--raw`, return PNG bytes instead of an SVG string |
| `--scale` | Raster pixel scale factor (default 2.0; also disables inline auto-fit) |
| `--inline-style` | Inline renderer: `auto` (default) \| `halfblock` \| `iterm2` \| `kitty` \| `sixel` |
| `--no-inline` | Never draw inline |
| `--force-inline` | Draw inline even when stdout is not detected as a TTY (testing) |

## Configuration

Set defaults in `$env.config.plugins.charton`. Command-line flags take
precedence; config beats built-in defaults. Unknown keys are ignored with a
warning on stderr, so typos surface instead of silently doing nothing.

```nu
$env.config.plugins.charton = {
    width: 1000
    height: 700
    scale: 2.0
    cell_width: 9                  # terminal cell size in device px
    cell_height: 20                # (tune these if inline charts look soft)
    inline_style: kitty            # auto | halfblock | iterm2 | kitty | sixel
    grid: true
    palette: tab10                 # or ["#333" "#6fc481" "red"]
    legend: bottom                 # left | right | top | bottom | none
    x_angle: -45
    background: "#ffffff"
    theme: dark                    # auto | light | dark
    color_map: viridis             # continuous color scheme
}
```

| Config key | Type | Purpose |
|---|---|---|
| `width`, `height` | int | Default pixel canvas |
| `scale` | float | Raster scale factor (also disables inline auto-fit) |
| `cell_width`, `cell_height` | int | Assumed terminal cell size in device px; used to fit inline images |
| `inline_style` | string | Default inline renderer |
| `grid` | bool | Show grid lines |
| `palette` | string \| list | Named palette (`tab10`…`accent`) or explicit colors |
| `legend` | string | `left`/`right`/`top`/`bottom`/`none` |
| `x_angle` | float | X tick label angle |
| `background` | string | Chart background color |
| `theme` | string | Color theme: `auto`/`light`/`dark` |
| `color_map` | string | Continuous color map, e.g. `viridis`/`magma`/`ylgnbu` |

## Inline rendering

In an interactive terminal the plugin draws the chart itself, most-detailed
renderer first, chosen automatically from the environment:

| Tier | Protocol | Terminals |
|---|---|---|
| 1 | **Kitty graphics** | Kitty, Ghostty, recent WezTerm |
| 2 | **iTerm2 inline images** (`OSC 1337`) | iTerm2, WezTerm |
| 3 | **Sixel** | xterm, foot, mlterm, recent Windows Terminal |
| 4 | **Truecolor half-blocks** (`▀`) | everywhere |

Tiers 1–3 hand a real PNG bitmap to the terminal, so text and thin axis lines
stay sharp. Tier 4 is the universal fallback: it packs the bitmap into colored
`▀` characters and works on any UTF-8 terminal. Override the choice with
`--inline-style`, the `CHARTON_INLINE_STYLE` environment variable, or the
`inline_style` config key.

Inline drawing only happens when the plugin is **not** in stdio mode and its
`stdout` is a TTY; otherwise the command falls back to `-o` saving, `--raw`
data, or an SVG string. The full rationale (process model, resolution fitting,
protocol versioning) is in the book chapter
[Nushell Plugin Internals](https://wangjiawen2013.github.io/charton/ecosystem/nushell_internals.html).

### Over SSH

Terminal identity variables (`TERM_PROGRAM`, `WEZTERM_*`, `KITTY_*`, ...) are
not forwarded by SSH by default, but `TERM` always is, so `auto` reads `TERM`
too. If your client is still not detected — for example WezTerm configured with
`term = "xterm-256color"`, which looks like a plain `xterm` to the server —
force the protocol:

```nu
$env.CHARTON_INLINE_STYLE = "iterm2"             # WezTerm / iTerm2
$env.config.plugins.charton = { inline_style: kitty }
```

## Troubleshooting

| Symptom | Try |
|---|---|
| Text and axes render as small colored blocks | The half-block fallback is in use. Force an image protocol: `--inline-style iterm2` (WezTerm/iTerm2), `kitty`, or `sixel`. |
| Nothing inline, just an SVG string | stdout is not a TTY (piping/`nu -c`), or the plugin is in stdio mode. Use `-o` or `--raw`. |
| Charts look soft or stretched | Tune `cell_width`/`cell_height` in the config, or pass an explicit `--scale`. |
| Plugin command not found after a rebuild | Re-register (`plugin add <path>`) and restart, or `plugin rm charton` then add it again. |

The `charton-probe` command reports the transport, terminal capabilities and the
environment variables used for protocol detection. It is a development tool,
built only with `--features probe`:

```sh
cargo build -p nu_plugin_charton --release --features probe
```

```nu
charton-probe | to md
```

## Compatibility

`nu-plugin` / `nu-protocol` **must match the installed Nushell version**; this
crate is pinned to `=0.116`, so it targets **Nushell 0.116**. After upgrading
Nushell, install the matching plugin release (or rebuild) and re-run
`plugin add`.

`nu_plugin_charton` shares its version number with the `charton` library it
depends on, so `charton 0.7.1` and `nu_plugin_charton 0.7.1` ship as a pair.
The Nushell target is stated here and in the release notes.

## Uninstall

```nu
plugin rm charton
```

Also delete any `plugin use charton` line you added to `config.nu`. Commands
already loaded stay in scope until the session ends, so restart Nushell to
unload fully. If you installed the binary with `cargo install`, remove it too:

```sh
cargo uninstall nu_plugin_charton
```

`plugin rm` does not touch `$env.config.plugins.charton` settings in `config.nu`;
delete those yourself if you no longer want them.

## Learn more

- [The Nushell Plugin](https://wangjiawen2013.github.io/charton/ecosystem/nushell.html)
  — the conceptual overview.
- [Nushell Plugin Internals](https://wangjiawen2013.github.io/charton/ecosystem/nushell_internals.html)
  — process model, inline protocols, resolution fitting.
- [charton library docs](https://docs.rs/charton) — the plotting engine behind
  this plugin.

## Status

- [x] `Value` table → charton `Dataset` converter (per-column type inference
      over all rows; int/float/string/bool/datetime, null-aware)
- [x] all charton marks: `point`, `line`, `area`, `bar`, `boxplot`,
      `errorbar`, `rule`, `tick`, `text`, `rect`/`heatmap`, `hist`,
      `density`/`kde`, `ecdf`, `beeswarm`, `geo`
- [x] multi-layer overlays, faceting, polar and geographic coordinates
- [x] stacking, aggregation, binning, LOESS, KDE
- [x] axis scales, domains, explicit ticks, label formatting, light/dark themes
- [x] `size`/`shape` encoding channels
- [x] inline terminal rendering: Kitty / iTerm2 / Sixel / half-block
- [x] SVG / PNG export and `--raw` piping
- [x] configuration via `$env.config.plugins.charton`
- [x] unit tests via `nu-plugin-test-support`

Planned: companion label/annotation marks, a native ANSI terminal backend that
draws with braille and text rather than a raster image.

## Development

This crate is a member of the `charton` workspace. `default-members = ["."]`, so
a plain `cargo build` at the repo root builds only the core library and skips
the heavy `nu-plugin`/`nu-protocol` tree.

```sh
cargo build -p nu_plugin_charton --release
cargo test  -p nu_plugin_charton
# Include the `charton-probe` diagnostic command:
cargo build -p nu_plugin_charton --release --features probe
```

## License

Apache-2.0. See [LICENSE](https://github.com/wangjiawen2013/charton/blob/main/LICENSE).
