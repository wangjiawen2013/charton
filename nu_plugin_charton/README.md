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
| `--layer` | extra layer(s): a record or list of records with `geom`/`x`/`y`/`y2`/`color`/`text`, plus optional `style` and `x_scale`/`y_scale` |
| `-o, --output` | save to `.svg` or `.png` (relative paths resolve against the shell cwd) |
| `--geojson` | GeoJSON file to render with `-g geo` |
| `--scale` | raster pixel scale factor (default 2.0) |
| `--facet-wrap` | wrap panels by this column |
| `--facet-columns` | number of columns for `--facet-wrap` |
| `--facet-row`, `--facet-col` | two-field facet grid |
| `--facet-strategy` | `fixed` (default) \| `free` |
| `--x-label`, `--y-label`, `--color-label` | axis / legend labels |
| `--x-scale`, `--y-scale` | axis scale: `linear` \| `log` \| `discrete` \| `temporal` |
| `--color-map` | continuous color map for `rect`/heatmap/density, e.g. `viridis`, `magma`, `ylgnbu` |
| `--x-format`, `--y-format`, `--legend-format` | tick/legend label format: preset `compact`, or a record `{prefix, suffix, precision, compact, thousands, multiplier}` |
| `--x-min`, `--x-max`, `--y-min`, `--y-max` | axis domain overrides (must be paired) |
| `--flip` | swap the x and y axes |
| `--grid`, `--no-grid` | force grid lines on/off |
| `--legend` | legend position: `left`/`right`/`top`/`bottom`/`none` |
| `--theme` | color theme: `auto` (default) \| `light` \| `dark` |
| `--x-angle` | x tick label angle in degrees |
| `--mark-color`, `--opacity`, `--size`, `--stroke`, `--stroke-width` | mark-level styling |
| `--shape` | point shape: `circle`/`square`/`triangle`/`star`/`diamond`/`pentagon`/`hexagon`/`octagon` |
| `--dash` | line dash pattern, e.g. `6,4` |
| `--interpolation` | line interpolation: `linear`/`step`/`step-before` |
| `--outlier-color` | outlier color for `-g boxplot` |
| `--anchor`, `--weight` | text anchor (`start`/`middle`/`end`) and font weight (`normal`/`bold`/`100..900`) |
| `--layout`, `--quasirandom-method` | point layout (`standard`/`jitter`/`beeswarm`/`quasirandom`) and its pairing method |
| `--background` | chart background color (overrides `--theme`) |
| `-t, --title` | chart title |
| `--width`, `--height` | pixel canvas size (default 800x600) |
| `--raw` | return the image instead of drawing inline |
| `--png` | with `--raw`, return PNG bytes rather than an SVG string |
| `--no-inline` | never draw inline |
| `--force-inline` | draw inline even without a TTY (testing/override) |
| `--inline-style` | `auto` (default) \| `halfblock` \| `iterm2` \| `kitty` \| `sixel` |

Output precedence: inline (when the terminal supports it) → `-o` save →
`--raw` data → SVG string.

### Inline rendering over SSH

The plugin picks an inline protocol from the environment. The terminal-specific
variables (`TERM_PROGRAM`, `WEZTERM_*`, `KITTY_*`, ...) are **not forwarded by
SSH by default**, but `TERM` always is. `auto` therefore also reads `TERM`, so a
WezTerm/Kitty/Ghostty client is recognised on a remote server and the crisp
image protocols are used instead of the half-block fallback (where text and
thin axis lines degrade into coloured blocks).

If your client is not detected (for example WezTerm configured with
`term = "xterm-256color"`), force it either with the `CHARTON_INLINE_STYLE`
environment variable, the `--inline-style` flag, or the plugin config:

```nu
$env.CHARTON_INLINE_STYLE = "iterm2"            # WezTerm / iTerm2
$env.config.plugins.charton = { inline_style: kitty }
```

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
    cell_width: 9                  # terminal cell size in device px
    cell_height: 20                # (tune if inline charts look soft)
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
| `scale` | float | raster scale factor (also disables inline auto-fit) |
| `cell_width`, `cell_height` | int | assumed terminal cell size in device px; used to fit inline images |
| `inline_style` | string | default inline renderer |
| `grid` | bool | show grid lines |
| `palette` | string \| list | named palette (`tab10`…`accent`) or explicit colors |
| `legend` | string | `left`/`right`/`top`/`bottom`/`none` |
| `x_angle` | float | x tick label angle |
| `background` | string | chart background color |
| `theme` | string | color theme: `auto`/`light`/`dark` |
| `color_map` | string | continuous color map, e.g. `viridis`/`magma`/`ylgnbu` |

## Workspace

This crate is a member of the `charton` workspace. The workspace sets
`default-members = ["."]`, so a plain `cargo build` in the repo root only builds
the core `charton` crate and does **not** pull in the heavy
`nu-plugin`/`nu-protocol` dependency tree.

```sh
cargo build                          # core library only (fast)
cargo build -p nu_plugin_charton --release
cargo test  -p nu_plugin_charton
# Development-only diagnostic command `charton-probe` (off by default):
cargo build -p nu_plugin_charton --release --features probe
```

## Installation

The plugin is not published to crates.io yet, so build it from this repository.
It requires the **same Nushell version it is pinned to** — see
[Protocol version](#protocol-version).

### 1. Build

```sh
cargo build -p nu_plugin_charton --release
```

The binary lands in the workspace target directory:

| Platform | Path |
|---|---|
| Linux / macOS | `target/release/nu_plugin_charton` |
| Windows | `target/release/nu_plugin_charton.exe` |

Alternatively, install it onto your `PATH` with:

```sh
cargo install --path nu_plugin_charton --locked
```

### 2. Register with Nushell (once)

`plugin add` records the plugin in Nushell's registry
(`$nu.plugin-path`). From a Nushell session, pass the **executable**
(filename or path), not the command name:

```nu
# Linux / macOS
plugin add target/release/nu_plugin_charton

# Windows
plugin add target/release/nu_plugin_charton.exe
```

After `cargo install`, the executable lives in `~/.cargo/bin`; the most
reliable form is its full path, e.g.
`plugin add 'C:/Users/you/.cargo/bin/nu_plugin_charton.exe'`. Passing the
registered name instead (`plugin add charton`) fails with a file-not-found
error, and on Windows a bare `nu_plugin_charton` may fail to spawn even when it
is on `PATH`. The registered name drops the `nu_plugin_` prefix, so this plugin
is `charton`.

To try it without touching the registry:

```sh
# Linux / macOS
nu --plugins '[target/release/nu_plugin_charton]'
# Windows
nu --plugins '[target/release/nu_plugin_charton.exe]'
```

### 3. Load it

In Nushell 0.116 a plugin in the registry is **not** loaded automatically on
startup; its commands only enter scope after `plugin use`:

```nu
plugin use charton
```

To load it in every session, add the same line to your `config.nu` (after the
registry has been written by step 2). `plugin use` is a parser keyword, so it
must be on its own line and cannot share a script with the `plugin add` that
registers the plugin. If you only want to try it without touching the registry,
use the `--plugins` flag shown above instead — that loads the plugin directly.

### 4. Verify

```nu
plugin list | where name == charton
charton-probe | to md          # transport + terminal capability report
[[g, v]; [a, 1] [b, 2]] | charton -g bar -x g -y v
```

In an interactive terminal the last command renders the chart inline; in a
non-interactive context (`nu -c`, piping to a file) it returns the SVG string
instead. To update an already-registered plugin after a rebuild, run
`plugin add` again and restart, or `plugin use charton`.

## Uninstall

Remove it from the registry so `plugin use` can no longer load it (also delete
any `plugin use charton` line you added to `config.nu`):

```nu
plugin rm charton
```

Commands created by the plugin stay in scope until the current session ends, so
restart Nushell to fully unload it. If you installed the binary with
`cargo install`, remove that too:

```sh
cargo uninstall nu_plugin_charton
```

Otherwise delete the build artifact manually (`rm -rf target/release/nu_plugin_charton*`
on Linux/macOS). `plugin rm` does **not** touch any `$env.config.plugins.charton`
settings you added to `config.nu`; delete those yourself if you no longer want
them.

## Protocol version

`nu-plugin` / `nu-protocol` **must match the installed Nushell version**. This
crate is pinned to `=0.116.0`. After a Nushell upgrade, bump these and re-run
`plugin add`.

## How inline rendering works

> The full rationale — process model, why SVG cannot be drawn inline, cell-size
> tuning, and protocol-version semantics — is written up in the book chapter
> [**Nushell Plugin Internals**](https://wangjiawen2013.github.io/charton/ecosystem/nushell_internals.html).
> This section is the operational summary.

Nushell launches Rust plugins in **local-socket mode** by default, so the
protocol travels over a named pipe and the plugin's `stdout` is inherited
straight from Nushell. That means in an interactive terminal the plugin may
write image escape sequences directly to `stdout`.

Verified on Windows + WezTerm (Nushell 0.116.0):

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

For tiers 1–3 the PNG is rendered to fit the terminal's pixel area (the
character grid times the cell size), so the terminal barely scales it — that is
what keeps text crisp and horizontal/vertical strokes the same width. The cell
size defaults to **9×20 device px**; set `cell_width`/`cell_height` if your font
or DPI differ (an explicit `--scale` disables the auto-fit). To measure it:

```sh
wezterm cli list --format json   # cell = pixel_width/cols × pixel_height/rows
```

Tier 4 downsamples to half-blocks in the plugin itself and keeps the
full-resolution raster.

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
      `--stroke-width`/`--shape`/`--dash`/`--interpolation`/`--outlier-color`/
      `--anchor`/`--weight`)
- [x] axis scales (`--x-scale`/`--y-scale`), continuous color maps
      (`--color-map`), and tick/legend formatting (`--x-format`/`--y-format`/
      `--legend-format`)
- [x] per-layer `style` and `x_scale`/`y_scale` overrides in `--layer`
- [x] configuration via `$env.config.plugins.charton` (size, scale, palette,
      color map, legend, grid, x angle, background, inline style, theme)
- [x] light/dark color themes (`--theme`, auto-detected from `COLORFGBG`)
- [x] unit tests via `nu-plugin-test-support` (with unknown config-key warnings)

Planned: companion label/annotation marks, a native ANSI terminal backend
(crisp text without going through a raster image).
