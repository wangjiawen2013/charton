# Themes

A theme is a value, not a global. The same data and encodings can be drawn with
the light preset, the dark preset, or a theme you build, and the layout is
measured against the theme's own font sizes.

```rust
use charton::prelude::*;

let chart = chart!(height, weight)?
    .mark_point()?
    .encode((alt::x("height"), alt::y("weight")))?
    .with_theme(
        Theme::dark()
            .with_font_scale(1.2)
            .with_palette(ColorPalette::Tab10)
            .with_show_legend(false),
    );
```

`Theme::light()` and `Theme::dark()` are the two presets. `LayeredChart::with_theme`
replaces the theme; `configure_theme(|theme| …)` edits the current one, which is
the safer choice when a chart already carries a preset:

```rust
let chart = chart.configure_theme(|theme| {
    theme
        .with_background_color("#fafafa")
        .with_tick_count(6)
});
```

## What a theme controls

| Group | Examples |
|---|---|
| Canvas and spacing | `with_top_margin`, `with_bottom_margin`, `with_left_margin`, `with_right_margin` |
| Type | `with_title_size`, `with_label_size`, `with_tick_label_size`, `with_font_scale`, `with_*_family`, `with_*_color` |
| Axes | `with_axis_width`, `with_tick_length`, `with_tick_count`, `with_x_tick_label_angle`, `with_grid`, `with_grid_color` |
| Legend | `with_show_legend`, `with_legend_position`, `with_legend_*_size`, `with_legend_margin` |
| Colour | `with_palette`, `with_color_map` |
| Layout guardrails | `with_min_panel_size`, `with_panel_defense_ratio`, `with_axis_reserve_buffer` |

`with_font_scale` multiplies every text size **and** the spacing around it. It is
the right knob for a dense panel or a terminal-sized chart; changing individual
sizes one by one breaks the measured layout.

## Layout is measured, not guessed

The space manager reserves room for guides before it gives the panel what is
left. Two theme values keep that honest:

- `axis_reserve_buffer` is the margin the layout keeps around the plotting panel
  for ticks and labels.
- `panel_defense_ratio` is the minimum share of the canvas the panel keeps. A
  legend or a long category list cannot shrink the data region to a sliver.

Rotated labels are measured as parallel text lines, so `with_x_tick_label_angle`
recomputes the space it needs instead of clipping.

## Theme or stylesheet?

A theme is baked into the output at render time. When the *same* chart has to
appear on a dark page, in print, or in a tenant's brand colours, put that styling
in the host page's CSS, addressing the exported document's semantic classes
(see [Semantic Output: CSS, Accessibility & Tagging](../export/semantic_output.md)).

One rule a stylesheet cannot replace: layout is measured with the theme's font
sizes, so a large typographic change still belongs in `Theme::with_font_scale`.
