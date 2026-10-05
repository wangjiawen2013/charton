# Recipe Index

Named, copy-paste recipes for the charts people ask for by name. Each links to a
complete, compiled example — the image you see is the output of the code right
below it — and to the primitives it is built from.

> The convenience marks (`mark_violin`, `mark_density`, `mark_contour`,
> `mark_density_2d`, …) are just names for common compositions, so every entry's
> "Built from" column shows both the mark and the primitives it expands into.
> If your chart is missing, build it from the
> [Marks & Geometries](../grammar/marks.md) vocabulary and the
> [layer pipeline](../concepts/grammar_pipeline.md), then add it here.

## Distributions

| Recipe | Also known as | Built from |
| --- | --- | --- |
| [Density plot](density_1d.md#density-plot) | KDE | `mark_density` (`transform_density` + `mark_area`) |
| [Density heatmap](density_2d.md#density-heatmap) | 2-D KDE | `mark_density_2d` (`transform_density_2d` + `mark_rect`) |
| [Density contour](density_2d.md#density-contours) | 2-D KDE iso-lines | `mark_contour` over `transform_density_2d` |
| [Cumulative density](cumulative_density.md#cumulative-density-curve) | KDE integral | `mark_density` (`cumulative`) |
| [Empirical CDF](cumulative_density.md#empirical-cdf-cumulative-frequency) | cumulative frequency | `transform_window` (`CumeDist`) |
| [Violin](violin.md#violin-single) | density outline, upright | `mark_violin` |
| [Grouped violin](violin.md#grouped-and-faceted-violins) | dodged violin | `mark_violin` (`x` + `color`, dodge) |
| [Faceted violin](violin.md#grouped-and-faceted-violins) | small multiples | `mark_violin` + `facet` |
| [Split violin](violin.md#split-violin) | | `mark_violin` (`split`) |
| [Raincloud](violin.md#raincloud) | | `mark_violin` + `transform_quantile_box` + jittered points |
| [Ridgeline](primitives.md#a-ridgeline-from-a-half-band) | joyplot | `transform_density` + `transform_band` (`split`) + `mark_polygon` |
| [Violin with inner box](box_plot.md#the-composable-quantile-box) | | `mark_violin` + `transform_quantile_box` |
| [Box plot](box_plot.md#box-plot) | box & whiskers | `mark_boxplot` |
| [Histogram](histogram.md) | | `mark_hist` |
| [Scatter](point_charts.md) | | `mark_point` + shape/size/color |
| [Beeswarm](point_charts.md#beeswarm) | swarm | `mark_point` + layout |
| [Quasirandom](point_charts.md#quasirandom) | dot plot | `mark_point` + layout |
| [Strip plot](tick_chart.md#strip-plot) | rug | `mark_tick` |
| [Styled strip](tick_chart.md#strip-plot) | | `mark_tick` + `configure_tick` |

## Comparisons

| Recipe | Also known as | Built from |
| --- | --- | --- |
| [Bar with error bars](uncertainties_and_trends.md#basic-bar-with-error-bars) | mean ± error | `mark_bar` + `mark_errorbar` |
| [Grouped bar with error bars](uncertainties_and_trends.md#grouped-bar-with-errorbar) | | `mark_bar` + `mark_errorbar` + `Position::dodge` |
| [Stacked bar](uncertainties_and_trends.md#stacked-bar) | | `mark_bar` + `stack` |
| [Error bar](uncertainties_and_trends.md#error-bar-on-its-own) | | `mark_errorbar` |
| [Dumbbell](uncertainties_and_trends.md#dumbbell-connected-dot) | connected dot | `mark_rule` + `mark_point` |
| [Lollipop](uncertainties_and_trends.md#lollipop-and-range) | | `mark_rule` + `mark_point` |
| [Range plot](uncertainties_and_trends.md#lollipop-and-range) | intervals | `mark_rule` (`y` + `y2`) |
| [Waterfall](uncertainties_and_trends.md#waterfall) | running total | `mark_bar` (`y2`) + `transform_window` (`CumulativeSum`) |
| [Candlestick](uncertainties_and_trends.md#candlestick) | OHLC | `mark_rule` + `mark_bar` (`y2`) |

## Trends & time

| Recipe | Also known as | Built from |
| --- | --- | --- |
| [Line (multiple series)](line_charts.md#multiple-series) | | `mark_line` + `color` |
| [Cumulative frequency](line_charts.md#cumulative-frequency) | | `mark_line` |
| [Multi-series with error bars](line_charts.md#multi-series-with-error-bars) | | `mark_line` + `mark_errorbar` + `mark_point` |
| [Formatted axis labels](line_charts.md#formatted-axis-labels-economist-style) | `$20B`-style | `with_y_label_format` |
| [Slope chart](line_charts.md#slope-chart) | | `mark_line` + `mark_point` |
| [Bump chart](line_charts.md#bump-chart) | ranking | `transform_window` (`Rank`) + reversed axis |
| [Area chart](area_charts.md#area-chart) | | `mark_area` |
| [Simple stacked area](area_charts.md#simple-stacked-area-chart) | | `mark_area` + `stack: stacked` |
| [Normalized stacked area](area_charts.md#normalized-stacked-area-chart) | 100% area | `mark_area` + `stack: normalize` |
| [Steamgraph](area_charts.md#steamgraph) | streamgraph | `mark_area` + `stack: center` |

## Matrices & relationships

| Recipe | Also known as | Built from |
| --- | --- | --- |
| [Scatter](point_charts.md#shape-size-and-a-reference-grid) | | `mark_point` + shape/size/color |
| [Categorical heatmap](heatmaps.md#categorical-heatmap) | | `mark_rect` |
| [Continuous heatmap](heatmaps.md#continuous-heatmap-2-d-binning) | 2-D histogram | `mark_rect` + binning |
| [Iso-lines of a grid](contours.md#iso-lines-of-a-grid) | contour plot | `mark_contour` (`transform_contour` + `mark_path`) |
| [Single-colour contour](contours.md#how-the-pieces-fit) | | `mark_contour` without colour-by-level |
| [Density contours](contours.md#from-scattered-points) | 2-D KDE | `mark_contour` over `transform_density_2d` |

## Circular

| Recipe | Also known as | Built from |
| --- | --- | --- |
| [Pie](circular_charts.md#pie) | | `mark_bar` + `CoordSystem::Polar` |
| [Donut](circular_charts.md#donut) | | pie + inner radius |
| [Rose](circular_charts.md#rose-polar-bar) | polar bar | `mark_bar` + polar |
| [Nightingale](circular_charts.md#nightingale-polar-area) | coxcomb | `mark_bar` + polar |

## Layout & special

| Recipe | Also known as | Built from |
| --- | --- | --- |
| [Facet grid](faceting.md#grid-faceting-two-dimensions) | small multiples | `facet` |
| [World map](geospatial.md#world-map) | choropleth | `mark_geoshape` + `CoordSystem::Geo` |

## Choosing a distribution view

- **Histogram** — a first, honest look at the shape.
- **Density** — the same shape, smoothed; good when the distribution is
  interesting.
- **Box plot** — compact and precise about quartiles, hides the shape.
- **Violin** — the shape with quartiles available as an inner box.
- **Ridgeline** — many distributions compared at once, one density per category.
- **Beeswarm / strip** — every observation; good for small samples.

See [Missing Values & Gaps](../concepts/missing_values.md) for how each view
handles nulls and gaps.

## Going deeper

- [Composing Your Own](primitives.md) — recombining the primitives into new
  pictures the marks do not cover.
- [Contour Plots](contours.md) — the marching-squares machinery and the general
  scalar-grid contour.
- [Transforms & Columns](../grammar/transforms.md) — the column contract of
  every transform used by these recipes.
