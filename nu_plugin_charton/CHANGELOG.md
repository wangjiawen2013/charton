# Changelog

All notable changes to `nu_plugin_charton` are documented here. The crate
shares the `charton` version number, and the Nushell release it targets is
stated in the README and in each release.

## [0.7.1]

### Added

- Bundled sample datasets under `assets/`: `data.csv`, `sales.csv`, and
  `world.geojson`.

### Changed

- The Quick start examples now read the bundled `assets/` files, so they run
  out of the box from a clone.
- Simplified the install instructions for end users.

### Fixed

- Inline charts now leave the cursor at the start of the next line, so the
  shell prompt is no longer indented by the image width. All inline frames end
  with CRLF and a cursor resync for terminals whose image protocols park the
  cursor at the image edge.
- Polar bar charts now distinguish the two shapes charton defines: omitting `-x`
  produces a pie, or a donut with `--inner-radius`; keeping `-x` maps it to the
  angle for a rose / Nightingale chart. Previously every polar bar chart was a
  rose.

[0.7.1]: https://github.com/wangjiawen2013/charton/releases/tag/v0.7.1

## [0.7.0]

First public release. Targets Nushell 0.116.

### Added

- `charton` command: build a chart from a pipeline table and draw it inline, save
  it as SVG/PNG, or return the image data to the pipeline.
- Input handling: per-column type inference over all rows (int, float, string,
  bool, datetime; null-aware) from a Nushell `list<record>`.
- All charton marks: `point`, `line`, `area`, `bar`, `boxplot`, `errorbar`,
  `rule`, `tick`, `text`, `rect`/`heatmap`, `hist`, `density`/`kde`, `ecdf`,
  `beeswarm`, and `geo` (GeoJSON choropleth).
- Composition: multi-layer overlays (`--layer`), faceting (wrap and grid),
  cartesian/polar/geographic coordinates, and canvas margins.
- Statistics: kernel density estimation (`-g density`), empirical CDF
  (`-g ecdf`), aggregation (`--aggregate`), binning (`--bins`), and LOESS
  smoothing (`--loess`).
- Encodings: `x`/`y`/`y2`/`color`/`text`, plus `size`/`shape` channels and
  their legend labels.
- Scales and axes: linear/log/discrete/temporal scales, domain overrides, axis
  padding, explicit ticks, and label formatting.
- Appearance: light/dark themes, continuous color maps, palettes, background
  color, legend placement, and grid control, plus mark-level styling.
- Output: inline terminal rendering (Kitty graphics, iTerm2 inline images,
  Sixel, truecolor half-blocks) auto-detected with an override, SVG and PNG
  export, and `--raw` piping.
- Configuration via `$env.config.plugins.charton`.
- `charton-probe` diagnostic command behind an off-by-default `probe` feature.

[0.7.0]: https://github.com/wangjiawen2013/charton/releases/tag/v0.7.0
