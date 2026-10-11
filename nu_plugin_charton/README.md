# nu_plugin_charton

Turn any Nushell pipeline table into a chart — inline in the terminal, or saved
as SVG/PNG. Built on the [charton](https://github.com/wangjiawen2013/charton)
plotting library.

![charton drawing a bubble chart inline in the terminal](assets/nushell-demo.gif)

```nu
open assets/demo.csv | charton point -x x -y y -c group --size-by size --theme dark
```

That one line reads the table, infers column types, builds the chart, and draws
it in the terminal. The command is `charton`; the plugin is registered without
the `nu_plugin_` prefix.

> **Compatibility:** targets **Nushell 0.116** and shares the `charton` version
> number (both `0.8.x`), so one release covers the matching pair. See
> [Compatibility](#compatibility).

## Install

```sh
cargo install nu_plugin_charton
```

`plugin add` needs the path to the executable:

```nu
plugin add ~/.cargo/bin/nu_plugin_charton                        # Linux/macOS
plugin add 'C:/Users/you/.cargo/bin/nu_plugin_charton.exe'       # Windows
```

In Nushell 0.116 a registered plugin is not loaded automatically — its commands
enter scope after `plugin use`. Add that line to `config.nu` to load it every
session:

```nu
plugin use charton
```

`plugin use` is a parser keyword, so it must be on its own line and cannot share
a script with the `plugin add` that registers the plugin.

Verify:

```nu
plugin list | where name == charton
[[g, v]; [a, 1] [b, 2]] | charton bar -x g -y v
```

The last command draws inline in an interactive terminal; in a non-interactive
context (`nu -c`, redirected output) it returns the SVG text instead.

---

## The four levels

This plugin has a small vocabulary that recombines into every chart. There are
four levels, from "just draw my rows" to "write a recipe nobody named":

```text
 1. MARKS                2. COMPOSE              3. TRANSFORM            4. COMPOSE + TRANSFORM
 point, hist, …          --layer                 --transform             all three

 draw the rows      →    stack marks        →    rewrite the table  →    full recipes
 ─────────── basic ────────────      ───────────── advanced ─────────────────────────

 charton bar -x c -y v   charton line …           charton polygon …       charton polygon …
                         --layer point            --transform […]        --transform […]
```

| Level | You use | When you reach for it |
|---|---|---|
| **1. Marks** | `charton point`, `charton hist`, … | almost always — start here |
| **2. Compose** | `--layer` | one mark is not enough |
| **3. Transform** | `--transform` | no mark computes the numbers you need |
| **4. Compose + transform** | `--layer` + `--transform` | a complete recipe |

**Why it is built this way.** Chart *types* are infinite; *marks* and
*statistics* are finite. So the plugin exposes exactly Charton's public marks
and public transforms — no more — and gives you the two ways to combine them:
layering and transforms. A chart nobody named (ridgeline, candlestick,
waterfall, raincloud, …) is a Level 4 composition, **not a missing command**.
The trade-off is explicit: the first level is a one-liner, the last levels ask
you to know the pieces — but the vocabulary never grows, and a brand-new chart
type needs no plugin change.

Most charts are **Level 1**. Levels 2–4 exist so that "not a built-in chart
type" never means "not possible".

---

## Level 1 — Marks

A *mark* is the thing that draws. `charton <mark>` picks one; each command
declares only the flags it understands, so `help charton bar` and
`charton bar --<TAB>` stay focused. There are two kinds:

- **Primitives** draw the rows as they are: one row in, one shape out. `-x` /
  `-y` name existing columns.
- **Atoms** run one canonical statistic first, which **replaces** the table,
  then draw its output. `-y` is often a generated column (`count`, `density`, a
  five-number summary, …).

| Kind | Marks |
|---|---|
| Primitives | `point` `line` `path` `polygon` `bar` `area` `rect` `rule` `text` `tick` |
| Atoms | `hist` `boxplot` `violin` `errorbar` `density` `density-2d` `contour` |
| Geo | `geo` (Charton's `mark_geoshape`, fed by a GeoJSON file) |

This is exactly Charton's public mark list. `beeswarm` is
`point --layout beeswarm`, not a mark. The `--layer` / `--transform` records
also accept the short aliases `scatter`, `box`, `label`, `heatmap`,
`histogram`, `kde`, `density2d`, and `isoline`.

### Quick start

The mark is the subcommand and `-x` / `-y` name the columns. That is the whole
essentials:

```nu
# Draw inline (Kitty / iTerm2 / Sixel / truecolor half-blocks)
ls | charton bar -x name -y size

# Save instead — the format follows the extension
open assets/data.csv | charton line -x date -y value -o chart.svg
open assets/data.csv | charton point -x group -y score --layout beeswarm -o chart.png

# Or return the image to the pipeline
open assets/data.csv | charton point -x a -y b --raw | save chart.svg
open assets/data.csv | charton point -x a -y b --raw --png | save chart.png
```

Everything else is optional:

```nu
# Colour / group, stack, aggregate
open assets/sales.csv | charton bar -x region -y revenue -c quarter --stack stacked
open assets/sales.csv | charton bar -x region -y revenue --aggregate mean

# Atoms: a density curve, a violin (density + inner box), a histogram
open assets/data.csv | charton density -x score -c group --opacity 0.5
open assets/data.csv | charton violin -x category -y score -c group
open assets/grid.csv  | charton hist -x x --bins 20

# Contour (iso-lines) and a 2-D density heatmap
open assets/grid.csv | charton contour -x x -y y --z z -o contour.svg
open assets/data.csv | charton contour -x a -y b -o density_contour.svg   # no --z: density first
open assets/data.csv | charton density-2d -x a -y b -o density_2d.svg

# Polar: pie / donut (omit -x so y becomes the slices), rose (x is the angle)
open assets/data.csv | charton bar -y amount -c category --coord polar --inner-radius 0.5
open assets/data.csv | charton bar -x category -y amount -c category --coord polar

# Facets, and a geographic choropleth from GeoJSON
open assets/data.csv | charton point -x a -y b --facet-wrap group
charton geo --geojson assets/world.geojson -c POP_EST -o world.png
```

Every example reads a small CSV from `assets/`:

| File | Columns | Used by |
|---|---|---|
| `data.csv` | `date,value,group,score,a,b,t,v,category,amount` | the distribution marks and the recipes |
| `sales.csv` | `region,quarter,revenue` | grouped and stacked bars |
| `grid.csv` | `x,y,z` | `contour` / `hist` |
| `world.geojson` | — | `geo` |
| `dumbbell.csv` | `country,y2011,y2021` | the dumbbell recipe |
| `range.csv` | `month,low,high` | the range recipe |
| `slope.csv` | `country,period,rate` | the slope recipe |
| `lollipop.csv` | `language,share` | the lollipop recipe |
| `waterfall.csv` | `step,delta,kind` | the waterfall recipe |
| `ohlc.csv` | `date,open,high,low,close,direction` | the candlestick recipe |

Run the commands from the `nu_plugin_charton/` directory (or prefix `assets/`
with its full path) and swap in your own file and column names.

---

## Level 2 — Compose marks (`--layer`)

When one mark is not enough, stack another on top. `--layer` takes a mark name
(inheriting the primary encodings), a record, or a list. A layer writes only the
fields it changes.

```nu
# a line with its points on top
open assets/data.csv | charton line -x t -y v --layer point

# a rule with a dot at each end — a dumbbell (--flip makes it horizontal)
open assets/dumbbell.csv | charton rule -x country -y y2011 --y2 y2021 --flip \
    --layer [{geom: point, y: y2011}, {geom: point, y: y2021}]

# a low--high rule (wick) under a floating bar (body) — a candlestick
open assets/ohlc.csv | charton rule -x date -y low --y2 high -c direction \
    --layer {geom: bar, y: open, y2: close} --x-angle -45
```

More one-liners at this level:

```nu
# range: a single rule; slope: a line with its points; lollipop: a stem + dot
open assets/range.csv    | charton rule -x month -y low --y2 high --flip
open assets/slope.csv    | charton line -x period -y rate -c country --layer point
open assets/lollipop.csv | charton rule -x language -y share --flip --layer {geom: point, y: share}
```

This is the direct translation of Charton's `.and(other_chart)`. It is enough
for anything that is "the same rows, several shapes".

---

## Level 3 — Transform the table (`--transform`)

When no mark computes the numbers you need, rewrite the table yourself.
`--transform` runs Charton's public transforms in order **before** the mark
draws. Each transform **replaces** the table, so the mark's `-x` / `-y` /
`--path-group` name the columns the transform emits (its `as` names).

```nu
# density curve, written from the primitive pieces
open assets/data.csv | charton area -x value -y density \
    --transform {op: density, field: score, as: [value, density]}

# a running total
open assets/waterfall.csv | charton bar -x step -y cumulative -c kind \
    --transform {op: window, field: delta, kind: cumulative-sum, as: cumulative}
```

The vocabulary is **closed** — these are all the public transforms:

| `op` | Reads | Emits (`as`) | One row per |
|---|---|---|---|
| `density` | one numeric column | `[value, density]`; grouping kept | evaluation point × group |
| `density-2d` | `x`, `y` | `[x, y, density]` | grid node |
| `contour` | `x`, `y`, `z` | `[x, y, path_group]` + `level-as` | polyline vertex |
| `band` | `value`, `width` (+ `center`, `group`) | `[x, y, path_group]` | polygon vertex |
| `quantile-box` | `value` (+ `category`, `group`) | `[x, y, path_group]` | polygon vertex |
| `window` | one column | `as` (a new column; `kind: rank \| cume-dist \| cumulative-sum \| row-number`) | one per row |

Options: `density` takes `bandwidth`, `kernel`, `cumulative`, `counts`, `trim`;
`band` takes `side` (`both` / `left` / `right`), `scale` (`per-group` / `global`
/ `raw`), `overlap`, `span`, `split`, `position` (`identity` / `dodge`);
`quantile-box` and `density-2d` take their own layout/size options.

> `transform_calculate` is intentionally a Nushell job, not a flag: compute the
> column in the pipeline, where it is easier and no per-row closure crosses the
> plugin boundary.
>
> ```nu
> open data.csv | insert ratio {|r| $r.revenue / $r.cost} | charton point -x ratio -y score
> ```

---

## Level 4 — Compose and transform (recipes)

Putting Levels 2 and 3 together is how Charton's cookbook builds the charts
people ask for **by name but that are not marks**: ridgeline, ECDF, candlestick,
waterfall, raincloud, and so on. There is no command to learn — the recipe is
the code.

```nu
# Ridgeline = density + a one-sided band + polygon (violin's own pieces)
open assets/data.csv | charton polygon -x x -y y --path-group pg -c category --flip \
    --transform [
        {op: density, field: score, as: [value, density], group-by: [category], trim: true}
        {op: band, value: value, width: density, center: category,
         side: right, scale: per-group, overlap: 2.5, as: [x, y, pg]}
    ]

# ECDF = the CumeDist window transform + a step line
open assets/data.csv | charton line -x score -y ecdf -c group --interpolation step \
    --transform {op: window, field: score, kind: cume-dist, as: ecdf, group-by: [group]}

# Waterfall = floating bars on a running total
open assets/waterfall.csv | charton bar -x step -y cumulative -c kind \
    --transform {op: window, field: delta, kind: cumulative-sum, as: cumulative}

# Raincloud = a half violin, a quantile box, and the raw points
open assets/data.csv | charton polygon -x x -y y --path-group pg -c category --flip \
    --transform [
        {op: density, field: score, as: [value, density], group-by: [category], trim: true}
        {op: band, value: value, width: density, center: category,
         side: left, scale: per-group, as: [x, y, pg]}
    ] \
    --layer {geom: polygon, x: qx, y: qy, path_group: qpg, color: category,
             transforms: [{op: quantile-box, value: score, category: category, as: [qx, qy, qpg]}]} \
    --layer {geom: point, x: category, y: score}
```

Because a transform is just a record, the same pieces recombine into new charts
— and nothing has to be added to the plugin when the next one comes along.

---

## Flag reference

### Encodings

| Flag | Meaning |
|---|---|
| `-x, --x` | Column for the x axis |
| `-y, --y` | Column for the y axis (`hist` uses a generated `count`; `density` a generated `density`) |
| `-c, --color` | Column mapped to colour / grouping (required for `rect`) |
| `--path-group` | Column grouping rows into one `path` / `polygon` |
| `--y2` | Second y value: upper bound for `errorbar` / `rule`, or a floating `bar` |
| `--text` | Label column for `text` |
| `--size-by`, `--shape-by`, `--size-label`, `--shape-label` | Point size / shape channels and their legend labels (`point`) |
| `--geojson` | GeoJSON file to render (`geo`) |

### Statistics and transforms

| Flag | Meaning |
|---|---|
| `--transform` | One transform record, or a list applied in order (Level 3) |
| `--stack` | Bar/area stacking: `none` \| `stacked` \| `normalize` \| `center` |
| `--normalize` | Normalize y values to proportions |
| `--aggregate` | Aggregate y per x group: `sum` \| `mean` \| `median` \| `min` \| `max` \| `count` |
| `--bins` | Bins for `hist`, iso-levels for `contour`, grid size for `density-2d` |
| `--z` | Value column for `contour`; omit it to contour a 2-D density of `x`/`y` |
| `--density-bandwidth`, `--density-kernel` | KDE options for `density` (bandwidth in data units; `normal` \| `epanechnikov` \| `uniform`) |
| `--cumulative`, `--counts` | `density`: cumulative density, or smoothed counts |
| `--loess`, `--loess-bandwidth` | Smooth a `line` with LOESS |

### Scales, axes, and layout

| Flag | Meaning |
|---|---|
| `--x-scale`, `--y-scale` | Axis scale: `linear` \| `log` \| `discrete` \| `temporal` |
| `--x-min`, `--x-max`, `--y-min`, `--y-max` | Fix an axis bound (min and max go together) |
| `--x-expand`, `--y-expand` | Padding at both ends, as a fraction of the range |
| `--x-ticks`, `--y-ticks` | Explicit tick values, e.g. `[0 2 4 6]` |
| `--x-format`, `--y-format`, `--legend-format` | Label format: `compact`, or a record `{prefix, suffix, precision, compact, thousands, multiplier}` |
| `--x-label`, `--y-label`, `--color-label` | Axis and legend titles |
| `--x-angle` | X tick label angle in degrees |
| `--flip` | Swap the x and y axes |
| `--layer` | Overlay layer(s): a mark name, a record, or a list (Level 2) |
| `--facet-wrap`, `--facet-columns` | Wrap panels by a column, in N columns |
| `--facet-row`, `--facet-col`, `--facet-strategy` | Facet grid; scales `fixed` (default) \| `free` |
| `--coord` | `cartesian` (default) \| `polar` |
| `--inner-radius`, `--start-angle`, `--end-angle` | Polar radius and arc |
| `--margins` | Canvas margins as `top,right,bottom,left` (fractions) |

### Appearance

| Flag | Meaning |
|---|---|
| `-t, --title` | Chart title |
| `--theme` | `auto` (default) \| `light` \| `dark` |
| `--background` | Background color (overrides `--theme`) |
| `--color-map` | Continuous colour map (`viridis`, `magma`, `ylgnbu`, ...) |
| `--legend` | `left` \| `right` \| `top` \| `bottom` \| `none` |
| `--grid`, `--no-grid` | Force grid lines on / off |
| `--width`, `--height` | Pixel canvas size (default 800×600) |
| `--font-scale` | Multiply every chart font (title, labels, ticks, legend); default 1.5 |

### Mark styling

These are declared only on the marks that use them.

| Flag | Meaning |
|---|---|
| `--mark-color`, `--opacity`, `--size`, `--stroke`, `--stroke-width` | Colour, opacity, size, stroke |
| `--mark-width` | Band width 0.0–1.0 (`bar` / `boxplot` / `point` / `errorbar`) |
| `--shape` | Point shape: `circle` \| `square` \| `triangle` \| `star` \| `diamond` \| `pentagon` \| `hexagon` \| `octagon` |
| `--layout`, `--quasirandom-method` | Point layout (`standard` / `jitter` / `beeswarm` / `quasirandom`) and pairing (`tukey` / `pseudorandom`) |
| `--dash`, `--interpolation` | Line dash pattern (`6,4`) and interpolation (`linear` / `step` / `step-before`) |
| `--outlier-color`, `--outlier-size`, `--no-outliers` | Boxplot outliers |
| `--cap-length`, `--no-center` | Errorbar cap and centre dot |
| `--anchor`, `--weight` | Text anchor (`start` / `middle` / `end`) and weight (`normal` / `bold` / `100..900`) |

### Output and inline rendering

| Flag | Meaning |
|---|---|
| `-o, --output` | Save to `.svg` or `.png` (relative paths resolve against the shell's directory) |
| `--raw` | Return the image instead of drawing inline |
| `--png` | With `--raw`, return PNG bytes instead of an SVG string |
| `--scale` | Raster pixel scale (default 2.0; also disables inline auto-fit) |
| `--inline-style` | `auto` (default) \| `halfblock` \| `iterm2` \| `kitty` \| `sixel` |
| `--no-inline` | Never draw inline |
| `--force-inline` | Draw inline even when stdout is not a TTY (testing) |

## Configuration

Set defaults in `$env.config.plugins.charton`. Flags win over config, and config
wins over built-in defaults. Unknown keys warn on stderr, so typos surface.

```nu
$env.config.plugins.charton = {
    width: 1000
    height: 700
    scale: 2.0
    cell_width: 9                  # terminal cell size in device px
    cell_height: 20                # (tune if inline charts look soft)
    inline_style: kitty            # auto | halfblock | iterm2 | kitty | sixel
    grid: true
    palette: tab10                 # or ["#333" "#6fc481" "red"]
    legend: bottom                 # left | right | top | bottom | none
    x_angle: -45
    background: "#ffffff"
    theme: dark                    # auto | light | dark
    color_map: viridis
    font_scale: 1.5                # chart text size (try 1.5–2.0 in a terminal)
}
```

| Config key | Type | Purpose |
|---|---|---|
| `width`, `height` | int | Default pixel canvas |
| `scale` | float | Raster scale (disables inline auto-fit) |
| `cell_width`, `cell_height` | int | Assumed terminal cell size in device px; used to fit inline images |
| `inline_style` | string | Default inline renderer |
| `grid` | bool | Show grid lines |
| `palette` | string \| list | Named palette (`tab10`…`accent`) or explicit colours |
| `legend` | string | `left` / `right` / `top` / `bottom` / `none` |
| `x_angle` | float | X tick label angle |
| `background` | string | Chart background colour |
| `theme` | string | `auto` / `light` / `dark` |
| `color_map` | string | Continuous colour map |
| `font_scale` | float | Multiplies every chart font; default 1.5 |

## Inline rendering

In an interactive terminal the plugin draws the chart itself, best renderer
first, chosen from the environment:

| Tier | Protocol | Terminals |
|---|---|---|
| 1 | **Kitty graphics** | Kitty, Ghostty, recent WezTerm |
| 2 | **iTerm2 inline images** (`OSC 1337`) | iTerm2, WezTerm |
| 3 | **Sixel** | xterm, foot, mlterm, recent Windows Terminal |
| 4 | **Truecolor half-blocks** (`▀`) | everywhere |

Tiers 1–3 hand a real PNG bitmap to the terminal, so text and thin lines stay
sharp. Tier 4 is the universal fallback: it packs the bitmap into coloured `▀`
characters and works on any UTF-8 terminal. Override with `--inline-style`, the
`CHARTON_INLINE_STYLE` environment variable, or `inline_style` in config.

Inline drawing only happens when the plugin is **not** in stdio mode and its
`stdout` is a TTY; otherwise it falls back to `-o`, `--raw`, or an SVG string.
The rationale (process model, resolution fitting, protocol versioning) is in
[Nushell Plugin Internals](https://wangjiawen2013.github.io/charton/ecosystem/nushell_internals.html).

### Over SSH

Terminal identity variables (`TERM_PROGRAM`, `WEZTERM_*`, `KITTY_*`, ...) are
often not forwarded, but `TERM` is. If your client is not detected — for
example WezTerm set to `term = "xterm-256color"` — force the protocol:

```nu
$env.CHARTON_INLINE_STYLE = "iterm2"             # WezTerm / iTerm2
$env.config.plugins.charton = { inline_style: kitty }
```

## Troubleshooting

| Symptom | Try |
|---|---|
| Text and axes render as small coloured blocks | The half-block fallback is in use. Force an image protocol: `--inline-style iterm2` (WezTerm/iTerm2), `kitty`, or `sixel`. |
| Nothing inline, just an SVG string | stdout is not a TTY (piping / `nu -c`), or the plugin is in stdio mode. Use `-o` or `--raw`. |
| Charts look soft or stretched | Tune `cell_width` / `cell_height`, or pass an explicit `--scale`. |
| Command not found after a rebuild | Re-register (`plugin add <path>`) and restart, or `plugin rm charton` then add it again. |
| `--interpolation` rejected by `charton bar` | Flags are per-mark. Use `charton line` (or check `help charton bar`). |

`charton-probe` reports the transport, terminal capabilities, and the
environment variables used for detection. It is a development tool, built only
with `--features probe`:

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

`nu_plugin_charton` shares its version number with the `charton` library, so
`charton 0.8.3` and `nu_plugin_charton 0.8.3` ship as a pair.

## Uninstall

```nu
plugin rm charton
```

Delete any `plugin use charton` line from `config.nu`, and remove the binary
(`cargo uninstall nu_plugin_charton`) if you installed it with `cargo install`.
Commands already loaded stay in scope until the session ends, so restart
Nushell. `plugin rm` does not touch `$env.config.plugins.charton`; delete that
yourself if you no longer want it.

## Learn more

- [The Nushell Plugin](https://wangjiawen2013.github.io/charton/ecosystem/nushell.html)
  — the conceptual overview.
- [Nushell Plugin Internals](https://wangjiawen2013.github.io/charton/ecosystem/nushell_internals.html)
  — process model, inline protocols, resolution fitting.
- [Composing Your Own](https://wangjiawen2013.github.io/charton/gallery/primitives.html)
  — the grammar-of-graphics recipes the Level 4 examples follow.
- [charton library docs](https://docs.rs/charton) — the plotting engine.

## Development

This crate is a member of the `charton` workspace. `default-members = ["."]`, so
a plain `cargo build` at the repo root builds only the core library and skips
the heavy `nu-plugin` / `nu-protocol` tree.

```sh
cargo build -p nu_plugin_charton --release
cargo test  -p nu_plugin_charton
cargo build -p nu_plugin_charton --release --features probe
```

## License

Apache-2.0. See [LICENSE](https://github.com/wangjiawen2013/charton/blob/main/LICENSE).
