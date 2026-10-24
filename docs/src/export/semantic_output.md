# Semantic Output: CSS, Accessibility & Tagging

An SVG exported by Charton is a *document*, not a picture: every axis label,
grid line and data mark is an element in the page that embeds it. The document
carries its own structure — semantic classes, identity attributes and
accessibility metadata — so a page stylesheet can restyle a chart **without
re-rendering it**. A PDF exported from the same chart carries the equivalent
structure as a tag tree.

```rust
let svg = chart.to_svg()?;                       // semantic classes, image-level a11y
let svg = chart.to_svg_with(&SvgOptions::default()
    .with_a11y(A11y::Document)
    .with_id_prefix("sales")
    .with_defs(HATCH_PATTERN))?;
```

The options never change the picture, only how it is written: the same chart
rendered with different options draws exactly the same shapes.

## A live example

Below is a real HTML page, embedded as is: one static SVG document plus one
stylesheet. Nothing re-renders per theme, and no JavaScript is involved — switch
the book's light/dark theme and reload to see the same file restyled from your
operating system's preference (the frame follows `prefers-color-scheme`).

<iframe src="semantic_demo.html" title="A chart restyled by a page stylesheet"
        style="width:100%;height:520px;border:1px solid #8884;border-radius:6px"></iframe>

Its stylesheet is written against the classes and attributes documented here:

```css
[data-layer="0"] path            { fill: url(#demo-band); stroke: none; }
[data-layer="1"] path            { fill: none; stroke: var(--accent); stroke-width: 2.5; }
#demo-band rect                  { fill: var(--accent); fill-opacity: .10; }
#demo-band path                  { stroke: var(--accent); stroke-opacity: .22; }
.charton-tick-label text         { font-size: var(--text-tick); fill: var(--muted); }
.charton-tick-label text[data-value="2020"] { fill: var(--highlight); font-weight: 700; }
@media (prefers-color-scheme: dark) { :root { --accent: #60a5fa; --highlight: #f87171; } }
```

`examples/semantic_demo.rs` generates that page (and a standalone copy under
`target/test-output/`), so the example and the documentation cannot drift.

## The model: three channels

| Channel | Lives on | Example | Cost |
|---|---|---|---|
| **role** | a container `<g>` | `class="charton-tick-label"` | one group per tick, layer, legend block |
| **identity** | the same container | `data-layer="1"`, `data-series="Ford"` | none beyond the group |
| **observation** | the drawn primitive | `data-value="2020"` | a few bytes per addressed element |

Roles and identity describe a *run* of elements, so their number is
O(ticks + layers) rather than O(points) — which is what keeps the streaming,
allocation-free backends intact. A value describes one bar or one label, and it
has to sit on the element itself: a stylesheet rule that matches a parent
container cannot override the child's own presentation attributes.

## Why this matters

Colors and font sizes are inlined as presentation attributes, and CSS already
overrides those. What was missing was a way to *address* an element: without
classes, a stylesheet can only say `text { … }`, which hits the axis labels, the
data labels, the legend and the title at once.

```css
/* All or nothing. */
text { font-size: 11px; }

/* By role. */
.charton-tick-label text { font-size: 11px; fill: var(--text-muted); }
.charton-grid            { display: none; }
[data-layer="1"] path    { stroke: var(--accent); stroke-width: 2.5; }
[data-value="2020"]      { font-weight: 700; }
```

Two consequences of the CSS cascade are worth stating once:

* A rule must match the element that carries the property. Colors and fonts live
  on the drawn `<text>`/`<path>`, so a role class on a container needs a
  descendant selector (`.charton-tick-label text`); otherwise the element's own
  presentation attribute wins over the value inherited from its parent.
* `data-value` is the exception that proves the rule: it is written **on the
  element**, so a plain `[data-value="2020"]` rule works directly. When a role
  rule also sets the same property — the tick-label colour, for example —
  qualify the selector (`.charton-tick-label text[data-value="2020"]`) so it
  wins on specificity.

This is what makes the following possible without rendering twice:

* **Dark mode and design tokens.** One cached SVG, restyled by
  `prefers-color-scheme` or by the host page's CSS custom properties.
* **Print and greyscale.** `@media print { .charton-grid { display: none } }`,
  or hatched fills from a custom `<defs>` pattern.
* **Highlighting.** A few lines of JavaScript (or `:hover`) on
  `[data-value="2020"]`, or on a whole series through `.charton-series`.
* **Accessibility.** Screen readers get the chart's title and description
  instead of a stream of tick numbers.

> **Inline only.** Page CSS can only reach an SVG that is part of the page's
> DOM. An `<img src="chart.svg">` or a CSS `background-image` is a separate
> document: nothing from the page applies, and neither do webfonts. Use
> `element.innerHTML = svg` (or a server-side include) when you want to style it.

## The class grammar

Roles are a **closed** set, so they can be class names. They are all prefixed
with `charton-`, so an inlined chart cannot collide with the host page's own
`.axis` or `.title` rules.

| Class | Element |
|---|---|
| `charton` | the root `<svg>` |
| `charton-background` | the canvas fill |
| `charton-title` | the chart title |
| `charton-facet-label` | a facet strip's label |
| `charton-plot` | the clipped plotting area |
| `charton-grid`, `charton-frame` | grid lines, the panel frame |
| `charton-axis`, `charton-axis-x` / `-y` / `-radius` / `-theta` | one axis |
| `charton-axis-line`, `charton-axis-title` | its spine, its title |
| `charton-tick`, `charton-tick-label` | tick marks, tick labels |
| `charton-mark`, `charton-mark-<layer>`, `charton-mark-<type>` | data marks |
| `charton-series` | the marks of one series inside a layer |
| `charton-legend`, `charton-legend-title` | one legend block, its title |
| `charton-legend-entry`, `-swatch`, `-label` | one legend entry and its parts |
| `charton-colorbar` | a continuous colour scale |

A mark group carries three classes at once, e.g.
`class="charton-mark charton-mark-1 charton-mark-errorbar"`: the role, the layer
index, and the mark type.

## Identity attributes

Roles are closed; *identity* is open-ended (it comes from the data), so it is an
attribute, never a class. Keeping data values out of class names avoids escaping
problems and protects the hot path.

| Attribute | On | Value |
|---|---|---|
| `data-layer` | mark groups | the layer index (`0`, `1`, …) |
| `data-mark` | mark groups | the mark type (`line`, `bar`, …) |
| `data-series` | series groups, legend blocks, flat-stream marks | the series name, or the field a legend explains |
| `data-axis` | axis groups | `x`, `y`, `radius`, `theta` |
| `data-value` | the drawn element itself | the value the element stands for |

The same value appears on a tick label and on the bar it labels, so one rule can
highlight both; and a series can be restyled without counting layers:

```css
[data-value="2020"]                             { font-weight: 700; }
path[data-value="2020"]                         { fill: var(--accent); }
.charton-series[data-series="Ford"] path        { stroke: var(--accent); }
```

`data-value` is set where a single primitive *is* one observation **and that
observation has an unambiguous text form**: categorical bar charts, and every
axis tick label. Some elements deliberately get none:

* a line or area stands for a whole series, which its `charton-series` scope
  already identifies;
* a histogram bin is a continuous interval whose centre has no stable text form;
* a heat-map cell is identified by two values, not one.

All three would need a dedicated identity scheme rather than a single
`data-value`. Marks drawn as a flat stream — a grouped bar chart, where
regrouping would change the draw order and therefore the visual stacking — carry
the series on the element instead of in a scope:

```xml
<path data-value="Jan" data-series="A" … />
```

## Accessibility

`SvgOptions::a11y` selects how much metadata is emitted:

| Mode | Emits |
|---|---|
| `A11y::Off` | nothing: a document that describes only what it draws |
| `A11y::Image` (default) | `role="img"`, `aria-roledescription="chart"`, `<title>`/`<desc>`, `aria-labelledby` (name) + `aria-describedby` (description) |
| `A11y::Document` | `role="graphics-document"`, marks as `graphics-object`, `aria-label` on legend entries, `aria-hidden="true"` on decorative groups |

`Image` satisfies WCAG 2.1 SC 1.1.1 ("non-text content has a text alternative")
and is what almost every chart needs. It only emits `role` when the chart has a
title or a description: an `role="img"` element with no accessible name is worse
than no role at all.

The title and the description keep the two distinct roles SVG defines for them:
`<title>` is the accessible *name*, `<desc>` the accessible *description*,
repeated explicitly as `aria-labelledby` / `aria-describedby` because support for
the native mapping is uneven. A description without a title becomes the name —
there is nothing else to announce, and an unnamed image is the worse failure.

`Document` follows the WAI-ARIA Graphics Module for charts that should be
explored element by element. Only in that mode does `aria-hidden` on the grid
and background mean anything — under `role="img"` the whole subtree is already
one node.

The text comes from the chart itself; `<desc>` is never drawn, it exists only
for assistive technology:

```rust
chart
    .with_title("Annual output")
    .with_description("Annual output with its uncertainty band.");
```

## Definitions and ids

Every definition (`<clipPath>`, `<linearGradient>`, and anything you supply) is
collected into a single `<defs>` block ahead of the drawing.

```rust
let hatch = r#"<pattern id="sales-hatch" width="4" height="4" patternUnits="userSpaceOnUse">
                 <path d="M0 4 L4 0" stroke="#888" />
               </pattern>"#;

let svg = chart.to_svg_with(
    &SvgOptions::default()
        .with_id_prefix("sales")
        .with_defs(hatch),
)?;
```

```css
[data-layer="0"] path { fill: url(#sales-hatch); }
```

Fragments are inserted verbatim: their well-formedness is your responsibility,
and their ids should be namespaced with the same prefix.

Generated ids are namespaced automatically, with a prefix derived from a hash of
the finished drawing. That gives two useful properties:

* **No collisions.** Two different charts inlined into the same page keep their
  clip paths and gradients to themselves: an inline chart cannot steal another
  chart's definitions.
* **Deterministic output.** The same chart renders to the same bytes in every
  process, so golden files and HTTP caches keep working.

Two *identical* charts still produce identical ids; if you inline the same chart
twice, pass an explicit `with_id_prefix` to each.

## PDF tagging

The same roles drive a **tagged PDF**, the accessibility equivalent for
documents. `PdfOptions` selects it:

```rust
chart.to_pdf()?;                                    // tagged PDF (default)
chart.to_pdf_with(&PdfOptions::default()
    .with_a11y(A11y::Off))?;                        // plain PDF, no structure
```

The mapping lives in `Role::structure()`, next to the class mapping, so both
formats are driven by one description of the chart:

| Role | Structure element |
|---|---|
| `Background`, `Grid`, `Frame` | `/Artifact` (decorative, excluded from the tree) |
| `Plot` | `/Figure`, carrying the title and description as `/Alt` |
| `Title` | `/Caption` |
| `Axis`, `Legend`, `FacetLabel` | `/P` |

A tagged-document section cannot nest, so only *whole* accessible units start
one; tick labels, marks and legend entries belong to their enclosing unit. The
result is a structure tree like

```text
Document
└── Figure  /Alt "Output ⏎⏎ Yearly output, 2019 to 2022."
    ├── Caption   (the title)
    ├── <content> (the marks)
    ├── P         (the axes)
    └── P         (the legend)
```

PDF has no equivalent of the SVG's "navigable graphics" mode, so `A11y::Image`
and `A11y::Document` produce the same structure; `A11y::Off` disables tagging
entirely, so the file never claims a structure it does not have.
`PdfOptions::pretty()` writes an uncompressed, human-inspectable document
(handy for checking the tree, not for delivery).

> **Known limitation.** `krilla` 0.8 can only start marked-content sections
> named `/Artifact`, `/Span` or `/P`; there is no figure tag yet. The chart body
> is therefore *marked* as `/P` while its structure element is `/Figure` — the
> structure tree, which is what a reader navigates, is correct. This will be
> fixed when the PDF library exposes a figure content tag.

## Relationship to `Theme`

`Theme` describes how a chart *looks* and is baked into the output at render
time; `SvgOptions`/`PdfOptions` describe how the rendered document is
*serialised*. Keeping them apart is what makes "one chart, many presentations"
possible: the standalone asset, the inline accessible graphic and the printable
version differ in options, not in themes.

Note that layout is measured with the **theme's** font sizes. A stylesheet may
change colors, weights, dashes and visibility freely, and may nudge font sizes;
changing them a lot after the fact will overflow the bands the layout reserved.
For a different typographic scale, use `Theme::with_font_scale` (Rust) and let
CSS follow it.

## What we deliberately do not do

* **No inline `style` attributes.** Presentation attributes are overridden by
  CSS; an inline style would *win* over the page's stylesheet, defeating the
  point.
* **No `var()` in presentation attributes.** Browsers do not support custom
  properties there, and the only workaround is an inline style (see above). Use
  classes and override the properties instead.
* **No data values in class names.** See [Identity attributes](#identity-attributes).

See `examples/semantic_demo.rs` for a band-plus-line chart rendered three ways,
including an HTML page whose stylesheet restyles the inlined SVG.
