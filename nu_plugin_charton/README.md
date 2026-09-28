# nu_plugin_charton

A Nushell plugin that turns pipeline tables into charts using
[charton](https://github.com/wangjiawen2013/charton).

## Usage

```nu
# inline in the terminal (Kitty / iTerm2 / Sixel / truecolor half-blocks)
ls | charton -g bar -x name -y size

# save by extension (.svg or .png)
open data.csv | charton -g line -x date -y value -o chart.svg
open data.csv | charton -g beeswarm -x group -y score -o chart.png

# return data for further piping
open data.csv | charton -g point -x a -y b --raw | save chart.svg
open data.csv | charton -g point -x a -y b --raw --png | save chart.png

# group by a column with color
open data.csv | charton -g line -x t -y v -c series
open data.csv | charton -g boxplot -x group -y score -c cohort
open data.csv | charton -g errorbar -x group -y mean --y2 upper

# geographic choropleth from a GeoJSON file
charton -g geo --geojson world.geojson -c POP_EST -o world.png
```

### Flags

| Flag | Meaning |
|---|---|
| `-g, --geom` | `point` (default) \| `line` \| `area` \| `bar` \| `boxplot` \| `errorbar` \| `rule` \| `tick` \| `text` \| `rect`/`heatmap` \| `hist` \| `beeswarm` \| `geo` |
| `-x, --x` | column for the x axis |
| `-y, --y` | column for the y axis (`hist` uses a generated `count`) |
| `-c, --color` | column mapped to color/grouping (required for `rect`) |
| `--y2` | upper-bound column for `errorbar`/`rule` (errorbar aggregates mean ± std without it) |
| `--text` | label column for `-g text` |
| `--layer` | extra layer(s): a record or list of records with `geom`/`x`/`y`/`y2`/`color`/`text` |
| `-o, --output` | save to `.svg` or `.png` (relative paths resolve against the shell cwd) |
| `--geojson` | GeoJSON file to render with `-g geo` |
| `--scale` | raster pixel scale factor (default 2.0) |
| `--facet-wrap` | wrap panels by this column |
| `--facet-columns` | number of columns for `--facet-wrap` |
| `--facet-row`, `--facet-col` | two-field facet grid |
| `--facet-strategy` | `fixed` (default) \| `free` |
| `--x-label`, `--y-label`, `--color-label` | axis / legend labels |
| `--x-min`, `--x-max`, `--y-min`, `--y-max` | axis domain overrides (must be paired) |
| `--flip` | swap the x and y axes |
| `--grid`, `--no-grid` | force grid lines on/off |
| `--legend` | legend position: `left`/`right`/`top`/`bottom`/`none` |
| `--x-angle` | x tick label angle in degrees |
| `--mark-color`, `--opacity`, `--size`, `--stroke`, `--stroke-width` | mark-level styling |
| `-t, --title` | chart title |
| `--width`, `--height` | pixel canvas size (default 800x600) |
| `--raw` | return the image instead of drawing inline |
| `--png` | with `--raw`, return PNG bytes rather than an SVG string |
| `--no-inline` | never draw inline |
| `--force-inline` | draw inline even without a TTY (testing/override) |
| `--inline-style` | `auto` (default) \| `halfblock` \| `iterm2` \| `kitty` \| `sixel` |

Output precedence: inline (when the terminal supports it) → `-o` save →
`--raw` data → SVG string.

The `-g` names cover all **12 charton marks**; `beeswarm` is the `point` mark
configured with a beeswarm layout, so it is not a separate mark. `scatter`,
`box`, `label`, `heatmap`, `histogram`, and `geoshape` are aliases.

### Layers

Overlay additional marks that share the same scales and coordinate system.
Omitted fields default to the primary layer's values:

```nu
# line with points on top
open data.csv | charton -g line -x t -y v --layer {geom: point}

# bar with error bars
open data.csv | charton -g bar -x g -y v \
    --layer {geom: errorbar, y: v}

# several extra layers at once
open data.csv | charton -g line -x t -y v \
    --layer [{geom: point} {geom: rule, y2: upper}]
```

### Faceting

```nu
open data.csv | charton -g point -x t -y v --facet-wrap country --facet-columns 4
open data.csv | charton -g line  -x t -y v --facet-row species --facet-col sex
```

### Styling

Mark-level options apply to every layer where the mark supports them
(e.g. `--size` sets point size and text size, `--stroke-width` sets line
thickness; unsupported combinations are ignored):

```nu
open data.csv | charton -g point -x t -y v --size 6 --opacity 0.5 --mark-color "#e45756"
open data.csv | charton -g line  -x t -y v --stroke-width 2 --flip
open data.csv | charton -g bar   -x g -y v --y-label "count" --x-angle -30
```

### Configuration

Set defaults in `$env.config.plugins.charton`; command-line flags take
precedence, config beats built-in defaults:

```nu
$env.config.plugins.charton = {
    width: 1000
    height: 700
    scale: 2.0
    inline_style: kitty            # auto | halfblock | iterm2 | kitty | sixel
    grid: true
    palette: tab10                 # or ["#333" "#6fc481" "red"]
    legend: bottom                 # left | right | top | bottom | none
    x_angle: -45
    background: "#ffffff"
}
```

| Config key | Type | Purpose |
|---|---|---|
| `width`, `height` | int | default pixel canvas |
| `scale` | float | raster scale factor |
| `inline_style` | string | default inline renderer |
| `grid` | bool | show grid lines |
| `palette` | string \| list | named palette (`tab10`…`accent`) or explicit colors |
| `legend` | string | `left`/`right`/`top`/`bottom`/`none` |
| `x_angle` | float | x tick label angle |
| `background` | string | chart background color |

## Workspace

This crate is a member of the `charton` workspace. The workspace sets
`default-members = ["."]`, so a plain `cargo build` in the repo root only builds
the core `charton` crate and does **not** pull in the heavy
`nu-plugin`/`nu-protocol` dependency tree.

```sh
cargo build                          # core library only (fast)
cargo build -p nu_plugin_charton --release
cargo test  -p nu_plugin_charton
```

## Install

```sh
cargo build -p nu_plugin_charton --release
nu --plugins '[target/release/nu_plugin_charton.exe]'   # quick, no registry entry
# or register it (name is `charton`, without the nu_plugin_ prefix):
plugin add target/release/nu_plugin_charton.exe
plugin use charton
```

## Protocol version

`nu-plugin` / `nu-protocol` **must match the installed Nushell version**. This
crate is pinned to `=0.103.0`. After a Nushell upgrade, bump these and re-run
`plugin add`.

### Windows / interprocess pin

`nu-plugin-core 0.103.0` uses `local_socket::traits::ListenerNonblockingMode`,
which `interprocess >= 2.3` moved. This crate pins `interprocess = "=2.2.0"` to
keep the crate compiling. Remove the pin once Nushell ships a fix.

## How inline rendering works

Nushell launches Rust plugins in **local-socket mode** by default, so the
protocol travels over a named pipe and the plugin's `stdout` is inherited
straight from Nushell. That means in an interactive terminal the plugin may
write image escape sequences directly to `stdout`.

Verified on Windows + WezTerm (Nushell 0.103.0):

- `engine.is_using_stdio() == false` → local-socket mode.
- Writing an ANSI escape sequence to `stdout` passes through to the parent
  process (confirmed with `charton-probe --force`).

Rendering rules:

- Inline is used only when **not** stdio mode **and** `stdout` is a TTY.
- `charton-probe` reports the transport and terminal capabilities:

```nu
charton-probe | to md
```

- In stdio mode we never touch `stdout` (it is the protocol channel); the
  command falls back to returning data or saving to a file.

### Inline tiers

Three renderers, highest fidelity first, auto-detected from the environment
(override with `--inline-style`):

| Tier | Protocol | Terminals |
|---|---|---|
| 1 | **Kitty graphics** (`ESC_G...`) | Kitty, Ghostty, WezTerm (recent) |
| 2 | **iTerm2 inline images** (`OSC 1337`) | iTerm2, WezTerm |
| 3 | **Sixel** (`ESC P...q`) | xterm, foot, mlterm, Windows Terminal 1.22+ |
| 4 | **Truecolor half-blocks** (`▀`) | everywhere |

The full-resolution PNG is sent to the terminal in tiers 1–2 and scaled to the
terminal cell grid; sixel is downscaled to that grid first; tier 4 downsamples
to half-blocks in the plugin itself.

Detection order: `TERM_PROGRAM` (`*iterm*`/`*wezterm*` → iTerm2, `*ghostty*` →
Kitty) → `KITTY_WINDOW_ID` → `WEZTERM_EXECUTABLE`/`WEZTERM_PANE` → `TERM`
containing `sixel`/`foot`/`mlterm` → Sixel, `kitty` → Kitty → half-block.
WezTerm is deliberately mapped to iTerm2, which is its most reliable path.
Sixel is only auto-selected when `TERM` advertises it; otherwise choose it with
`--inline-style sixel`.

## Status

- [x] workspace + protocol-version pinning
- [x] `Value` table → charton `Dataset` converter (per-column type inference
      over all rows; int/float/string/bool/datetime, null-aware)
- [x] `charton` command: `point`, `line`, `area`, `bar`, `boxplot`,
      `errorbar` (explicit `--y2` or aggregated mean ± std), `rule`, `tick`,
      `text`, `rect`/`heatmap`, `hist`, `beeswarm`, `geo` (GeoJSON choropleth)
- [x] multi-layer overlays via `--layer`
- [x] inline terminal rendering: auto-detected **Kitty / iTerm2 / Sixel /
      half-block**
- [x] SVG / PNG export, `--raw` piping
- [x] faceting: `--facet-wrap` and `--facet-row`/`--facet-col` grid
- [x] axis labels, domain overrides, `--flip`, legend/grid overrides, and
      mark-level styling (`--mark-color`/`--opacity`/`--size`/`--stroke`/
      `--stroke-width`)
- [x] configuration via `$env.config.plugins.charton` (size, scale, palette,
      legend, grid, x angle, background, inline style)
- [x] unit tests via `nu-plugin-test-support`

Planned: theme presets, heatmap color maps, companion label/annotation marks.
