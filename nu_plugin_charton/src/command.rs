//! The `charton` command: read a pipeline table, build a charton chart, then
//! either render it inline in the terminal, save it, or return the raw data.

use std::io::{IsTerminal, Write};
use std::path::Path;

use charton::error::ChartonError;
use charton::prelude::{
    Chart, ColorPalette, CoordSystem, Dataset, FacetSpec, IntoLayered, LayeredChart, MarkArea,
    MarkBar, MarkBoxplot, MarkErrorBar, MarkLine, MarkPoint, MarkRect, MarkRule, MarkText,
    MarkTick, alt, geojson_to_dataset,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    IntoPipelineData, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type, Value,
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
                "Mark type: point | line | area | bar | boxplot | errorbar | rule | tick | text | rect | hist | beeswarm | geo",
                Some('g'),
            )
            .named("x", SyntaxShape::String, "Column mapped to the x axis", Some('x'))
            .named("y", SyntaxShape::String, "Column mapped to the y axis", Some('y'))
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
            .switch("grid", "Show grid lines", None)
            .switch("no-grid", "Hide grid lines", None)
            .named(
                "legend",
                SyntaxShape::String,
                "Legend position: left | right | top | bottom | none",
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
        };
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
        let scale = call
            .get_flag::<f64>("scale")?
            .map(|v| v as f32)
            .or(cfg.scale)
            .filter(|s| *s > 0.0)
            .unwrap_or(2.0);
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
            y2: y2.clone(),
            color: color.clone(),
            text: text.clone(),
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

        // 3. Build the (layered) chart.
        let opts = BuildOpts {
            primary,
            layers,
            style,
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
            background: cfg.background.clone(),
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
            let bytes = render::to_png(&chart)
                .map_err(|e| LabeledError::new("PNG rendering failed").with_label(e, span))?;
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

            let art = match style {
                render::InlineStyle::Iterm2 => {
                    let (w, h) = render::png_dimensions(&bytes).map_err(|e| {
                        LabeledError::new("Inline rendering failed").with_label(e, span)
                    })?;
                    let (c, r) = render::fit_cells(w, h, max_cols, max_rows);
                    render::iterm2_image(&bytes, c, r)
                }
                render::InlineStyle::Kitty => {
                    let (w, h) = render::png_dimensions(&bytes).map_err(|e| {
                        LabeledError::new("Inline rendering failed").with_label(e, span)
                    })?;
                    let (c, r) = render::fit_cells(w, h, max_cols, max_rows);
                    render::kitty_image(&bytes, c, r)
                }
                render::InlineStyle::Sixel => render::sixel_image(&bytes, max_cols, max_rows)
                    .map_err(|e| {
                        LabeledError::new("Inline rendering failed").with_label(e, span)
                    })?,
                // `Auto` already resolved above; treat as the universal fallback.
                render::InlineStyle::HalfBlock | render::InlineStyle::Auto => {
                    render::png_to_halfblock(&bytes, max_cols, max_rows).map_err(|e| {
                        LabeledError::new("Inline rendering failed").with_label(e, span)
                    })?
                }
            };

            let mut out = std::io::stdout();
            let _ = out.write_all(art.as_bytes());
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

/// One chart layer. `x`/`color` default to the primary layer for `--layer`
/// overlays when omitted.
struct LayerOpts {
    geom: String,
    x: Option<String>,
    y: Option<String>,
    y2: Option<String>,
    color: Option<String>,
    text: Option<String>,
}

struct BuildOpts<'a> {
    primary: LayerOpts,
    layers: Vec<LayerOpts>,
    style: Style,
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
    width: u32,
    height: u32,
    scale: f32,
    span: Span,
}

/// Mark-level visual overrides applied to every layer where the mark supports
/// them. Unsupported options are ignored for that mark (e.g. `--size` on bars).
#[derive(Clone, Default)]
struct Style {
    color: Option<String>,
    opacity: Option<f64>,
    size: Option<f64>,
    stroke: Option<String>,
    stroke_width: Option<f64>,
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

    let mut layered = build_layer(dataset.clone(), &opts.primary, &opts.style, span)?;
    for extra in &opts.layers {
        layered = layered.and(build_layer(dataset.clone(), extra, &opts.style, span)?);
    }
    Ok(finish(layered, opts))
}

/// Build a single chart layer. Layers are combined with `.and()` so they share
/// one scale and coordinate system.
fn build_layer(
    dataset: Dataset,
    layer: &LayerOpts,
    style: &Style,
    span: Span,
) -> Result<LayeredChart, LabeledError> {
    let geom = layer.geom.as_str();
    let (x, y, y2) = (layer.x.as_deref(), layer.y.as_deref(), layer.y2.as_deref());
    let color = layer.color.as_deref();

    // Encode `(x, y [, color])`, requiring both x and y. Mark styling is applied
    // *before* `encode` so mark-dependent transforms see the final settings.
    macro_rules! enc_xy_color {
        ($chart:expr) => {
            match (x, y, color) {
                (Some(x), Some(y), Some(col)) => {
                    $chart.encode((alt::x(x), alt::y(y), alt::color(col)))
                }
                (Some(x), Some(y), None) => $chart.encode((alt::x(x), alt::y(y))),
                _ => return Err(missing(geom, "both --x and --y", span)),
            }
            .map_err(|e| chart_err(span, e))?
        };
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
            enc_xy_color!(c).into()
        }
        "boxplot" | "box" => {
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_boxplot()
                .map_err(|e| chart_err(span, e))?
                .configure_boxplot(|m| style_boxplot(m, style));
            enc_xy_color!(c).into()
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
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_errorbar()
                .map_err(|e| chart_err(span, e))?
                .configure_errorbar(|m| style_errorbar(m, style));
            // With --y2: explicit min/max columns. Without: charton aggregates
            // the raw y values per group into mean +/- std.
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
        "hist" | "histogram" => {
            let x = x.ok_or_else(|| missing(geom, "--x", span))?;
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .mark_hist()
                .map_err(|e| chart_err(span, e))?;
            // charton computes the bin counts into the y field named here.
            c.encode((alt::x(x), alt::y("count")))
                .map_err(|e| chart_err(span, e))?
                .into()
        }
        other => {
            return Err(LabeledError::new("Unknown geom").with_label(
                format!(
                    "'{other}' is not supported; try point, line, area, bar, boxplot, \
                     errorbar, rule, tick, text, rect, hist, beeswarm, or geo"
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
        Value::List { vals, .. } => vals,
        other => vec![other],
    };

    let mut out = Vec::with_capacity(records.len());
    for (i, record) in records.into_iter().enumerate() {
        let Value::Record { val, .. } = &record else {
            return Err(LabeledError::new("Invalid layer")
                .with_label(format!("layer {i} must be a record"), record.span()));
        };
        let field = |k: &str| val.get(k).and_then(|v| v.as_str().ok().map(str::to_string));
        let geom = field("geom").or_else(|| field("mark")).ok_or_else(|| {
            LabeledError::new("Invalid layer")
                .with_label(format!("layer {i} is missing `geom`"), span)
        })?;
        out.push(LayerOpts {
            geom: geom.to_lowercase(),
            x: field("x").or_else(|| primary.x.clone()),
            y: field("y").or_else(|| primary.y.clone()),
            y2: field("y2").or_else(|| primary.y2.clone()),
            color: field("color").or_else(|| primary.color.clone()),
            text: field("text").or_else(|| primary.text.clone()),
        });
    }
    Ok(out)
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
    if let Some((min, max)) = opts.x_domain {
        l = l.with_x_domain(min, max);
    }
    if let Some((min, max)) = opts.y_domain {
        l = l.with_y_domain(min, max);
    }
    if opts.flip {
        l = l.coord_flip();
    }
    if let Some(facet) = &opts.facet {
        l = l.facet(facet.clone());
    }
    if let Some(grid) = opts.grid {
        l = l.with_grid(grid);
    }
    if opts.palette.is_some()
        || opts.legend.is_some()
        || opts.x_angle.is_some()
        || opts.background.is_some()
    {
        l = l.configure_theme(|mut t| {
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
            if let Some(bg) = &opts.background {
                t = t.with_background_color(bg.as_str());
            }
            t
        });
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

    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("png"))
    {
        let bytes = render::to_png(chart)
            .map_err(|e| LabeledError::new("PNG rendering failed").with_label(e, span))?;
        std::fs::write(path, bytes).map_err(write_err)?;
    } else {
        let svg = render::to_svg(chart)
            .map_err(|e| LabeledError::new("SVG rendering failed").with_label(e, span))?;
        std::fs::write(path, svg).map_err(write_err)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn bar_returns_svg() -> Result<(), ShellError> {
        let out = run("charton -g bar -x species -y petal_length")?;
        assert!(out.as_str()?.trim_start().starts_with("<svg"));
        Ok(())
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
}
