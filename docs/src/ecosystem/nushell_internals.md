# Nushell Plugin Internals

This chapter is deliberately about the parts of `nu_plugin_charton` that are
*not* obvious from the command line: how a plugin talks to Nushell, how a chart
gets onto the screen, why some inline charts look blurry, and how the protocol
version is actually enforced. If you only want to use the plugin, read
[The Nushell Plugin](nushell.md) first.

## 1. Process model: local-socket mode

Nushell normally launches a Rust plugin in **local-socket mode**: the plugin
protocol travels over a named pipe (a Unix domain socket or Windows named
pipe), and the child process **inherits the shell's `stdin`/`stdout`/`stderr`**.
This is the detail that makes inline images possible at all — the plugin can
write an ANSI/terminal escape sequence straight to `stdout` and it lands on the
real terminal.

The alternative is **stdio mode**, where the protocol itself uses
`stdin`/`stdout`. Writing anything else to `stdout` there would corrupt the
protocol.

The rule the plugin follows:

```text
inline rendering is allowed  ⇔  NOT stdio mode  AND  stdout is a TTY
```

`engine.is_using_stdio()` reports the transport; `stdout().is_terminal()` gates
the rest. The `charton-probe` command prints exactly this decision, plus the
environment variables used for protocol detection:

```nu
charton-probe | to md
```

In stdio mode (or when `stdout` is a pipe) the plugin silently falls back to
`-o` saving, `--raw` data, or an SVG string — it never writes escape sequences.

## 2. The inline rendering ladder

Terminals disagree about how to display images, so the plugin tries four tiers
and picks the best one the environment advertises:

| Tier | Protocol | Detected from | Terminals |
|---|---|---|---|
| 1 | **Kitty graphics** (`ESC_G…`) | `KITTY_WINDOW_ID`, `TERM=*kitty*`, `TERM_PROGRAM=*ghostty*` | Kitty, Ghostty, recent WezTerm |
| 2 | **iTerm2 inline images** (`OSC 1337`) | `TERM_PROGRAM=*iterm*` or `*wezterm*`, `WEZTERM_*` | iTerm2, WezTerm |
| 3 | **Sixel** (`ESC P…q`) | `TERM=*sixel*`, `foot*`, `mlterm*` | xterm, foot, mlterm, Windows Terminal 1.22+ |
| 4 | **Truecolor half-blocks** (`▀`) | always | everything |

Override with `--inline-style auto|halfblock|iterm2|kitty|sixel`. WezTerm is
mapped to **iTerm2** deliberately: it is its most reliable protocol, and it
avoids the platform-specific Sixel support matrix.

Every frame ends on a clean line at column 0, so whatever follows — usually
the shell prompt — starts below the chart at the left margin.

## 3. Pitfall: terminals cannot render SVG

It is tempting to ask "why rasterize at all — can't we hand the terminal the
SVG?" The answer is no. The image protocols are **raster-only**:

- The iTerm2 protocol carries an encoded bitmap (`type=image/png`, `…/jpeg`).
- Kitty carries raw RGB/RGBA or PNG.
- Sixel is, by definition, a bitmap format.
- Half-blocks are colored Unicode cells.

WezTerm, for example, rejects SVG outright:

```text
$ wezterm imgcat chart.svg
ERROR wezterm > unknown image format!?; terminating
```

So the correct division of labour is:

| Goal | Format |
|---|---|
| Inline in the terminal | **raster** (PNG → terminal protocol) |
| Browser, print, publication, further editing | **SVG** (`-o chart.svg`, or `--raw`) |

Because the terminal path is raster, the quality question is entirely about
**resolution and scaling**, which is the next section.

## 4. Fitting the raster to the terminal

### The symptom

The first generation of the plugin always rendered an 800×600 logical canvas at
`scale = 2.0`, i.e. a **1600×1200 PNG**, and asked the terminal to draw it into
the character grid. The terminal then had to shrink it by a large,
non-integer factor — in one measured case ≈ 2.7×. That produced the two
classic complaints:

- **Blurry text** — glyphs rendered at 24 px and resampled down to ~9 px.
- **Uneven / doubled lines** — a 2 px stroke resampled by a fractional factor
  lands between pixel rows, so the x-axis can appear as *two lines overlapping
  by half a pixel*.

The raster itself was fine: measuring the PNG showed the horizontal axis, the
vertical axis, and the bar outlines were all exactly 2 px. The distortion was
created *after* rendering, by the terminal.

### The fix

Render the PNG at (approximately) the terminal's **device-pixel area**, so the
terminal scales little or not at all. With `max_cols`/`max_rows` characters and
a cell size of `cell_w × cell_h` device pixels:

```text
target_w = max_cols · cell_w
target_h = max_rows · cell_h
scale    = clamp( min(target_w / logical_w, target_h / logical_h), 0.25, 8.0 )
```

The chart's **logical layout is unchanged** — only the pixel density is fitted
— which keeps fonts and margins proportional. The chosen cell counts are then
computed with the same `cell_w : cell_h` ratio (`fit_cells_with_cell`), so the
display box has the image's aspect ratio and the terminal does not stretch it.

### The cell size is the crux

The single most important input is the terminal cell size, and it **cannot be
assumed**. The original default of `8×16` was wrong for a real WezTerm setup
using JetBrains Mono 11 pt, whose cells are `9×20`. The result was that the
terminal *upscaled* the image by 1.125× — and upscaling is always blurrier than
downscaling. The default is now `9×20`; tune it if your font/DPI differ.

Measure it exactly. WezTerm reports the window in pixels *and* in cells via its
CLI:

```sh
wezterm cli list --format json
# → "size": { "rows": 48, "cols": 211,
#             "pixel_width": 1899, "pixel_height": 960, "dpi": 96 }
# cell = 1899/211 × 960/48 = 9 × 20
```

Then pin it in config:

```nu
$env.config.plugins.charton = { cell_width: 9, cell_height: 20 }
```

Rules of thumb:

- **Downscaling beats upscaling.** When unsure, over-estimate the cell size a
  little; a mild downscale stays sharp, an upscale does not.
- Setting an explicit `--scale` **disables** the auto-fit, in case you want full
  manual control.
- The **half-block** tier is exempt: one character always covers a fixed 1×2
  block of pixels, so its resampling is decided inside the plugin and always
  uses a 2:1 ratio.

### Why not just render at integer scale?

Charton's raster backend draws glyph **outlines** and fills them with an
anti-aliasing rasterizer; it does not hint glyphs to the pixel grid. At a
fractional scale a 1 px line is spread across two rows, which is why an
over-eager fractional fit can itself look soft. The practical goal is therefore
*one* resample at most — ideally none — between Charton and the screen. Fitting
the raster to the measured cell box gets you to ≈1:1, which is where both the
text and the stroke widths look correct.

## 5. Protocol versioning: what is actually checked

The plugin's `Cargo.toml` pins the Nushell crates exactly, and the comment says
the protocol version *must match* the installed shell. Here is the precise
semantics, because "match" is more permissive than it sounds.

At startup the plugin and the engine exchange a `Hello` carrying a
`ProtocolInfo`:

```rust
// nu-plugin-protocol/src/protocol_info.rs
pub struct ProtocolInfo {
    pub version: String,   // == env!("CARGO_PKG_VERSION") of nu-plugin-protocol
    pub features: Vec<Feature>,
}

pub fn is_compatible_with(&self, other: &ProtocolInfo) -> Result<bool, ShellError> {
    // sort the two versions, drop any prerelease tag, then test the *higher*
    // version against a caret requirement taken from the *lower* version
    Ok(semver::Comparator {
        op: semver::Op::Caret,
        major: versions[0].major,
        minor: Some(versions[0].minor),
        patch: Some(versions[0].patch),
        ..
    }.matches(&versions[1]))
}
```

Three consequences:

1. **Only `nu-plugin-protocol`'s version is checked.** The plugin's own
   `version()` (what `plugin list` shows) is pure metadata — the plugin can
   report `0.7.1` and still load under Nushell `0.116.0`.
2. **Patch versions are compatible.** For `0.x`, a caret requirement locks the
   minor: `^0.116.0` means `>= 0.116.0, < 0.117.0`. So `0.116.1` and
   `0.116.0` are compatible in either direction, which is exactly why the
   dependency can be written `=0.116`.
3. **Only a minor bump breaks.** `0.116.x` ↔ `0.117.x` is rejected. This is the
   real constraint the exact pin protects.

| Engine | Plugin protocol | Compatible? |
|---|---|---|
| 0.116.0 | 0.116.0 | ✅ |
| 0.116.0 | 0.116.1 | ✅ (`^0.116.0`) |
| 0.116.0 | 0.117.0 | ❌ |
| 0.116.0 | 0.0.0 | ❌ |

## 6. The Nushell 0.116 plugin model

0.116 changed how plugins are managed, and two of the changes trip people up:

- **Registration ≠ loading.** `plugin add` only writes the plugin registry
  (`$nu.plugin-path`); it does not put commands in scope. You must call
  `plugin use charton`, and to make it permanent, add that line to `config.nu`.
  `plugin use` is a *parser keyword*, so it has to stand on its own line and
  cannot share a script with the `plugin add` that registers the plugin.
- **The registry stores the reported version.** After a rebuild that changes
  the version, re-run `plugin add`, otherwise `plugin list` keeps showing the
  old number.

Also worth knowing on Windows: `plugin add` wants the **executable path**, not
the command name. `plugin add charton` fails with *file not found*, and even a
bare `nu_plugin_charton` can fail to spawn — use the full path
(`C:/Users/you/.cargo/bin/nu_plugin_charton.exe`).

## 7. Migrating the crate across a Nushell release

Beyond the version pins, each Nushell release can change `nu-protocol`'s public
types. The upgrade to 0.116 required one such fix, worth remembering as a
pattern:

```rust
// nu-protocol 0.116 wraps list payloads in a copy-on-write container
Value::List { vals, .. } => vals.into_owned(),   // vals: SharedCow<Vec<Value>>
```

A `cargo build -p nu_plugin_charton` is the fastest way to surface these — the
compiler points straight at the changed signature.

## 8. Pitfall cheat-sheet

| Pitfall | Root cause | Fix |
|---|---|---|
| Blurry inline text | terminal scaled a fixed 1600×1200 bitmap | render at `max_cols × cell` device pixels |
| x-axis looks like two lines | fractional resample of a 1–2 px stroke | match the cell size, reach ≈1:1 |
| Chart too small / huge | `cell_width`/`cell_height` wrong for the font | measure with `wezterm cli list`, set config |
| `plugin add charton` → file not found | argument must be the executable, not the command | pass the full path to `nu_plugin_charton(.exe)` |
| `plugin use` → plugin not found | 0.116 does not auto-load registered plugins | `plugin add` first, then `plugin use` in `config.nu` |
| Protocol error after a version bump | plugin protocol minor differs from the engine | keep `nu-plugin*` on the engine's `0.116.x` line |
| Inline output corrupts a pipe | escape sequence written in stdio mode | inline is gated on `!is_using_stdio() && tty` |
| Prompt indented after an inline chart | some image protocols leave the cursor at the image's right edge | every frame ends with a CRLF + backspace/CR cursor resync |
