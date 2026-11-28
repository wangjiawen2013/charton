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

```sh
cargo install --path nu_plugin_charton --locked
```

```nu
plugin add '~/.cargo/bin/nu_plugin_charton.exe'   # Windows: full path is most reliable
plugin use charton
```

`plugin use` is a parser keyword, so the two lines must be separate inputs, not
one script.

```nu
# inline
open data.csv | charton bar -x region -y revenue

# save by extension
open data.csv | charton line -x date -y value -o revenue.svg
```

The full command surface — every mark, `--layer`, `--transform`, the flag
reference, config keys, inline protocols, troubleshooting and compatibility — is
the [plugin README](https://github.com/wangjiawen2013/charton/blob/main/nu_plugin_charton/README.md).
This chapter is the design behind it.

> **First chart looks blurry?** Nothing is wrong. With no `inline_style`
> configured the plugin uses the universal `halfblock` renderer, which works on
> every terminal but is soft. Set the protocol once in
> `$env.config.plugins.charton` (`iterm2`, `kitty`, or `sixel`) for sharp
> bitmap output.

The plugin is built around four levels, from "just draw my rows" to writing a
full recipe: **marks** (geometry primitives and statistical atoms),
**composition** (`--layer`), **transforms** (`--transform`), and both together.
Chart types that Charton builds from primitives — ridgeline, ECDF, candlestick,
raincloud, violin-with-box — are not commands: they are recipes written with
`--transform` and `--layer`. The vocabulary is closed, so a new chart type needs
no plugin change. [Writing Transforms](nushell_transforms.md) is the full
catalogue and the worked recipes.

## Chart documents

Every example above is a flag recipe. The same chart can also be a JSON
document — the Vega-Lite v5 subset described in
[Writing a chart document](../spec/index.md) — which `charton render` draws:

```nu
# The document carries its rows inline; `open` parses the JSON.
open chart.json | charton render

# A path names the document, and the pipeline supplies its rows.
open sales.csv | charton render chart.json -o report.svg
```

The document owns the grammar; the entry point owns the canvas and delivery, so
`--width`, `--height`, `--scale`, `--font-scale`, `-o`, `--raw` and the inline
flags behave exactly as they do on a mark command. Two boundaries keep the
dialect single:

- **A piped table replaces the document's `data`**, so one document can be
  replayed against a different table without editing it.
- **A document read from the pipeline does not resolve `data.url`.** The plugin
  has no document directory to anchor a relative path to, and reading files is
  the shell's job: inline the rows, or pipe the table.

Because it is the same dialect as the CLI and the MCP server, a document is what
you commit, review in a diff, and hand to an agent — and the CLI's
`charton schema` prints its JSON Schema.

## Configuration

Defaults live in `$env.config.plugins.charton`; flags win over config, and
config wins over built-in defaults. `CHARTON_INLINE_STYLE` sits between the
flag and config for the inline renderer. The
[plugin README](https://github.com/wangjiawen2013/charton/blob/main/nu_plugin_charton/README.md#configuration)
lists every key, its default, and the one knob that is deliberately not a key
(tick density). [Nushell Plugin Internals](nushell_internals.md) explains the
cell-size settings.
