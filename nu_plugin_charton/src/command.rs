//! The `charton` command: read a pipeline table, build a charton chart, then
//! either render it inline in the terminal, save it, or return the raw data.

use std::io::{IsTerminal, Write};
use std::path::Path;

use charton::error::ChartonError;
use charton::prelude::{
    BandScale, BandTransform, BandwidthType, Chart, ColorMap, ColorPalette, ContourTransform,
    CoordSystem, Dataset, Density2DTransform, DensityTransform, Expansion, FacetSpec, IntoLayered,
    KernelType, LabelFormat, LayeredChart, MarkArea, MarkBar, MarkBoxplot, MarkErrorBar,
    MarkGeoPath, MarkLine, MarkPoint, MarkRect, MarkRule, MarkText, MarkTick, Position,
    QuantileBoxTransform, Scale, ThemeMode, WindowFieldDef, WindowOnlyOp, WindowTransform, alt,
    geojson_to_dataset,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Example, IntoPipelineData, LabeledError, PipelineData, Record, Signature, Span, SyntaxShape,
    Type, Value,
};

use crate::ChartonPlugin;
use crate::config::{self, Config, LegendSetting};
use crate::converter::Table;
use crate::render;

pub struct Charton;

impl PluginCommand for Charton {
    type Plugin = ChartonPlugin;

    fn name(&self) -> &str {
        "charton"
    }

    fn description(&self) -> &str {
        "Render a pipeline table as a chart (inline terminal, SVG, or PNG)"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .named(
                "geom",
                SyntaxShape::String,
                "Mark type: point | line | area | bar | boxplot | violin | errorbar | rule | tick | text | rect | hist | density | ecdf | contour | beeswarm | geo",
                Some('g'),
            )
            .named("x", SyntaxShape::String, "Column mapped to the x axis", Some('x'))
            .named("y", SyntaxShape::String, "Column mapped to the y axis", Some('y'))
            .named(
                "z",
                SyntaxShape::String,
                "Value column for -g contour; omit it to estimate a 2D density first",
                None,
            )
            .named(
                "color",
                SyntaxShape::String,
                "Column mapped to color/grouping",
                Some('c'),
            )
            .named(
                "output",
                SyntaxShape::String,
                "Save the chart to a file (.svg or .png)",
                Some('o'),
            )
            .named(
                "title",
                SyntaxShape::String,
                "Chart title",
                Some('t'),
            )
            .named("x-label", SyntaxShape::String, "X axis label", None)
            .named("y-label", SyntaxShape::String, "Y axis label", None)
            .named("color-label", SyntaxShape::String, "Color legend label", None)
            .named("x-min", SyntaxShape::Number, "Override the x axis minimum", None)
            .named("x-max", SyntaxShape::Number, "Override the x axis maximum", None)
            .named("y-min", SyntaxShape::Number, "Override the y axis minimum", None)
            .named("y-max", SyntaxShape::Number, "Override the y axis maximum", None)
            .switch("flip", "Swap the x and y axes", None)
            .named(
                "coord",
                SyntaxShape::String,
                "Coordinate system: cartesian (default) | polar (rose, or pie when --x is omitted)",
                None,
            )
            .named(
                "inner-radius",
                SyntaxShape::Number,
                "Polar inner radius ratio 0.0-1.0 (turns a pie into a donut)",
                None,
            )
            .named(
                "start-angle",
                SyntaxShape::Number,
                "Polar start angle in degrees (default 0)",
                None,
            )
            .named(
                "end-angle",
                SyntaxShape::Number,
                "Polar end angle in degrees (default 360)",
                None,
            )
            .named(
                "stack",
                SyntaxShape::String,
                "Bar/area stacking: none | stacked | normalize | center",
                None,
            )
            .switch(
                "normalize",
                "Normalize y values (hist/bar) to proportions",
                None,
            )
            .named(
                "aggregate",
                SyntaxShape::String,
                "Aggregate y per x group: sum | mean | median | min | max | count",
                None,
            )
            .named(
                "bins",
                SyntaxShape::Int,
                "Number of bins for a continuous x axis",
                None,
            )
            .named(
                "x-expand",
                SyntaxShape::Number,
                "Padding added to both ends of the x axis (fraction of the range)",
                None,
            )
            .named(
                "y-expand",
                SyntaxShape::Number,
                "Padding added to both ends of the y axis (fraction of the range)",
                None,
            )
            .named(
                "x-ticks",
                SyntaxShape::Any,
                "Explicit x tick values, e.g. [0 1 2 3]",
                None,
            )
            .named(
                "y-ticks",
                SyntaxShape::Any,
                "Explicit y tick values, e.g. [0 1 2 3]",
                None,
            )
            .named(
                "margins",
                SyntaxShape::String,
                "Canvas margins as 'top,right,bottom,left' (fractions 0.0-1.0)",
                None,
            )
            .named(
                "density-bandwidth",
                SyntaxShape::Number,
                "KDE bandwidth for -g density (in data units; default Scott's rule)",
                None,
            )
            .named(
                "density-kernel",
                SyntaxShape::String,
                "KDE kernel for -g density: normal | epanechnikov | uniform",
                None,
            )
            .switch("cumulative", "-g density: cumulative density instead of density", None)
            .switch("counts", "-g density: smoothed counts instead of probabilities", None)
            .switch("grid", "Show grid lines", None)
            .switch("no-grid", "Hide grid lines", None)
            .named(
                "legend",
                SyntaxShape::String,
                "Legend position: left | right | top | bottom | none",
                None,
            )
            .named(
                "theme",
                SyntaxShape::String,
                "Color theme: auto | light | dark (default auto)",
                None,
            )
            .named("x-angle", SyntaxShape::Number, "X tick label angle in degrees", None)
            .named("mark-color", SyntaxShape::String, "Fixed color for the mark", None)
            .named("opacity", SyntaxShape::Number, "Mark opacity 0.0-1.0", None)
            .named("size", SyntaxShape::Number, "Mark size (point/text)", None)
            .named("stroke", SyntaxShape::String, "Stroke color for the mark", None)
            .named(
                "stroke-width",
                SyntaxShape::Number,
                "Stroke width for the mark",
                None,
            )
            .named(
                "mark-width",
                SyntaxShape::Number,
                "Mark band width as a fraction 0.0-1.0 (bar/box/point/errorbar)",
                None,
            )
            .named(
                "cap-length",
                SyntaxShape::Number,
                "Errorbar cap length in pixels",
                None,
            )
            .switch("no-center", "Hide the center dot of error bars", None)
            .switch("no-outliers", "Hide boxplot outlier points", None)
            .named(
                "outlier-size",
                SyntaxShape::Number,
                "Boxplot outlier point size",
                None,
            )
            .named(
                "size-by",
                SyntaxShape::String,
                "Column mapped to point size (bubble charts; -g point/beeswarm)",
                None,
            )
            .named(
                "shape-by",
                SyntaxShape::String,
                "Column mapped to point shape (-g point/beeswarm)",
                None,
            )
            .named("size-label", SyntaxShape::String, "Size legend label", None)
            .named("shape-label", SyntaxShape::String, "Shape legend label", None)
            .named(
                "shape",
                SyntaxShape::String,
                "Point shape: circle | square | triangle | star | diamond | pentagon | hexagon | octagon",
                None,
            )
            .named(
                "dash",
                SyntaxShape::String,
                "Dash pattern for lines, e.g. '6,4' (on, off) in pixels",
                None,
            )
            .named("interpolation", SyntaxShape::String, "Line interpolation: linear | step | step-before", None)
            .switch("loess", "Smooth the line with LOESS", None)
            .named(
                "loess-bandwidth",
                SyntaxShape::Number,
                "LOESS bandwidth 0.0-1.0",
                None,
            )
            .named(
                "outlier-color",
                SyntaxShape::String,
                "Outlier point color for -g boxplot",
                None,
            )
            .named(
                "anchor",
                SyntaxShape::String,
                "Text horizontal anchor: start | middle | end",
                None,
            )
            .named(
                "weight",
                SyntaxShape::String,
                "Text font weight: normal | bold | 100..900",
                None,
            )
            .named(
                "layout",
                SyntaxShape::String,
                "Point layout: standard | jitter | beeswarm | quasirandom",
                None,
            )
            .named(
                "quasirandom-method",
                SyntaxShape::String,
                "Quasirandom pairing: tukey | pseudorandom",
                None,
            )
            .named(
                "x-scale",
                SyntaxShape::String,
                "X axis scale: linear | log | discrete | temporal",
                None,
            )
            .named(
                "y-scale",
                SyntaxShape::String,
                "Y axis scale: linear | log | discrete | temporal",
                None,
            )
            .named(
                "color-map",
                SyntaxShape::String,
                "Continuous color map, e.g. viridis | magma | ylgnbu",
                None,
            )
            .named(
                "x-format",
                SyntaxShape::Any,
                "X tick label format: a preset like 'compact' or a record {prefix, suffix, precision, compact, thousands, multiplier}",
                None,
            )
            .named(
                "y-format",
                SyntaxShape::Any,
                "Y tick label format (same forms as --x-format)",
                None,
            )
            .named(
                "legend-format",
                SyntaxShape::Any,
                "Legend/colorbar label format (same forms as --x-format)",
                None,
            )
            .named(
                "background",
                SyntaxShape::String,
                "Chart background color (overrides --theme)",
                None,
            )
            .named(
                "width",
                SyntaxShape::Int,
                "Pixel width of the rendered chart",
                None,
            )
            .named(
                "height",
                SyntaxShape::Int,
                "Pixel height of the rendered chart",
                None,
            )
            .switch(
                "raw",
                "Return the raw image data (SVG string, or PNG with --png) instead of drawing inline",
                None,
            )
            .switch("png", "With --raw, return PNG bytes instead of SVG", None)
            .switch("no-inline", "Do not draw inline even if the terminal supports it", None)
            .switch(
                "force-inline",
                "Draw inline even when stdout is not detected as a terminal (testing/override)",
                None,
            )
            .named(
                "inline-style",
                SyntaxShape::String,
                "Inline renderer: auto | halfblock | iterm2 | kitty | sixel",
                None,
            )
            .named(
                "y2",
                SyntaxShape::String,
                "Upper bound column for -g errorbar/-g rule (errorbar aggregates mean +/- std without it)",
                None,
            )
            .named(
                "text",
                SyntaxShape::String,
                "Label column for -g text",
                None,
            )
            .named(
                "layer",
                SyntaxShape::Any,
                "Extra layer(s): a record or list of records with geom/x/y/y2/color/text",
                None,
            )
            .named(
                "geojson",
                SyntaxShape::String,
                "GeoJSON file to render with -g geo",
                None,
            )
            .named(
                "scale",
                SyntaxShape::Number,
                "Raster pixel scale factor (default 2.0)",
                None,
            )
            .named(
                "facet-wrap",
                SyntaxShape::String,
                "Wrap panels by this column",
                None,
            )
            .named(
                "facet-columns",
                SyntaxShape::Int,
                "Number of columns for --facet-wrap",
                None,
            )
            .named(
                "facet-row",
                SyntaxShape::String,
                "Row field for a facet grid",
                None,
            )
            .named(
                "facet-col",
                SyntaxShape::String,
                "Column field for a facet grid",
                None,
            )
            .named(
                "facet-strategy",
                SyntaxShape::String,
                "Facet scale strategy: fixed | free (default fixed)",
                None,
            )
            .input_output_type(Type::Any, Type::Any)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "plot",
            "chart",
            "graph",
            "visualize",
            "svg",
            "png",
            "charton",
        ]
    }

    fn extra_description(&self) -> &str {
        "\
Turns any pipeline table into a chart. The columns named by the flags drive the \
plot; everything else is set by options.

`point` is the default geom. Use `-c` to colour and group, `--stack` to stack, \
`--aggregate` to summarise, `--facet-wrap` / `--facet-row` / `--facet-col` to \
split the chart into panels, and `--layer` to overlay another geom. The result \
is drawn inline in an image-capable terminal, or written to a file with \
`-o chart.svg` or `-o chart.png`.

A few chart types are compositions rather than single marks: `violin` is a \
density outline plus an inner quartile box, `beeswarm` is `point` with a swarm \
layout, and `contour` is marching-squares iso-lines. The README has one runnable \
line per geom."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Scatter plot of two columns",
                example: "[[x y]; [1 2] [2 3] [3 5]] | charton -g point -x x -y y",
                result: None,
            },
            Example {
                description: "Save a line chart to an SVG file",
                example: "open assets/data.csv | charton -g line -x date -y value -o chart.svg",
                result: None,
            },
            Example {
                description: "A violin per category, dodged by a group",
                example: "open assets/data.csv | charton -g violin -x category -y score -c group",
                result: None,
            },
            Example {
                description: "Iso-lines of a scalar x/y/z grid",
                example: "open assets/grid.csv | charton -g contour -x x -y y --z z -o contour.svg",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _plugin: &ChartonPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let span = call.head;

        let cfg = Config::load(engine)
            .map_err(|e| LabeledError::new("Invalid charton config").with_label(e, span))?;

        let geom = call
            .get_flag::<String>("geom")?
            .unwrap_or_else(|| "point".to_string())
            .to_lowercase();
        let x: Option<String> = call.get_flag("x")?;
        let y: Option<String> = call.get_flag("y")?;
        let z: Option<String> = call.get_flag("z")?;
        let color: Option<String> = call.get_flag("color")?;
        let y2: Option<String> = call.get_flag("y2")?;
        let text: Option<String> = call.get_flag("text")?;
        let geojson: Option<String> = call.get_flag("geojson")?;
        let output: Option<String> = call.get_flag("output")?;
        let title: Option<String> = call.get_flag("title")?;
        let x_label: Option<String> = call.get_flag("x-label")?;
        let y_label: Option<String> = call.get_flag("y-label")?;
        let color_label: Option<String> = call.get_flag("color-label")?;
        let x_min = call.get_flag::<f64>("x-min")?;
        let x_max = call.get_flag::<f64>("x-max")?;
        let y_min = call.get_flag::<f64>("y-min")?;
        let y_max = call.get_flag::<f64>("y-max")?;
        let flip = call.has_flag("flip")?;
        let legend_flag: Option<String> = call.get_flag("legend")?;
        let x_angle_flag = call.get_flag::<f64>("x-angle")?;
        let style = Style {
            color: call.get_flag("mark-color")?,
            opacity: call.get_flag::<f64>("opacity")?,
            size: call.get_flag::<f64>("size")?,
            stroke: call.get_flag("stroke")?,
            stroke_width: call.get_flag::<f64>("stroke-width")?,
            shape: call.get_flag("shape")?,
            dash: call
                .get_flag::<String>("dash")?
                .map(|s| parse_dash(&s, span))
                .transpose()?,
            interpolation: call.get_flag("interpolation")?,
            outlier_color: call.get_flag("outlier-color")?,
            anchor: call.get_flag("anchor")?,
            weight: call.get_flag("weight")?,
            layout: call.get_flag("layout")?,
            quasirandom_method: call.get_flag("quasirandom-method")?,
            mark_width: call.get_flag("mark-width")?,
            cap_length: call.get_flag("cap-length")?,
            center: if call.has_flag("no-center")? {
                Some(false)
            } else {
                None
            },
            show_outliers: if call.has_flag("no-outliers")? {
                Some(false)
            } else {
                None
            },
            outlier_size: call.get_flag("outlier-size")?,
            loess: call.has_flag("loess")?,
            loess_bandwidth: call.get_flag("loess-bandwidth")?,
        };
        let x_scale = call
            .get_flag::<String>("x-scale")?
            .map(|s| parse_scale(&s, span))
            .transpose()?;
        let y_scale = call
            .get_flag::<String>("y-scale")?
            .map(|s| parse_scale(&s, span))
            .transpose()?;
        let color_map = match call.get_flag::<String>("color-map")? {
            Some(name) => Some(
                config::to_color_map(&name)
                    .map_err(|e| LabeledError::new("Invalid color map").with_label(e, span))?,
            ),
            None => cfg.color_map,
        };
        let x_format = call
            .get_flag::<Value>("x-format")?
            .map(|v| parse_label_format(&v, span))
            .transpose()?;
        let y_format = call
            .get_flag::<Value>("y-format")?
            .map(|v| parse_label_format(&v, span))
            .transpose()?;
        let legend_format = call
            .get_flag::<Value>("legend-format")?
            .map(|v| parse_label_format(&v, span))
            .transpose()?;
        let background = call
            .get_flag::<String>("background")?
            .or_else(|| cfg.background.clone());
        let coord = parse_coord(call.get_flag::<String>("coord")?, span)?;
        let inner_radius = call.get_flag::<f64>("inner-radius")?;
        let start_angle = call.get_flag::<f64>("start-angle")?;
        let end_angle = call.get_flag::<f64>("end-angle")?;
        let stack = match call.get_flag::<String>("stack")? {
            Some(s) => Some(parse_stack(&s, span)?),
            None => None,
        };
        let normalize = call.has_flag("normalize")?;
        let size_by: Option<String> = call.get_flag("size-by")?;
        let shape_by: Option<String> = call.get_flag("shape-by")?;
        let size_label: Option<String> = call.get_flag("size-label")?;
        let shape_label: Option<String> = call.get_flag("shape-label")?;
        let aggregate = match call.get_flag::<String>("aggregate")? {
            Some(s) => Some(parse_aggregate(&s, span)?),
            None => None,
        };
        let bins = call.get_flag::<i64>("bins")?.map(|n| n.max(1) as usize);
        let x_expand = call.get_flag::<f64>("x-expand")?;
        let y_expand = call.get_flag::<f64>("y-expand")?;
        let margins = match call.get_flag::<String>("margins")? {
            Some(s) => Some(parse_margins(&s, span)?),
            None => None,
        };
        let x_ticks = parse_ticks(call.get_flag::<Value>("x-ticks")?, span)?;
        let y_ticks = parse_ticks(call.get_flag::<Value>("y-ticks")?, span)?;
        let density_bandwidth = call.get_flag::<f64>("density-bandwidth")?;
        let density_kernel = match call.get_flag::<String>("density-kernel")? {
            Some(s) => Some(parse_kernel(&s, span)?),
            None => None,
        };
        let density_cumulative = call.has_flag("cumulative")?;
        let density_counts = call.has_flag("counts")?;
        let raw = call.has_flag("raw")?;
        let want_png = call.has_flag("png")?;
        let no_inline = call.has_flag("no-inline")?;
        let force_inline = call.has_flag("force-inline")?;

        // Precedence: flag > plugin config > default.
        let width = call
            .get_flag::<i64>("width")?
            .map(|v| v.max(16) as u32)
            .or(cfg.width)
            .unwrap_or(800);
        let height = call
            .get_flag::<i64>("height")?
            .map(|v| v.max(16) as u32)
            .or(cfg.height)
            .unwrap_or(600);
        let scale_flag = call.get_flag::<f64>("scale")?;
        // Remember whether the user chose a scale: inline fitting only kicks in
        // when the resolution was left to us.
        let scale_explicit = scale_flag.is_some() || cfg.scale.is_some();
        let scale = scale_flag
            .map(|v| v as f32)
            .or(cfg.scale)
            .filter(|s| *s > 0.0)
            .unwrap_or(2.0);
        let cell_w = cfg
            .cell_width
            .map(|v| v as usize)
            .unwrap_or(render::DEFAULT_CELL_W);
        let cell_h = cfg
            .cell_height
            .map(|v| v as usize)
            .unwrap_or(render::DEFAULT_CELL_H);
        let palette = match &cfg.palette {
            Some(p) => Some(
                config::to_color_palette(p)
                    .map_err(|e| LabeledError::new("Invalid palette").with_label(e, span))?,
            ),
            None => None,
        };

        let inline_style = render::InlineStyle::parse(
            &call
                .get_flag::<String>("inline-style")?
                .or_else(|| cfg.inline_style.clone())
                .unwrap_or_else(|| "auto".to_string()),
        )
        .map_err(|e| LabeledError::new("Invalid inline style").with_label(e, span))?;

        // Faceting: --facet-wrap, or --facet-row + --facet-col.
        let facet_strategy = call
            .get_flag::<String>("facet-strategy")?
            .unwrap_or_else(|| "fixed".to_string());
        let facet = match (
            call.get_flag::<String>("facet-wrap")?,
            call.get_flag::<String>("facet-row")?,
            call.get_flag::<String>("facet-col")?,
        ) {
            (Some(field), _, _) => {
                let mut spec = FacetSpec::wrap(&field).with_strategy(facet_strategy.as_str());
                if let Some(cols) = call.get_flag::<i64>("facet-columns")? {
                    spec = spec.with_columns(cols.max(1) as usize);
                }
                Some(spec)
            }
            (None, Some(row), Some(col)) => {
                Some(FacetSpec::grid(&row, &col).with_strategy(facet_strategy.as_str()))
            }
            (None, Some(_), None) | (None, None, Some(_)) => {
                return Err(LabeledError::new("Incomplete facet")
                    .with_label("--facet-row and --facet-col must be given together", span));
            }
            (None, None, None) => None,
        };

        // 1. Ingest the pipeline and resolve file-backed inputs.
        let values: Vec<Value> = input.into_iter().collect();
        let geojson_path = match geojson.as_deref() {
            Some(p) => {
                let requested = Path::new(p);
                Some(if requested.is_absolute() {
                    requested.to_path_buf()
                } else {
                    Path::new(&engine.get_current_dir()?).join(requested)
                })
            }
            None => None,
        };

        // 2. Combine the primary layer with any `--layer` overlays.
        let primary = LayerOpts {
            geom: geom.clone(),
            x: x.clone(),
            y: y.clone(),
            z: z.clone(),
            y2: y2.clone(),
            color: color.clone(),
            text: text.clone(),
            style: style.clone(),
            x_scale,
            y_scale,
            stack,
            normalize,
            size_by,
            shape_by,
            aggregate,
            bins,
            x_expand,
            y_expand,
            density_bandwidth,
            density_kernel,
            density_cumulative,
            density_counts,
        };
        let layers = parse_layers(call.get_flag::<Value>("layer")?, &primary, span)?;

        // Domain overrides must be complete pairs.
        let x_domain = domain_pair(x_min, x_max, "--x-min", "--x-max", span)?;
        let y_domain = domain_pair(y_min, y_max, "--y-min", "--y-max", span)?;

        let grid = if call.has_flag("grid")? {
            Some(true)
        } else if call.has_flag("no-grid")? {
            Some(false)
        } else {
            cfg.grid
        };
        let legend = match legend_flag {
            Some(s) => Some(
                LegendSetting::parse(&s)
                    .map_err(|e| LabeledError::new("Invalid legend").with_label(e, span))?,
            ),
            None => cfg.legend,
        };
        let theme = resolve_theme(
            call.get_flag::<String>("theme")?
                .or_else(|| cfg.theme.clone()),
            span,
        )?;

        // 3. Build the (layered) chart.
        let opts = BuildOpts {
            primary,
            layers,
            title: title.as_deref(),
            x_label: x_label.as_deref(),
            y_label: y_label.as_deref(),
            color_label: color_label.as_deref(),
            x_domain,
            y_domain,
            flip,
            geojson: geojson_path.as_deref(),
            facet,
            grid,
            palette,
            legend,
            x_angle: x_angle_flag.or(cfg.x_angle),
            background,
            color_map,
            x_format,
            y_format,
            legend_format,
            coord,
            inner_radius,
            start_angle,
            end_angle,
            size_label: size_label.as_deref(),
            shape_label: shape_label.as_deref(),
            margins,
            x_ticks,
            y_ticks,
            theme,
            width,
            height,
            scale,
            span,
        };
        let chart = build_chart(&values, &opts)?;

        // 4. Decide how to deliver the result.
        let can_inline = !raw
            && !no_inline
            && !engine.is_using_stdio()
            && (std::io::stdout().is_terminal() || force_inline);

        // 3a. Save to file if requested (works alongside inline display).
        //     Plugins run with their executable's directory as the process CWD,
        //     so relative paths must be resolved against the shell context.
        if let Some(path) = &output {
            let requested = Path::new(path);
            let resolved = if requested.is_absolute() {
                requested.to_path_buf()
            } else {
                Path::new(&engine.get_current_dir()?).join(requested)
            };
            write_output(&chart, &resolved, span)?;
        }

        // 3b. Inline terminal rendering. Only safe when stdout is the terminal
        //     (local-socket mode); never in stdio mode.
        if can_inline {
            let (cols, rows) = terminal_size::terminal_size()
                .map(|(w, h)| (w.0 as usize, h.0 as usize))
                .unwrap_or((100, 30));
            let max_cols = cols.saturating_sub(1).max(20);
            let max_rows = rows.saturating_sub(2).max(6);

            let style = if inline_style == render::InlineStyle::Auto {
                render::InlineStyle::detect()
            } else {
                inline_style
            };

            // Protocols that hand a bitmap to the terminal (iTerm2, Kitty,
            // Sixel) look best when the raster already matches the terminal's
            // pixel area: the terminal then scales little or not at all, which
            // keeps text crisp and strokes even. Half-blocks are downsampled by
            // us per cell, so they keep the full-resolution raster.
            let terminal_sized = !scale_explicit
                && matches!(
                    style,
                    render::InlineStyle::Iterm2
                        | render::InlineStyle::Kitty
                        | render::InlineStyle::Sixel
                );
            let inline_chart = if terminal_sized {
                let fitted =
                    render::inline_scale(width, height, max_cols, max_rows, cell_w, cell_h);
                chart.clone().with_scale_factor(fitted)
            } else {
                chart.clone()
            };
            let bytes = render::to_png(&inline_chart)
                .map_err(|e| LabeledError::new("PNG rendering failed").with_label(e, span))?;

            let art = match style {
                render::InlineStyle::Iterm2 => {
                    let (w, h) = render::png_dimensions(&bytes).map_err(|e| {
                        LabeledError::new("Inline rendering failed").with_label(e, span)
                    })?;
                    let (c, r) =
                        render::fit_cells_with_cell(w, h, max_cols, max_rows, cell_w, cell_h);
                    render::iterm2_image(&bytes, c, r)
                }
                render::InlineStyle::Kitty => {
                    let (w, h) = render::png_dimensions(&bytes).map_err(|e| {
                        LabeledError::new("Inline rendering failed").with_label(e, span)
                    })?;
                    let (c, r) =
                        render::fit_cells_with_cell(w, h, max_cols, max_rows, cell_w, cell_h);
                    render::kitty_image(&bytes, c, r)
                }
                render::InlineStyle::Sixel => render::sixel_image(
                    &bytes, max_cols, max_rows, cell_w, cell_h,
                )
                .map_err(|e| LabeledError::new("Inline rendering failed").with_label(e, span))?,
                // `Auto` already resolved above; treat as the universal fallback.
                render::InlineStyle::HalfBlock | render::InlineStyle::Auto => {
                    render::png_to_halfblock(&bytes, max_cols, max_rows).map_err(|e| {
                        LabeledError::new("Inline rendering failed").with_label(e, span)
                    })?
                }
            };

            let mut out = std::io::stdout();
            let _ = out.write_all(art.as_bytes());
            // Put the cursor at the start of the next line. The backspace clears
            // any column that an image protocol left behind, so following output
            // (usually the shell prompt) starts at the left margin.
            let _ = out.write_all(b"\x08\r");
            let _ = out.flush();
            return Ok(Value::nothing(span).into_pipeline_data());
        }

        // 3c. Raw data for further piping.
        if raw {
            if want_png {
                let bytes = render::to_png(&chart)
                    .map_err(|e| LabeledError::new("PNG rendering failed").with_label(e, span))?;
                return Ok(Value::binary(bytes, span).into_pipeline_data());
            }
            let svg = render::to_svg(&chart)
                .map_err(|e| LabeledError::new("SVG rendering failed").with_label(e, span))?;
            return Ok(Value::string(svg, span).into_pipeline_data());
        }

        // 3d. Saved already? return nothing, else hand back SVG text.
        if output.is_some() {
            Ok(Value::nothing(span).into_pipeline_data())
        } else {
            let svg = render::to_svg(&chart)
                .map_err(|e| LabeledError::new("SVG rendering failed").with_label(e, span))?;
            Ok(Value::string(svg, span).into_pipeline_data())
        }
    }
}

/// One chart layer: the mark to draw and the columns that feed it.
///
/// Only `geom` is required. When a `--layer` overlay omits a field, it falls
/// back to the primary layer, so an overlay only states what it changes.
struct LayerOpts {
    geom: String,
    x: Option<String>,
    y: Option<String>,
    /// Scalar field for `-g contour`.
    z: Option<String>,
    y2: Option<String>,
    color: Option<String>,
    text: Option<String>,
    /// Style for this layer. Extra layers inherit the primary's unless the
    /// layer record carries its own `style` record.
    style: Style,
    x_scale: Option<Scale>,
    y_scale: Option<Scale>,
    stack: Option<String>,
    normalize: bool,
    size_by: Option<String>,
    shape_by: Option<String>,
    aggregate: Option<String>,
    bins: Option<usize>,
    x_expand: Option<f64>,
    y_expand: Option<f64>,
    density_bandwidth: Option<f64>,
    density_kernel: Option<KernelType>,
    density_cumulative: bool,
    density_counts: bool,
}

struct BuildOpts<'a> {
    primary: LayerOpts,
    layers: Vec<LayerOpts>,
    title: Option<&'a str>,
    x_label: Option<&'a str>,
    y_label: Option<&'a str>,
    color_label: Option<&'a str>,
    x_domain: Option<(f64, f64)>,
    y_domain: Option<(f64, f64)>,
    flip: bool,
    geojson: Option<&'a Path>,
    facet: Option<FacetSpec>,
    grid: Option<bool>,
    palette: Option<ColorPalette>,
    legend: Option<LegendSetting>,
    x_angle: Option<f64>,
    background: Option<String>,
    color_map: Option<ColorMap>,
    x_format: Option<LabelFormat>,
    y_format: Option<LabelFormat>,
    legend_format: Option<LabelFormat>,
    coord: Option<CoordSystem>,
    inner_radius: Option<f64>,
    start_angle: Option<f64>,
    end_angle: Option<f64>,
    size_label: Option<&'a str>,
    shape_label: Option<&'a str>,
    margins: Option<[f64; 4]>,
    x_ticks: Option<Vec<f64>>,
    y_ticks: Option<Vec<f64>>,
    theme: ThemeMode,
    width: u32,
    height: u32,
    scale: f32,
    span: Span,
}

/// Mark-level visual overrides shared by every layer.
///
/// A mark applies the options it supports and ignores the rest, so the same
/// style works across marks (for example `--size` on a bar). A `--layer` may
/// carry its own style record, which is merged over this one.
#[derive(Clone, Default)]
struct Style {
    color: Option<String>,
    opacity: Option<f64>,
    size: Option<f64>,
    stroke: Option<String>,
    stroke_width: Option<f64>,
    shape: Option<String>,
    dash: Option<Vec<f64>>,
    interpolation: Option<String>,
    outlier_color: Option<String>,
    anchor: Option<String>,
    weight: Option<String>,
    layout: Option<String>,
    quasirandom_method: Option<String>,
    mark_width: Option<f64>,
    cap_length: Option<f64>,
    /// `Some(false)` when `--no-center` is given; `None` keeps the default.
    center: Option<bool>,
    /// `Some(false)` when `--no-outliers` is given.
    show_outliers: Option<bool>,
    outlier_size: Option<f64>,
    loess: bool,
    loess_bandwidth: Option<f64>,
}

fn style_point(mut m: MarkPoint, s: &Style) -> MarkPoint {
    if let Some(c) = &s.color {
        m = m.with_color(c.as_str());
    }
    if let Some(o) = s.opacity {
        m = m.with_opacity(o);
    }
    if let Some(v) = s.size {
        m = m.with_size(v);
    }
    if let Some(c) = &s.stroke {
        m = m.with_stroke(c.as_str());
    }
    if let Some(w) = s.stroke_width {
        m = m.with_stroke_width(w);
    }
    if let Some(shape) = &s.shape {
        m = m.with_shape(shape.as_str());
    }
    if let Some(layout) = &s.layout {
        m = m.with_layout(layout.as_str());
    }
    if let Some(method) = &s.quasirandom_method {
        m = m.with_quasirandom_method(method.as_str());
    }
    if let Some(w) = s.mark_width {
        m = m.with_width(w);
    }
    m
}

fn style_line(mut m: MarkLine, s: &Style) -> MarkLine {
    if let Some(c) = &s.color {
        m = m.with_color(c.as_str());
    }
    if let Some(o) = s.opacity {
        m = m.with_opacity(o);
    }
    if let Some(w) = s.stroke_width {
        m = m.with_stroke_width(w);
    }
    if let Some(dash) = &s.dash {
        m = m.with_dash(dash.clone());
    }
    if let Some(interpolation) = &s.interpolation {
        m = m.with_interpolation(interpolation.as_str());
    }
    if s.loess || s.loess_bandwidth.is_some() {
        m = m.with_loess(true);
    }
    if let Some(bw) = s.loess_bandwidth {
        m = m.with_loess_bandwidth(bw);
    }
    m
}

fn style_bar(mut m: MarkBar, s: &Style) -> MarkBar {
    if let Some(c) = &s.color {
        m = m.with_color(c.as_str());
    }
    if let Some(o) = s.opacity {
        m = m.with_opacity(o);
    }
    if let Some(c) = &s.stroke {
        m = m.with_stroke(c.as_str());
    }
    if let Some(w) = s.stroke_width {
        m = m.with_stroke_width(w);
    }
    if let Some(w) = s.mark_width {
        m = m.with_width(w);
    }
    m
}

fn style_area(mut m: MarkArea, s: &Style) -> MarkArea {
    if let Some(c) = &s.color {
        m = m.with_color(c.as_str());
    }
    if let Some(o) = s.opacity {
        m = m.with_opacity(o);
    }
    if let Some(c) = &s.stroke {
        m = m.with_stroke(c.as_str());
    }
    if let Some(w) = s.stroke_width {
        m = m.with_stroke_width(w);
    }
    m
}

fn style_rect(mut m: MarkRect, s: &Style) -> MarkRect {
    if let Some(c) = &s.color {
        m = m.with_color(c.as_str());
    }
    if let Some(o) = s.opacity {
        m = m.with_opacity(o);
    }
    if let Some(c) = &s.stroke {
        m = m.with_stroke(c.as_str());
    }
    if let Some(w) = s.stroke_width {
        m = m.with_stroke_width(w);
    }
    m
}

fn style_polygon(mut m: MarkGeoPath, s: &Style) -> MarkGeoPath {
    if let Some(c) = &s.color {
        m = m.with_fill(c.as_str());
    }
    if let Some(o) = s.opacity {
        m = m.with_opacity(o);
    }
    if let Some(c) = &s.stroke {
        m = m.with_stroke(c.as_str());
    }
    if let Some(w) = s.stroke_width {
        m = m.with_stroke_width(w);
    }
    m
}

fn style_boxplot(mut m: MarkBoxplot, s: &Style) -> MarkBoxplot {
    if let Some(c) = &s.color {
        m = m.with_color(c.as_str());
    }
    if let Some(o) = s.opacity {
        m = m.with_opacity(o);
    }
    if let Some(c) = &s.stroke {
        m = m.with_stroke(c.as_str());
    }
    if let Some(w) = s.stroke_width {
        m = m.with_stroke_width(w);
    }
    if let Some(c) = &s.outlier_color {
        m = m.with_outlier_color(c.as_str());
    }
    if let Some(w) = s.mark_width {
        m = m.with_width(w);
    }
    if let Some(show) = s.show_outliers {
        m = m.with_outliers(show);
    }
    if let Some(size) = s.outlier_size {
        m = m.with_outlier_size(size);
    }
    m
}

fn style_tick(mut m: MarkTick, s: &Style) -> MarkTick {
    if let Some(c) = &s.color {
        m = m.with_color(c.as_str());
    }
    if let Some(o) = s.opacity {
        m = m.with_opacity(o);
    }
    if let Some(c) = &s.stroke {
        m = m.with_stroke(c.as_str());
    }
    if let Some(v) = s.size {
        m = m.with_thickness(v);
    }
    m
}

fn style_text(mut m: MarkText, s: &Style) -> MarkText {
    if let Some(c) = &s.color {
        m = m.with_color(c.as_str());
    }
    if let Some(o) = s.opacity {
        m = m.with_opacity(o);
    }
    if let Some(v) = s.size {
        m = m.with_size(v);
    }
    if let Some(anchor) = &s.anchor {
        m = m.with_anchor(anchor.as_str());
    }
    if let Some(weight) = &s.weight {
        m = m.with_weight(weight.as_str());
    }
    m
}

fn style_rule(mut m: MarkRule, s: &Style) -> MarkRule {
    if let Some(c) = &s.color {
        m = m.with_color(c.as_str());
    }
    if let Some(o) = s.opacity {
        m = m.with_opacity(o);
    }
    if let Some(w) = s.stroke_width {
        m = m.with_stroke_width(w);
    }
    m
}

fn style_errorbar(mut m: MarkErrorBar, s: &Style) -> MarkErrorBar {
    if let Some(c) = &s.color {
        m = m.with_color(c.as_str());
    }
    if let Some(o) = s.opacity {
        m = m.with_opacity(o);
    }
    if let Some(w) = s.stroke_width {
        m = m.with_stroke_width(w);
    }
    if let Some(w) = s.mark_width {
        m = m.with_width(w);
    }
    if let Some(l) = s.cap_length {
        m = m.with_cap_length(l);
    }
    if let Some(show) = s.center {
        m = m.with_center(show);
    }
    m
}

fn chart_err(span: Span, e: ChartonError) -> LabeledError {
    LabeledError::new("Chart build error").with_label(e.to_string(), span)
}

fn missing(geom: &str, what: &str, span: Span) -> LabeledError {
    LabeledError::new("Missing required option")
        .with_label(format!("`charton -g {geom}` requires {what}"), span)
}

/// Combine a min/max flag pair, requiring both or neither.
fn domain_pair(
    min: Option<f64>,
    max: Option<f64>,
    min_flag: &str,
    max_flag: &str,
    span: Span,
) -> Result<Option<(f64, f64)>, LabeledError> {
    match (min, max) {
        (Some(a), Some(b)) => Ok(Some((a, b))),
        (None, None) => Ok(None),
        _ => Err(LabeledError::new("Incomplete domain").with_label(
            format!("{min_flag} and {max_flag} must be given together"),
            span,
        )),
    }
}

fn build_chart(values: &[Value], opts: &BuildOpts<'_>) -> Result<LayeredChart, LabeledError> {
    let span = opts.span;

    // --- Geographic marks read from a GeoJSON file, not the pipeline. ---
    if matches!(opts.primary.geom.as_str(), "geo" | "geoshape") {
        if !opts.layers.is_empty() {
            return Err(LabeledError::new("Layers are not supported for geo charts")
                .with_label("remove `--layer` when using `-g geo`", span));
        }
        let path = opts
            .geojson
            .ok_or_else(|| missing("geo", "--geojson <path>", span))?;
        let text = std::fs::read_to_string(path).map_err(|e| {
            LabeledError::new("Failed to read GeoJSON")
                .with_label(format!("{}: {e}", path.display()), span)
        })?;
        let ds = geojson_to_dataset(&text).map_err(|e| chart_err(span, e))?;
        let c = Chart::build(ds)
            .map_err(|e| chart_err(span, e))?
            .mark_geoshape()
            .map_err(|e| chart_err(span, e))?;
        let c = match opts.primary.color.as_deref() {
            Some(col) => c.encode((
                alt::x("_lon"),
                alt::y("_lat"),
                alt::path_group("_path_id"),
                alt::color(col),
            )),
            None => c.encode((alt::x("_lon"), alt::y("_lat"), alt::path_group("_path_id"))),
        }
        .map_err(|e| chart_err(span, e))?;
        let layered: LayeredChart = c.into();
        return Ok(finish(
            layered.with_coord(CoordSystem::Geo).with_grid(true),
            opts,
        ));
    }

    // --- Tabular marks read from the pipeline table; layers share the dataset. ---
    let table = Table::from_values(values, span)?;
    let dataset = table.to_dataset()?;

    let mut layered = build_layer(dataset.clone(), &opts.primary, opts.coord, span)?;
    for extra in &opts.layers {
        layered = layered.and(build_layer(dataset.clone(), extra, opts.coord, span)?);
    }
    Ok(finish(layered, opts))
}

/// Build a single chart layer. Layers are combined with `.and()` so they share
/// one scale and coordinate system.
fn build_layer(
    dataset: Dataset,
    layer: &LayerOpts,
    coord: Option<CoordSystem>,
    span: Span,
) -> Result<LayeredChart, LabeledError> {
    let geom = layer.geom.as_str();
    let (x, y, y2) = (layer.x.as_deref(), layer.y.as_deref(), layer.y2.as_deref());
    let z = layer.z.as_deref();
    let color = layer.color.as_deref();
    let style = &layer.style;
    let (x_scale, y_scale) = (layer.x_scale, layer.y_scale);
    let stack = layer.stack.as_deref();
    let normalize = layer.normalize;
    let (size_by, shape_by) = (layer.size_by.as_deref(), layer.shape_by.as_deref());
    let aggregate = layer.aggregate.as_deref();
    let bins = layer.bins;
    let (x_expand, y_expand) = (layer.x_expand, layer.y_expand);

    // Axis padding from `--x-expand`/`--y-expand`, as a symmetric fraction.
    let pad = |v: f64| Expansion {
        mult: (v, v),
        add: (0.0, 0.0),
    };

    // Build an x or y encoding from this layer's settings. The x encoder carries
    // the axis scale, binning and padding; the y encoder additionally carries
    // stacking, normalization and aggregation.
    let make_x = |field: &str| {
        let mut e = match x_scale {
            Some(s) => alt::x(field).with_scale(s),
            None => alt::x(field),
        };
        if let Some(n) = bins {
            e = e.with_bins(n);
        }
        if let Some(v) = x_expand {
            e = e.with_expansion(pad(v));
        }
        e
    };
    let make_y = |field: &str| {
        let mut e = match y_scale {
            Some(s) => alt::y(field).with_scale(s),
            None => alt::y(field),
        };
        if let Some(mode) = stack {
            e = e.with_stack(mode);
        }
        if normalize {
            e = e.with_normalize(true);
        }
        if let Some(op) = aggregate {
            e = e.with_aggregate(op);
        }
        if let Some(v) = y_expand {
            e = e.with_expansion(pad(v));
        }
        e
    };

    // Encode x and y (both required) together with whichever of the
    // color/size/shape channels this layer uses. Styling is applied to the mark
    // before encoding, so mark-dependent transforms see the final settings.
    macro_rules! enc_xy_color {
        ($chart:expr) => {{
            let (x, y) = match (x, y) {
                (Some(x), Some(y)) => (x, y),
                _ => return Err(missing(geom, "both --x and --y", span)),
            };
            let xe = make_x(x);
            let ye = make_y(y);
            match (color, size_by, shape_by) {
                (None, None, None) => $chart.encode((xe, ye)),
                (Some(c), None, None) => $chart.encode((xe, ye, alt::color(c))),
                (None, Some(sz), None) => $chart.encode((xe, ye, alt::size(sz))),
                (None, None, Some(sh)) => $chart.encode((xe, ye, alt::shape(sh))),
                (Some(c), Some(sz), None) => $chart.encode((xe, ye, alt::color(c), alt::size(sz))),
                (Some(c), None, Some(sh)) => $chart.encode((xe, ye, alt::color(c), alt::shape(sh))),
                (None, Some(sz), Some(sh)) => {
                    $chart.encode((xe, ye, alt::size(sz), alt::shape(sh)))
                }
                (Some(c), Some(sz), Some(sh)) => {
                    $chart.encode((xe, ye, alt::color(c), alt::size(sz), alt::shape(sh)))
                }
            }
            .map_err(|e| chart_err(span, e))?
        }};
    }

    let layered: LayeredChart = match geom {
        "point" | "scatter" => {
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_point()
                .map_err(|e| chart_err(span, e))?
                .configure_point(|m| style_point(m, style));
            enc_xy_color!(c).into()
        }
        "beeswarm" | "swarm" => {
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_point()
                .map_err(|e| chart_err(span, e))?
                .configure_point(|m| style_point(m.with_layout("beeswarm").with_size(2.0), style));
            enc_xy_color!(c).into()
        }
        "line" => {
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_line()
                .map_err(|e| chart_err(span, e))?
                .configure_line(|m| style_line(m, style));
            enc_xy_color!(c).into()
        }
        "area" => {
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_area()
                .map_err(|e| chart_err(span, e))?
                .configure_area(|m| style_area(m, style));
            enc_xy_color!(c).into()
        }
        "bar" => {
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_bar()
                .map_err(|e| chart_err(span, e))?
                .configure_bar(|m| style_bar(m, style));
            // Polar bar charts have two forms, matching charton's semantics:
            //   * with `-x`, x is the angle and y the radius — a rose / Nightingale
            //     chart;
            //   * without `-x`, charton uses pie mode: y becomes the slices
            //     (aggregated per colour) and the bar width is the radial extent —
            //     a pie, or a donut with `--inner-radius`.
            let pie = coord == Some(CoordSystem::Polar) && x.map(str::is_empty).unwrap_or(true);
            if pie {
                let y = y.ok_or_else(|| missing(geom, "--y", span))?;
                let (xe, ye) = (alt::x(""), make_y(y));
                let c = match color {
                    Some(col) => c.encode((xe, ye, alt::color(col))),
                    None => c.encode((xe, ye)),
                }
                .map_err(|e| chart_err(span, e))?;
                c.into()
            } else {
                enc_xy_color!(c).into()
            }
        }
        "boxplot" | "box" => {
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_boxplot()
                .map_err(|e| chart_err(span, e))?
                .configure_boxplot(|m| style_boxplot(m, style));
            enc_xy_color!(c).into()
        }
        "violin" | "violinplot" => {
            let x = x.ok_or_else(|| missing(geom, "--x (the category column)", span))?;
            let y = y.ok_or_else(|| missing(geom, "--y (the value column)", span))?;

            // A violin is a composition, not a mark: a density outline plus an
            // inner inter-quartile box. The outline is the general recipe —
            // `transform_density` (grouped by category, optionally also by the
            // colour group) followed by the general `transform_band` geometry.
            // `--color` dodges one violin per group; without it there is one
            // violin per category.
            // The composition generates intermediate columns. They are
            // namespaced so they can never collide with the user's own columns,
            // which are passed straight through as transform inputs — a
            // category column is very often literally called `x`.
            const VALUE: &str = "__charton_violin_value";
            const WIDTH: &str = "__charton_violin_width";
            const BAND_X: &str = "__charton_violin_x";
            const BAND_Y: &str = "__charton_violin_y";
            const BAND_PATH: &str = "__charton_violin_path";
            const BOX_X: &str = "__charton_violin_box_x";
            const BOX_Y: &str = "__charton_violin_box_y";
            const BOX_PATH: &str = "__charton_violin_box_path";

            let mut density = DensityTransform::new(y)
                .with_as(VALUE, WIDTH)
                .with_groupbys([x])
                // A violin ends at the data (ggplot2 `trim = TRUE`), unlike a
                // density plot, which keeps its smooth tails.
                .with_trim(true);
            let mut band = BandTransform::new(VALUE, WIDTH)
                .with_center(x)
                .with_as(BAND_X, BAND_Y, BAND_PATH)
                .with_scale(BandScale::PerGroup);
            let mut quantile_box = QuantileBoxTransform::new(y)
                .with_category(x)
                .with_as(BOX_X, BOX_Y, BOX_PATH);
            if let Some(group) = color {
                density = density.with_groupbys([x, group]);
                band = band.with_group(group).with_position(Position::dodge());
                quantile_box = quantile_box
                    .with_group(group)
                    .with_position(Position::dodge());
            }

            let outline = Chart::build(dataset.clone())
                .map_err(|e| chart_err(span, e))?
                .transform_density(density)
                .map_err(|e| chart_err(span, e))?
                .transform_band(band)
                .map_err(|e| chart_err(span, e))?
                .mark_polygon()
                .map_err(|e| chart_err(span, e))?
                .configure_geoshape(|m| style_polygon(m, style));
            let outline = match color {
                Some(c) => outline.encode((
                    alt::x(BAND_X).with_category_labels(x),
                    make_y(BAND_Y),
                    alt::path_group(BAND_PATH),
                    alt::color(c),
                )),
                None => outline.encode((
                    alt::x(BAND_X).with_category_labels(x),
                    make_y(BAND_Y),
                    alt::path_group(BAND_PATH),
                )),
            }
            .map_err(|e| chart_err(span, e))?;

            let inner_box = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .transform_quantile_box(quantile_box)
                .map_err(|e| chart_err(span, e))?
                .mark_polygon()
                .map_err(|e| chart_err(span, e))?
                .configure_geoshape(|m| {
                    m.with_fill("white")
                        .with_stroke("black")
                        .with_stroke_width(1.0)
                });
            let inner_box = inner_box
                .encode((
                    alt::x(BOX_X).with_category_labels(x),
                    make_y(BOX_Y),
                    alt::path_group(BOX_PATH),
                ))
                .map_err(|e| chart_err(span, e))?;

            // The generated columns are namespaced, so default the axis titles
            // to the user's own column names. Explicit `--x-label` / `--y-label`
            // still win; they are applied later.
            outline.and(inner_box).with_x_label(x).with_y_label(y)
        }
        "tick" => {
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_tick()
                .map_err(|e| chart_err(span, e))?
                .configure_tick(|m| style_tick(m, style));
            enc_xy_color!(c).into()
        }
        "text" | "label" => {
            let x = x.ok_or_else(|| missing(geom, "--x", span))?;
            let y = y.ok_or_else(|| missing(geom, "--y", span))?;
            let label = layer
                .text
                .as_deref()
                .ok_or_else(|| missing(geom, "--text <column>", span))?;
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_text()
                .map_err(|e| chart_err(span, e))?
                .configure_text(|m| style_text(m, style));
            let c = match color {
                Some(col) => c.encode((alt::x(x), alt::y(y), alt::text(label), alt::color(col))),
                None => c.encode((alt::x(x), alt::y(y), alt::text(label))),
            }
            .map_err(|e| chart_err(span, e))?;
            c.into()
        }
        "rect" | "heatmap" => {
            let x = x.ok_or_else(|| missing(geom, "--x", span))?;
            let y = y.ok_or_else(|| missing(geom, "--y", span))?;
            let col = color.ok_or_else(|| missing(geom, "--color <value column>", span))?;
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_rect()
                .map_err(|e| chart_err(span, e))?
                .configure_rect(|m| style_rect(m, style));
            c.encode((alt::x(x), alt::y(y), alt::color(col)))
                .map_err(|e| chart_err(span, e))?
                .into()
        }
        "rule" => {
            let x = x.ok_or_else(|| missing(geom, "--x", span))?;
            let y = y.ok_or_else(|| missing(geom, "--y", span))?;
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_rule()
                .map_err(|e| chart_err(span, e))?
                .configure_rule(|m| style_rule(m, style));
            let c = match (y2, color) {
                (Some(y2), Some(col)) => {
                    c.encode((alt::x(x), alt::y(y), alt::y2(y2), alt::color(col)))
                }
                (Some(y2), None) => c.encode((alt::x(x), alt::y(y), alt::y2(y2))),
                (None, Some(col)) => c.encode((alt::x(x), alt::y(y), alt::color(col))),
                (None, None) => c.encode((alt::x(x), alt::y(y))),
            }
            .map_err(|e| chart_err(span, e))?;
            c.into()
        }
        "errorbar" => {
            let x = x.ok_or_else(|| missing(geom, "--x", span))?;
            let y = y.ok_or_else(|| missing(geom, "--y", span))?;
            let xe = make_x(x);
            let ye = make_y(y);
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_errorbar()
                .map_err(|e| chart_err(span, e))?
                .configure_errorbar(|m| style_errorbar(m, style));
            // With --y2: explicit min/max columns. Without: charton aggregates
            // the raw y values per group into mean +/- std.
            let c = match (y2, color) {
                (Some(y2), Some(col)) => c.encode((xe, ye, alt::y2(y2), alt::color(col))),
                (Some(y2), None) => c.encode((xe, ye, alt::y2(y2))),
                (None, Some(col)) => c.encode((xe, ye, alt::color(col))),
                (None, None) => c.encode((xe, ye)),
            }
            .map_err(|e| chart_err(span, e))?;
            c.into()
        }
        "density" | "kde" => {
            let x = x.ok_or_else(|| missing(geom, "--x", span))?;
            let mut dt = DensityTransform::new(x)
                .with_as(x, "density")
                .with_cumulative(layer.density_cumulative)
                .with_counts(layer.density_counts);
            if let Some(group) = color {
                dt = dt.with_groupbys([group]);
            }
            if let Some(kernel) = layer.density_kernel {
                dt = dt.with_kernel(kernel);
            }
            if let Some(bw) = layer.density_bandwidth {
                dt = dt.with_bandwidth(BandwidthType::Fixed(bw));
            }
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_area()
                .map_err(|e| chart_err(span, e))?
                .configure_area(|m| style_area(m, style))
                .transform_density(dt)
                .map_err(|e| chart_err(span, e))?;
            let xe = make_x(x);
            let ye = make_y("density");
            match color {
                Some(col) => c.encode((xe, ye, alt::color(col))),
                None => c.encode((xe, ye)),
            }
            .map_err(|e| chart_err(span, e))?
            .into()
        }
        "ecdf" => {
            let x = x.ok_or_else(|| missing(geom, "--x", span))?;
            let field = WindowFieldDef::new(x, WindowOnlyOp::CumeDist, "ecdf");
            let mut wt = WindowTransform::new(field);
            if let Some(group) = color {
                wt = wt.with_groupbys([group]);
            }
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_line()
                .map_err(|e| chart_err(span, e))?
                .configure_line(|m| style_line(m.with_interpolation("step"), style))
                .transform_window(wt)
                .map_err(|e| chart_err(span, e))?;
            let xe = make_x(x);
            let ye = make_y("ecdf");
            match color {
                Some(col) => c.encode((xe, ye, alt::color(col))),
                None => c.encode((xe, ye)),
            }
            .map_err(|e| chart_err(span, e))?
            .into()
        }
        "hist" | "histogram" => {
            let x = x.ok_or_else(|| missing(geom, "--x", span))?;
            let xe = make_x(x);
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_hist()
                .map_err(|e| chart_err(span, e))?;
            // charton computes the bin counts into the y field named here.
            c.encode((xe, make_y("count")))
                .map_err(|e| chart_err(span, e))?
                .into()
        }
        "contour" | "isoline" => {
            let x = x.ok_or_else(|| missing(geom, "--x", span))?;
            let y = y.ok_or_else(|| missing(geom, "--y", span))?;
            let levels = bins.unwrap_or(10);

            // Two forms, both ending in the same iso-line table:
            //   * with `--z`: contours of that scalar grid;
            //   * without `--z`: a 2D density is estimated from the x/y points
            //     first, so the classic scatter → density contour works.
            let contours = match z {
                Some(z) => Chart::build(dataset)
                    .map_err(|e| chart_err(span, e))?
                    .transform_contour(ContourTransform::new(x, y, z).with_levels(levels))
                    .map_err(|e| chart_err(span, e))?,
                None => Chart::build(dataset)
                    .map_err(|e| chart_err(span, e))?
                    .transform_density_2d(Density2DTransform::new(x, y))
                    .map_err(|e| chart_err(span, e))?
                    .transform_contour(
                        ContourTransform::new("x", "y", "density").with_levels(levels),
                    )
                    .map_err(|e| chart_err(span, e))?,
            };

            let c = contours
                .mark_path()
                .map_err(|e| chart_err(span, e))?
                .configure_path(|m| style_polygon(m, style));

            // By default the lines are coloured by their level. A single-colour
            // contour is requested with `--stroke`, which drops the colour
            // channel so the mark's own stroke shows through.
            if style.stroke.is_some() {
                c.encode((make_x("x"), make_y("y"), alt::path_group("path_group")))
                    .map_err(|e| chart_err(span, e))?
                    .into()
            } else {
                c.encode((
                    make_x("x"),
                    make_y("y"),
                    alt::path_group("path_group"),
                    alt::color("level"),
                ))
                .map_err(|e| chart_err(span, e))?
                .into()
            }
        }
        other => {
            return Err(LabeledError::new("Unknown geom").with_label(
                format!(
                    "'{other}' is not supported; try point, line, area, bar, boxplot, \
                     violin, errorbar, rule, tick, text, rect, hist, density, ecdf, \
                     contour, beeswarm, or geo"
                ),
                span,
            ));
        }
    };

    Ok(layered)
}

/// Parse the `--layer` flag into extra layers. Each record needs `geom`; `x`,
/// `y`, `y2`, `color`, and `text` default to the primary layer when omitted.
fn parse_layers(
    value: Option<Value>,
    primary: &LayerOpts,
    span: Span,
) -> Result<Vec<LayerOpts>, LabeledError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let records = match value {
        Value::List { vals, .. } => vals.into_owned(),
        other => vec![other],
    };

    let mut out = Vec::with_capacity(records.len());
    for (i, record) in records.into_iter().enumerate() {
        let layer_span = record.span();
        let Value::Record { val, .. } = &record else {
            return Err(LabeledError::new("Invalid layer")
                .with_label(format!("layer {i} must be a record"), layer_span));
        };
        let field = |k: &str| val.get(k).and_then(|v| v.as_str().ok().map(str::to_string));
        let geom = field("geom").or_else(|| field("mark")).ok_or_else(|| {
            LabeledError::new("Invalid layer")
                .with_label(format!("layer {i} is missing `geom`"), span)
        })?;
        // A layer may carry its own style record and axis scales; otherwise it
        // inherits the primary layer's.
        let style = match val.get("style") {
            Some(Value::Record { val: style_rec, .. }) => {
                parse_style_record(style_rec, &primary.style, layer_span)?
            }
            _ => primary.style.clone(),
        };
        let scale_field = |k: &str, legacy: &str| -> Result<Option<Scale>, LabeledError> {
            match field(k).or_else(|| field(legacy)) {
                Some(s) => Ok(Some(parse_scale(&s, layer_span)?)),
                None => Ok(None),
            }
        };
        let x_scale = scale_field("x_scale", "x-scale")?.or(primary.x_scale);
        let y_scale = scale_field("y_scale", "y-scale")?.or(primary.y_scale);
        let stack = match field("stack") {
            Some(s) => Some(parse_stack(&s, layer_span)?),
            None => primary.stack.clone(),
        };
        let normalize = field("normalize")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(primary.normalize);
        out.push(LayerOpts {
            geom: geom.to_lowercase(),
            x: field("x").or_else(|| primary.x.clone()),
            y: field("y").or_else(|| primary.y.clone()),
            z: field("z").or_else(|| primary.z.clone()),
            y2: field("y2").or_else(|| primary.y2.clone()),
            color: field("color").or_else(|| primary.color.clone()),
            text: field("text").or_else(|| primary.text.clone()),
            style,
            x_scale,
            y_scale,
            stack,
            normalize,
            size_by: field("size_by")
                .or_else(|| field("size-by"))
                .or_else(|| primary.size_by.clone()),
            shape_by: field("shape_by")
                .or_else(|| field("shape-by"))
                .or_else(|| primary.shape_by.clone()),
            aggregate: match field("aggregate") {
                Some(s) => Some(parse_aggregate(&s, layer_span)?),
                None => primary.aggregate.clone(),
            },
            bins: field("bins").and_then(|s| s.parse().ok()).or(primary.bins),
            x_expand: primary.x_expand,
            y_expand: primary.y_expand,
            density_bandwidth: primary.density_bandwidth,
            density_kernel: primary.density_kernel,
            density_cumulative: primary.density_cumulative,
            density_counts: primary.density_counts,
        });
    }
    Ok(out)
}

/// Parse `--x-scale` / `--y-scale`: `linear`, `log`, `discrete`, or `temporal`.
fn parse_scale(spec: &str, span: Span) -> Result<Scale, LabeledError> {
    Ok(match spec.trim().to_ascii_lowercase().as_str() {
        "linear" | "lin" => Scale::Linear,
        "log" | "log10" | "logarithmic" => Scale::Log,
        "discrete" | "category" | "categorical" => Scale::Discrete,
        "temporal" | "time" | "date" | "datetime" => Scale::Temporal,
        other => {
            return Err(LabeledError::new("Invalid axis scale").with_label(
                format!("unknown scale '{other}'; expected linear, log, discrete, or temporal"),
                span,
            ));
        }
    })
}

/// Parse `--coord`: `cartesian` (the default) or `polar`. Geographic charts are
/// selected with `-g geo`, not with this flag.
fn parse_coord(spec: Option<String>, span: Span) -> Result<Option<CoordSystem>, LabeledError> {
    let Some(spec) = spec else {
        return Ok(None);
    };
    Ok(match spec.trim().to_ascii_lowercase().as_str() {
        "cartesian" | "cartesian2d" | "xy" => None,
        "polar" => Some(CoordSystem::Polar),
        other => {
            return Err(LabeledError::new("Invalid coordinate system").with_label(
                format!("unknown coord '{other}'; expected cartesian or polar"),
                span,
            ));
        }
    })
}

/// Parse `--stack`: `none`, `stacked`, `normalize`, or `center`.
fn parse_stack(spec: &str, span: Span) -> Result<String, LabeledError> {
    Ok(match spec.trim().to_ascii_lowercase().as_str() {
        "none" | "grouped" | "identity" => "none",
        "stacked" | "stack" => "stacked",
        "normalize" | "normalized" | "percent" => "normalize",
        "center" | "stream" | "streamgraph" => "center",
        other => {
            return Err(LabeledError::new("Invalid stack mode").with_label(
                format!("unknown stack '{other}'; expected none, stacked, normalize, or center"),
                span,
            ));
        }
    }
    .to_string())
}

/// Parse `--aggregate`: `sum`, `mean`, `median`, `min`, `max`, or `count`.
fn parse_aggregate(spec: &str, span: Span) -> Result<String, LabeledError> {
    let key = spec.trim().to_ascii_lowercase();
    match key.as_str() {
        "sum" | "mean" | "avg" | "median" | "min" | "max" | "count" | "n" => Ok(key),
        other => Err(LabeledError::new("Invalid aggregate").with_label(
            format!("unknown aggregate '{other}'; expected sum, mean, median, min, max, or count"),
            span,
        )),
    }
}

/// Parse `--density-kernel`: `normal`, `epanechnikov`, or `uniform`.
fn parse_kernel(spec: &str, span: Span) -> Result<KernelType, LabeledError> {
    Ok(match spec.trim().to_ascii_lowercase().as_str() {
        "normal" | "gaussian" | "gauss" => KernelType::Normal,
        "epanechnikov" | "epan" => KernelType::Epanechnikov,
        "uniform" | "box" => KernelType::Uniform,
        other => {
            return Err(LabeledError::new("Invalid density kernel").with_label(
                format!("unknown kernel '{other}'; expected normal, epanechnikov, or uniform"),
                span,
            ));
        }
    })
}

/// Parse `--margins` as four fractions: `top,right,bottom,left`.
fn parse_margins(spec: &str, span: Span) -> Result<[f64; 4], LabeledError> {
    let parts: Result<Vec<f64>, _> = spec
        .split([',', ' '])
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.trim().parse::<f64>())
        .collect();
    let parts = parts.map_err(|_| {
        LabeledError::new("Invalid margins").with_label(
            format!("'{spec}' is not four numbers; try '0.05,0.03,0.08,0.06'"),
            span,
        )
    })?;
    parts.try_into().map_err(|_| {
        LabeledError::new("Invalid margins").with_label(
            format!("'{spec}' must be exactly top,right,bottom,left"),
            span,
        )
    })
}

/// Parse `--x-ticks` / `--y-ticks` from a list of numbers (or a single number).
fn parse_ticks(value: Option<Value>, span: Span) -> Result<Option<Vec<f64>>, LabeledError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let values = match value {
        Value::List { vals, .. } => vals.into_owned(),
        other => vec![other],
    };
    let mut ticks = Vec::with_capacity(values.len());
    for v in values {
        let n = v
            .as_float()
            .ok()
            .or_else(|| v.as_int().ok().map(|i| i as f64));
        match n {
            Some(n) => ticks.push(n),
            None => {
                return Err(LabeledError::new("Invalid ticks").with_label(
                    format!("tick value must be a number, got {}", v.get_type()),
                    span,
                ));
            }
        }
    }
    Ok(Some(ticks))
}

/// Parse `--dash`, e.g. `"6,4"` or `"2 2"`, into an on/off pattern in pixels.
fn parse_dash(spec: &str, span: Span) -> Result<Vec<f64>, LabeledError> {
    let parts: Result<Vec<f64>, _> = spec
        .split([',', ' '])
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.trim().parse::<f64>())
        .collect();
    let parts = parts.map_err(|_| {
        LabeledError::new("Invalid dash pattern").with_label(
            format!("'{spec}' is not a list of numbers; try '6,4'"),
            span,
        )
    })?;
    if parts.is_empty() || parts.iter().any(|v| !v.is_finite() || *v < 0.0) {
        return Err(LabeledError::new("Invalid dash pattern").with_label(
            format!("'{spec}' must contain non-negative numbers; try '6,4'"),
            span,
        ));
    }
    Ok(parts)
}

/// Parse `--x-format` / `--y-format` / `--legend-format`.
///
/// Accepts the preset string `compact` (or `plain`), or a record such as
/// `{prefix: "$", compact: true, precision: 0}`.
fn parse_label_format(value: &Value, span: Span) -> Result<LabelFormat, LabeledError> {
    let bad = |msg: String| LabeledError::new("Invalid label format").with_label(msg, span);
    match value {
        Value::String { val, .. } => match val.trim().to_ascii_lowercase().as_str() {
            "compact" | "short" => Ok(LabelFormat::new().with_compact_notation()),
            "plain" | "none" | "default" => Ok(LabelFormat::new()),
            other => Err(bad(format!(
                "unknown format preset '{other}'; use 'compact' or a record like \
                 {{compact: true, prefix: \"$\"}}"
            ))),
        },
        Value::Record { val, .. } => {
            let mut format = LabelFormat::new();
            let strf = |k: &str| val.get(k).and_then(|v| v.as_str().ok());
            let numf = |k: &str| {
                val.get(k).and_then(|v| {
                    v.as_float()
                        .ok()
                        .or_else(|| v.as_int().ok().map(|i| i as f64))
                })
            };
            if let Some(p) = strf("prefix") {
                format = format.with_prefix(p);
            }
            if let Some(suf) = strf("suffix") {
                format = format.with_suffix(suf);
            }
            if let Some(p) = numf("precision") {
                format = format.with_precision(p.max(0.0) as usize);
            }
            if val
                .get("compact")
                .and_then(|v| v.as_bool().ok())
                .unwrap_or(false)
            {
                format = format.with_compact_notation();
            }
            if let Some(sep) = strf("thousands").and_then(|s| s.chars().next()) {
                format = format.with_thousands_separator(sep);
            }
            if let Some(m) = numf("multiplier") {
                format = format.with_multiplier(m);
            }
            Ok(format)
        }
        other => Err(bad(format!(
            "expected a string preset or a record, got {}",
            other.get_type()
        ))),
    }
}

/// Build a per-layer [`Style`] by overriding `base` with the fields present in
/// a `--layer` `style` record.
fn parse_style_record(rec: &Record, base: &Style, span: Span) -> Result<Style, LabeledError> {
    let mut s = base.clone();
    let strf = |k: &str| rec.get(k).and_then(|v| v.as_str().ok().map(str::to_string));
    let numf = |k: &str| {
        rec.get(k).and_then(|v| {
            v.as_float()
                .ok()
                .or_else(|| v.as_int().ok().map(|i| i as f64))
        })
    };
    if let Some(v) = strf("color") {
        s.color = Some(v);
    }
    if let Some(v) = numf("opacity") {
        s.opacity = Some(v);
    }
    if let Some(v) = numf("size") {
        s.size = Some(v);
    }
    if let Some(v) = strf("stroke") {
        s.stroke = Some(v);
    }
    if let Some(v) = numf("stroke_width").or_else(|| numf("stroke-width")) {
        s.stroke_width = Some(v);
    }
    if let Some(v) = strf("shape") {
        s.shape = Some(v);
    }
    if let Some(v) = strf("dash") {
        s.dash = Some(parse_dash(&v, span)?);
    }
    if let Some(v) = strf("interpolation") {
        s.interpolation = Some(v);
    }
    if let Some(v) = strf("outlier_color").or_else(|| strf("outlier-color")) {
        s.outlier_color = Some(v);
    }
    if let Some(v) = strf("anchor") {
        s.anchor = Some(v);
    }
    if let Some(v) = strf("weight") {
        s.weight = Some(v);
    }
    if let Some(v) = strf("layout") {
        s.layout = Some(v);
    }
    if let Some(v) = strf("quasirandom_method").or_else(|| strf("quasirandom-method")) {
        s.quasirandom_method = Some(v);
    }
    Ok(s)
}

/// Resolve the `--theme` flag (or plugin config) into a [`ThemeMode`].
///
/// `auto` inspects the terminal's background when the shell exposes it
/// (`COLORFGBG`) and otherwise falls back to [`ThemeMode::Light`], so behavior
/// stays unchanged on terminals we cannot detect.
fn resolve_theme(setting: Option<String>, span: Span) -> Result<ThemeMode, LabeledError> {
    let value = setting.unwrap_or_else(|| "auto".to_string());
    if value.eq_ignore_ascii_case("auto") {
        return Ok(detect_theme_mode());
    }
    value
        .parse::<ThemeMode>()
        .map_err(|e| LabeledError::new("Invalid theme").with_label(e, span))
}

/// Best-effort detection of the terminal background from `COLORFGBG`.
///
/// The variable is commonly formatted as `"fg;bg"` (for example `"15;0"`),
/// where the last field is the background color index. Indices 0-6 and 8 are
/// dark; 7 and 9-15 are light. Absent or malformed values keep the light
/// default.
fn detect_theme_mode() -> ThemeMode {
    std::env::var("COLORFGBG")
        .ok()
        .as_deref()
        .and_then(parse_colorfgbg)
        .unwrap_or(ThemeMode::Light)
}

/// Parse a `COLORFGBG` value into a mode, if it carries a usable background
/// index.
fn parse_colorfgbg(value: &str) -> Option<ThemeMode> {
    let bg = value.rsplit(';').next()?;
    match bg.trim().parse::<u8>().ok()? {
        7 | 9..=15 => Some(ThemeMode::Light),
        _ => Some(ThemeMode::Dark),
    }
}

fn finish(layered: LayeredChart, opts: &BuildOpts<'_>) -> LayeredChart {
    let mut l = layered
        .with_size(opts.width, opts.height)
        .with_scale_factor(opts.scale);
    if let Some(title) = opts.title {
        l = l.with_title(title);
    }
    if let Some(x) = opts.x_label {
        l = l.with_x_label(x);
    }
    if let Some(y) = opts.y_label {
        l = l.with_y_label(y);
    }
    if let Some(c) = opts.color_label {
        l = l.with_color_label(c);
    }
    if let Some(s) = opts.size_label {
        l = l.with_size_label(s);
    }
    if let Some(s) = opts.shape_label {
        l = l.with_shape_label(s);
    }
    if let Some((min, max)) = opts.x_domain {
        l = l.with_x_domain(min, max);
    }
    if let Some((min, max)) = opts.y_domain {
        l = l.with_y_domain(min, max);
    }
    // Canvas margins come from the theme; `--margins` overrides them.
    if let Some([top, right, bottom, left]) = opts.margins {
        l = l.with_margins(top, right, bottom, left);
    }
    if let Some(ticks) = opts.x_ticks.clone() {
        l = l.with_x_ticks(ticks);
    }
    if let Some(ticks) = opts.y_ticks.clone() {
        l = l.with_y_ticks(ticks);
    }
    if opts.flip {
        l = l.coord_flip();
    }
    if let Some(coord) = opts.coord {
        l = l.with_coord(coord);
    }
    if let Some(r) = opts.inner_radius {
        l = l.with_inner_radius(r);
    }
    if let Some(a) = opts.start_angle {
        l = l.with_start_angle(a.to_radians());
    }
    if let Some(a) = opts.end_angle {
        l = l.with_end_angle(a.to_radians());
    }
    if let Some(facet) = &opts.facet {
        l = l.facet(facet.clone());
    }
    if let Some(grid) = opts.grid {
        l = l.with_grid(grid);
    }
    // Start from the selected light/dark preset, then layer user overrides on
    // top. An explicit `--background` always wins over the theme's background.
    l = l.configure_theme(|mut t| {
        t = t.with_mode(opts.theme);
        if let Some(p) = &opts.palette {
            t = t.with_palette(p.clone());
        }
        if let Some(legend) = opts.legend {
            match legend {
                LegendSetting::Off => t = t.with_show_legend(false),
                LegendSetting::Position(pos) => t = t.with_legend_position(pos),
            }
        }
        if let Some(angle) = opts.x_angle {
            t = t.with_x_tick_label_angle(angle);
        }
        if let Some(map) = opts.color_map {
            t = t.with_color_map(map);
        }
        if let Some(bg) = &opts.background {
            t = t.with_background_color(bg.as_str());
        }
        t
    });
    if let Some(format) = opts.x_format.clone() {
        l = l.with_x_label_format(format);
    }
    if let Some(format) = opts.y_format.clone() {
        l = l.with_y_label_format(format);
    }
    if let Some(format) = opts.legend_format.clone() {
        l = l.with_legend_label_format(format);
    }
    l
}

fn write_output(chart: &LayeredChart, path: &Path, span: Span) -> Result<(), LabeledError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            LabeledError::new("Failed to create output directory").with_label(e.to_string(), span)
        })?;
    }
    let write_err = |e: std::io::Error| {
        LabeledError::new("Failed to write output").with_label(e.to_string(), span)
    };

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext.eq_ignore_ascii_case("png") {
        let bytes = render::to_png(chart)
            .map_err(|e| LabeledError::new("PNG rendering failed").with_label(e, span))?;
        std::fs::write(path, bytes).map_err(write_err)?;
    } else if ext.is_empty() || ext.eq_ignore_ascii_case("svg") {
        let svg = render::to_svg(chart)
            .map_err(|e| LabeledError::new("SVG rendering failed").with_label(e, span))?;
        std::fs::write(path, svg).map_err(write_err)?;
    } else {
        return Err(LabeledError::new("Unsupported output format")
            .with_label(format!("'.{ext}' is not supported; use .svg or .png"), span));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::{IntoPipelineData, ShellError, Span, Value, record};

    fn table() -> Value {
        Value::test_list(vec![
            Value::test_record(record! {
                "species" => Value::test_string("setosa"),
                "petal_length" => Value::test_float(1.4),
                "t" => Value::test_int(0),
                "lo" => Value::test_float(1.0),
                "hi" => Value::test_float(1.8),
                "grp" => Value::test_string("x"),
            }),
            Value::test_record(record! {
                "species" => Value::test_string("setosa"),
                "petal_length" => Value::test_float(1.6),
                "t" => Value::test_int(1),
                "lo" => Value::test_float(1.2),
                "hi" => Value::test_float(2.0),
                "grp" => Value::test_string("y"),
            }),
            Value::test_record(record! {
                "species" => Value::test_string("virginica"),
                "petal_length" => Value::test_float(6.0),
                "t" => Value::test_int(2),
                "lo" => Value::test_float(5.5),
                "hi" => Value::test_float(6.5),
                "grp" => Value::test_string("x"),
            }),
        ])
    }

    fn run(src: &str) -> Result<Value, ShellError> {
        PluginTest::new("charton", crate::ChartonPlugin.into())?
            .eval_with(src, table().into_pipeline_data())?
            .into_value(Span::test_data())
    }

    /// A regular `x`/`y`/`z` grid, for the contour test.
    fn grid() -> Value {
        let mut rows = Vec::new();
        for i in 0..5 {
            for j in 0..5 {
                let x = i as f64;
                let y = j as f64;
                let z = ((x - 2.0).powi(2) + (y - 2.0).powi(2)).sqrt();
                rows.push(Value::test_record(record! {
                    "x" => Value::test_float(x),
                    "y" => Value::test_float(y),
                    "z" => Value::test_float(z),
                }));
            }
        }
        Value::test_list(rows)
    }

    fn run_on(table: Value, src: &str) -> Result<Value, ShellError> {
        PluginTest::new("charton", crate::ChartonPlugin.into())?
            .eval_with(src, table.into_pipeline_data())?
            .into_value(Span::test_data())
    }

    #[test]
    fn bar_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g bar -x species -y petal_length")?;
        assert!(out.as_str()?.trim_start().starts_with("<svg"));
        Ok(())
    }

    #[test]
    fn theme_dark_uses_dark_background() -> Result<(), ShellError> {
        let out = run("charton -g bar -x species -y petal_length --theme dark")?;
        // Catppuccin Mocha base #1E1E2E renders as rgba(30,30,46,1.000).
        assert!(out.as_str()?.contains("rgba(30,30,46,1.000)"));
        Ok(())
    }

    #[test]
    fn theme_light_uses_white_background() -> Result<(), ShellError> {
        let out = run("charton -g bar -x species -y petal_length --theme light")?;
        assert!(out.as_str()?.contains("rgba(255,255,255,1.000)"));
        Ok(())
    }

    #[test]
    fn unknown_theme_is_an_error() {
        assert!(run("charton -g bar -x species -y petal_length --theme blue").is_err());
    }

    #[test]
    fn hist_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g hist -x petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn beeswarm_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g beeswarm -x species -y petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn area_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g area -x t -y petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn boxplot_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g boxplot -x species -y petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn contour_returns_svg() -> Result<(), ShellError> {
        let out = run_on(grid(), "charton -g contour -x x -y y --z z")?;
        assert!(out.as_str()?.contains("<svg"));
        // `--bins` sets the number of iso-levels.
        let out = run_on(grid(), "charton -g contour -x x -y y --z z --bins 4")?;
        assert!(out.as_str()?.contains("<svg"));
        // `--stroke` gives a single-colour contour instead of a level colormap.
        let out = run_on(grid(), "charton -g contour -x x -y y --z z --stroke black")?;
        assert!(out.as_str()?.contains("<svg"));
        // Without `--z`, a 2D density is estimated first (density contour).
        let out = run_on(grid(), "charton -g contour -x x -y y")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn violin_returns_svg() -> Result<(), ShellError> {
        // One violin per category.
        let out = run("charton -g violin -x species -y petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        // Grouped (dodged) violins, one per `grp`.
        let out = run("charton -g violin -x species -y petal_length -c grp")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn violin_accepts_columns_named_x_and_y() -> Result<(), ShellError> {
        // The composition's intermediate columns are namespaced, so a category
        // column literally called `x` (and a value column called `y`) is fine.
        let table = Value::test_list(vec![
            Value::test_record(record! {
                "x" => Value::test_string("a"),
                "y" => Value::test_float(1.0),
            }),
            Value::test_record(record! {
                "x" => Value::test_string("a"),
                "y" => Value::test_float(2.0),
            }),
            Value::test_record(record! {
                "x" => Value::test_string("b"),
                "y" => Value::test_float(3.0),
            }),
            Value::test_record(record! {
                "x" => Value::test_string("b"),
                "y" => Value::test_float(4.0),
            }),
        ]);
        let out = run_on(table, "charton -g violin -x x -y y")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn errorbar_aggregates_without_y2() -> Result<(), ShellError> {
        let out = run("charton -g errorbar -x species -y petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn errorbar_accepts_explicit_bounds() -> Result<(), ShellError> {
        let out = run("charton -g errorbar -x t -y lo --y2 hi")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn geo_reads_geojson_file() -> Result<(), ShellError> {
        let path = std::env::temp_dir().join("charton_test_geo.geojson");
        std::fs::write(
            &path,
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"name":"A"},"geometry":{"type":"Polygon","coordinates":[[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0],[0.0,0.0]]]}}]}"#,
        )
        .unwrap();
        let arg = path.to_string_lossy().replace('\\', "/");
        let out = run(&format!("charton -g geo --geojson '{arg}'"));
        let _ = std::fs::remove_file(&path);
        assert!(out?.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn facet_wrap_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g point -x t -y petal_length --facet-wrap species")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn facet_grid_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g point -x t -y petal_length --facet-row species --facet-col grp")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn incomplete_facet_grid_is_an_error() {
        assert!(run("charton -g point -x t -y petal_length --facet-row species").is_err());
    }

    #[test]
    fn rect_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g rect -x species -y grp -c petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn rule_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g rule -x t -y lo --y2 hi")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn text_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g text -x t -y petal_length --text species")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn text_requires_text_column() {
        assert!(run("charton -g text -x t -y petal_length").is_err());
    }

    #[test]
    fn tick_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g tick -x t -y petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn layer_overlay_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g line -x t -y petal_length --layer {geom: point}")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn layer_list_and_bad_layer() -> Result<(), ShellError> {
        let out = run(
            "charton -g line -x t -y petal_length --layer [{geom: point} {geom: rule, y2: hi}]",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        assert!(run("charton -g line -x t -y petal_length --layer {x: t}").is_err());
        Ok(())
    }

    #[test]
    fn labels_appear_in_svg() -> Result<(), ShellError> {
        let out = run(
            "charton -g bar -x species -y petal_length --x-label FooAxis --y-label BarAxis --color-label ColAxis -c grp",
        )?;
        let svg = out.as_str()?;
        assert!(svg.contains("FooAxis") && svg.contains("BarAxis") && svg.contains("ColAxis"));
        Ok(())
    }

    #[test]
    fn style_and_flip_render() -> Result<(), ShellError> {
        let out = run(
            "charton -g point -x t -y petal_length --opacity 0.5 --size 8 --mark-color red --flip",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn incomplete_domain_is_an_error() {
        assert!(run("charton -g point -x t -y petal_length --x-min 0").is_err());
        assert!(run("charton -g point -x t -y petal_length --y-max 10").is_err());
    }

    #[test]
    fn full_domain_renders() -> Result<(), ShellError> {
        let out =
            run("charton -g point -x t -y petal_length --x-min 0 --x-max 10 --y-min 0 --y-max 10")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn missing_xy_is_an_error() {
        assert!(run("charton -g bar").is_err());
    }

    #[test]
    fn unknown_geom_is_an_error() {
        assert!(run("charton -g pie -x species -y petal_length").is_err());
    }

    #[test]
    fn colorfgbg_selects_a_mode() {
        assert_eq!(parse_colorfgbg("15;0"), Some(ThemeMode::Dark));
        assert_eq!(parse_colorfgbg("0;15"), Some(ThemeMode::Light));
        assert_eq!(parse_colorfgbg("7"), Some(ThemeMode::Light));
        assert_eq!(parse_colorfgbg("8"), Some(ThemeMode::Dark));
        assert_eq!(parse_colorfgbg("not-a-color"), None);
    }

    #[test]
    fn log_scale_renders() -> Result<(), ShellError> {
        let out = run("charton -g point -x t -y petal_length --y-scale log")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn invalid_scale_is_an_error() {
        assert!(run("charton -g point -x t -y petal_length --y-scale nope").is_err());
    }

    #[test]
    fn color_map_renders_and_rejects_unknown() -> Result<(), ShellError> {
        let out = run("charton -g rect -x species -y grp -c petal_length --color-map magma")?;
        assert!(out.as_str()?.contains("<svg"));
        assert!(run("charton -g rect -x species -y grp -c petal_length --color-map nope").is_err());
        Ok(())
    }

    #[test]
    fn label_format_preset_and_record_render() -> Result<(), ShellError> {
        let out = run("charton -g bar -x species -y petal_length --y-format compact")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run(
            "charton -g bar -x species -y petal_length --y-format {compact: true, prefix: '$'}",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        assert!(run("charton -g bar -x species -y petal_length --y-format nope").is_err());
        Ok(())
    }

    #[test]
    fn mark_options_render() -> Result<(), ShellError> {
        let out = run("charton -g point -x t -y petal_length --shape square")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton -g line -x t -y petal_length --dash 6,4 --interpolation step")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton -g boxplot -x species -y petal_length --outlier-color red")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn invalid_dash_is_an_error() {
        assert!(run("charton -g line -x t -y petal_length --dash abc").is_err());
    }

    #[test]
    fn layer_style_override_renders() -> Result<(), ShellError> {
        let out = run(
            "charton -g bar -x species -y petal_length --layer {geom: point, style: {color: red, size: 4}}",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn background_flag_overrides_theme() -> Result<(), ShellError> {
        let out =
            run("charton -g bar -x species -y petal_length --theme dark --background '#ff0000'")?;
        assert!(out.as_str()?.contains("rgba(255,0,0,1.000)"));
        Ok(())
    }

    #[test]
    fn polar_charts_render() -> Result<(), ShellError> {
        // Rose / Nightingale: `-x` maps to the angle (bar mark in polar).
        let out = run("charton -g bar -x species -y petal_length --coord polar")?;
        assert!(out.as_str()?.contains("<svg"));
        // Donut adds an inner radius.
        let out =
            run("charton -g bar -x species -y petal_length --coord polar --inner-radius 0.5")?;
        assert!(out.as_str()?.contains("<svg"));
        // A partial angular span (rose / nightingale style).
        let out = run(
            "charton -g bar -x species -y petal_length --coord polar --start-angle 0 --end-angle 270",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn polar_pie_and_donut_without_x() -> Result<(), ShellError> {
        // Omitting `-x` selects charton's pie mode: the y values become the
        // slices and the colour column separates them, so percentage labels
        // are drawn.
        let pie = run("charton -g bar -y petal_length -c species --coord polar")?;
        assert!(pie.as_str()?.contains('%'), "pie mode should draw % labels");
        let donut =
            run("charton -g bar -y petal_length -c species --coord polar --inner-radius 0.5")?;
        assert!(donut.as_str()?.contains("<svg"));
        // Outside polar coordinates, an omitted x is still an error.
        assert!(run("charton -g bar -y petal_length").is_err());
        Ok(())
    }

    #[test]
    fn invalid_coord_is_an_error() {
        assert!(run("charton -g bar -x species -y petal_length --coord spherical").is_err());
    }

    #[test]
    fn stacking_renders() -> Result<(), ShellError> {
        let out = run("charton -g bar -x species -y petal_length -c grp --stack stacked")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton -g area -x t -y petal_length -c grp --stack normalize")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton -g bar -x species -y petal_length -c grp --normalize")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn invalid_stack_is_an_error() {
        assert!(run("charton -g bar -x species -y petal_length -c grp --stack wobble").is_err());
    }

    #[test]
    fn mark_geometry_options_render() -> Result<(), ShellError> {
        let out = run("charton -g bar -x species -y petal_length --mark-width 0.5")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton -g errorbar -x t -y lo --y2 hi --cap-length 4 --no-center")?;
        assert!(out.as_str()?.contains("<svg"));
        let out =
            run("charton -g boxplot -x species -y petal_length --no-outliers --outlier-size 3")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn size_and_shape_encoding_render() -> Result<(), ShellError> {
        let out = run(
            "charton -g point -x t -y petal_length --size-by petal_length --size-label Weight",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        let out =
            run("charton -g point -x t -y petal_length --shape-by species --shape-label Species")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn unsupported_output_extension_is_an_error() {
        let path = std::env::temp_dir().join("charton_test_output.pdf");
        let arg = path.to_string_lossy().replace('\\', "/");
        let res = run(&format!(
            "charton -g bar -x species -y petal_length -o '{arg}'"
        ));
        let _ = std::fs::remove_file(&path);
        assert!(
            res.is_err(),
            "a .pdf output must be rejected, not written as SVG"
        );
    }

    #[test]
    fn density_and_ecdf_render() -> Result<(), ShellError> {
        let out = run("charton -g density -x petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton -g density -x petal_length -c species --cumulative")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton -g density -x petal_length --density-kernel uniform")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton -g ecdf -x petal_length -c species")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn density_opacity_is_opt_in() -> Result<(), ShellError> {
        // No implicit transparency: like charton's KDE example, the caller asks
        // for a translucent fill explicitly.
        let out = run("charton -g density -x petal_length -c species")?;
        assert!(!out.as_str()?.contains("fill-opacity=\"0.500\""));
        // --opacity is applied as written.
        let out = run("charton -g density -x petal_length -c species --opacity 0.5")?;
        assert!(out.as_str()?.contains("fill-opacity=\"0.500\""));
        Ok(())
    }

    #[test]
    fn invalid_density_kernel_is_an_error() {
        assert!(run("charton -g density -x petal_length --density-kernel cosine").is_err());
    }

    #[test]
    fn aggregate_bins_and_loess_render() -> Result<(), ShellError> {
        let out = run("charton -g bar -x species -y petal_length --aggregate mean")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton -g hist -x petal_length --bins 5")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton -g line -x t -y petal_length --loess --loess-bandwidth 0.5")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn invalid_aggregate_is_an_error() {
        assert!(run("charton -g bar -x species -y petal_length --aggregate mode").is_err());
    }

    #[test]
    fn layout_overrides_render() -> Result<(), ShellError> {
        let out = run(
            "charton -g point -x t -y petal_length --margins 0.1,0.1,0.1,0.1 --x-expand 0.2 --y-ticks [0 2 4 6]",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        assert!(run("charton -g point -x t -y petal_length --margins 1,2,3").is_err());
        assert!(run("charton -g point -x t -y petal_length --y-ticks a").is_err());
        Ok(())
    }
}
