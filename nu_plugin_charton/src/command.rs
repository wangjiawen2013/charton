//! The `charton` Nushell plugin.
//!
//! A Nushell pipeline is a table; `charton` turns one into a chart and either
//! draws it inline in the terminal, saves it (`-o chart.svg`), or hands the
//! image back to the pipeline (`--raw`).
//!
//! # Commands
//!
//! Every mark is its own subcommand — `charton point`, `charton bar`,
//! `charton density`, ... — registered from [`MARKS`]. A subcommand declares
//! only the flags it understands, so `charton <TAB>` and `help charton bar`
//! stay focused. `charton` itself is a parent that lists the marks.
//!
//! The API follows Charton's grammar of graphics, in widening levels:
//! - **a mark** (`-x` / `-y` and friends) draws the rows;
//! - **`--layer`** stacks more marks on the same data;
//! - **`--transform`** rewrites the table first (Charton's public transforms).
//!
//! `--layer` and `--transform` combine into full recipes — ridgeline, ECDF,
//! candlestick, ... — the README's Level 4.
//!
//! # Layout
//!
//! The file is organised in two halves:
//! - the **command layer** (top): one [`MarkCommand`] per mark, its signature
//!   ([`base_signature`] plus [`mark_signature`]), flag completions, and
//!   [`run_chart`], which reads the flags into a [`LayerOpts`];
//! - the **build engine** (below): [`build_chart`] ingests the pipeline table
//!   and calls [`build_layer`] per layer, and [`finish`] applies the chart-wide
//!   options (title, theme, facets, coordinates, size).
//!
//! A layer is built by the big mark `match` in [`build_layer`] — or, when it
//! carries `--transform` or is a `path`/`polygon`, by [`build_primitive_layer`],
//! which walks the [`TransformSpec`] chain and then draws a Tier-1 mark.

use std::io::{IsTerminal, Write};
use std::path::Path;

use charton::error::ChartonError;
use charton::prelude::{
    AnyValue, BandScale, BandSide, BandTransform, BandwidthType, Chart, ColorMap, ColorPalette,
    ContourTransform, CoordSystem, Dataset, Density2DTransform, DensityTransform, Expansion,
    FacetSpec, IntoLayered, KernelType, LabelFormat, LayeredChart, MarkArea, MarkBar, MarkBoxplot,
    MarkErrorBar, MarkGeoPath, MarkLine, MarkPoint, MarkRect, MarkRule, MarkText, MarkTick,
    Position, QuantileBoxTransform, RowAccessor, Scale, ThemeMode, WindowFieldDef, WindowOnlyOp,
    WindowTransform, alt, geojson_to_dataset,
};
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    DynamicSuggestion, Example, IntoPipelineData, LabeledError, PipelineData, Record, Signature,
    Span, Spanned, SyntaxShape, Type, Value, engine::ArgType,
};

use crate::ChartonPlugin;
use crate::config::{self, Config, LegendSetting};
use crate::converter::Table;
use crate::render;

/// Default multiplier for every chart font. Charton's base sizes target a
/// standalone figure; fitted into a terminal they look small, so the plugin
/// enlarges them. Override with `--font-scale` or the `font_scale` config key.
pub(crate) const DEFAULT_FONT_SCALE: f64 = 1.5;

/// Every mark subcommand: `(command suffix, charton geom)`.
///
/// The command name is `charton <suffix>`; the geom is the internal name used
/// by the build engine and by `--layer` records.
pub const MARKS: &[(&str, &str)] = &[
    // Tier 1 — geometry primitives (stat = identity).
    ("point", "point"),
    ("line", "line"),
    ("path", "path"),
    ("polygon", "polygon"),
    ("bar", "bar"),
    ("area", "area"),
    ("rect", "rect"),
    ("rule", "rule"),
    ("text", "text"),
    ("tick", "tick"),
    // Tier 2 — statistical atoms (canonical statistic × Tier-1 geometry).
    ("hist", "hist"),
    ("boxplot", "boxplot"),
    ("violin", "violin"),
    ("errorbar", "errorbar"),
    ("density", "density"),
    ("density-2d", "density_2d"),
    ("contour", "contour"),
    // `mark_geoshape`, fed by a GeoJSON file.
    ("geo", "geo"),
];

/// The parent `charton` command.
///
/// It exists so that `help charton` lists every mark as a subcommand. The
/// actual drawing is done by [`MarkCommand`].
pub struct Charton;

impl PluginCommand for Charton {
    type Plugin = ChartonPlugin;

    fn name(&self) -> &str {
        "charton"
    }

    fn description(&self) -> &str {
        "Draw a chart from a pipeline table; pick a mark, e.g. `charton bar`"
    }

    fn signature(&self) -> Signature {
        Signature::build("charton").input_output_type(Type::Any, Type::Any)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["plot", "chart", "graph", "visualize", "svg", "png"]
    }

    fn extra_description(&self) -> &str {
        "charton is a family of mark subcommands. Call one with the encoding it \
         needs, for example `charton bar -x category -y value`. \
         `help commands | where name =~ '^charton'` lists every mark."
    }

    fn run(
        &self,
        _plugin: &ChartonPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let names: Vec<&str> = MARKS.iter().map(|(name, _)| *name).collect();
        Ok(Value::string(
            format!(
                "charton: choose a mark, e.g. `charton bar`. Marks: {}",
                names.join(", ")
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}

/// A `charton <mark>` subcommand.
pub struct MarkCommand {
    /// Full command name, e.g. `"charton bar"`.
    pub name: String,
    /// The charton mark this command draws, e.g. `"bar"`.
    pub mark: &'static str,
}

impl MarkCommand {
    pub fn new(name: String, mark: &'static str) -> Self {
        Self { name, mark }
    }
}

impl PluginCommand for MarkCommand {
    type Plugin = ChartonPlugin;

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        mark_description(self.mark)
    }

    fn signature(&self) -> Signature {
        mark_signature(&self.name, self.mark)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["plot", "chart", "graph", "visualize", "svg", "png"]
    }

    fn extra_description(&self) -> &str {
        "Turns a pipeline table into a chart. The columns named by the flags drive \
         the plot; everything else is set by options. The result is drawn inline in \
         an image-capable terminal, or written to a file with `-o chart.svg`."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        mark_examples(&self.name, self.mark)
    }

    #[allow(deprecated)]
    fn get_dynamic_completion(
        &self,
        _plugin: &ChartonPlugin,
        _engine: &EngineInterface,
        _call: DynamicCompletionCall,
        arg_type: ArgType,
        _experimental: nu_protocol::engine::ExperimentalMarker,
    ) -> Option<Vec<DynamicSuggestion>> {
        complete_arg(arg_type)
    }

    fn run(
        &self,
        _plugin: &ChartonPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        run_chart(self.mark, _plugin, engine, call, input)
    }
}

/// A one-line description for a mark, shown by `help charton <mark>`.
fn mark_description(mark: &str) -> &'static str {
    match mark {
        "point" => "Scatter/bubble plot: one shape per row",
        "line" => "Connect rows sorted by x to show a trend",
        "path" => "Connect rows in row order, grouped by --path-group (open)",
        "polygon" => "Close and fill rows grouped by --path-group",
        "bar" => "Rectangles from zero to y, one per category",
        "area" => "Filled area between y and a baseline",
        "rect" => "Heatmap cells: colour a value over an x/y grid",
        "rule" => "A horizontal or vertical line segment",
        "tick" => "A short tick mark per row",
        "text" => "Place a text label at each x/y",
        "hist" => "Histogram: count rows per bin",
        "boxplot" => "Five-number summary with Tukey outliers",
        "violin" => "Kernel density outline plus an inner box",
        "errorbar" => "Confidence interval / error bars",
        "density" => "1-D kernel density estimate",
        "density_2d" => "Joint 2-D kernel density drawn as a grid",
        "contour" => "Iso-lines of a scalar x/y/z grid",
        "geo" => "GeoJSON regions coloured by a value",
        _ => "Draw a chart",
    }
}

/// Flags shared by every mark: encoding, axes, theme, facets, output.
fn base_signature(name: &str) -> Signature {
    Signature::build(name)
        .named("x", SyntaxShape::String, "Column mapped to the x axis", Some('x'))
        .named("y", SyntaxShape::String, "Column mapped to the y axis", Some('y'))
        .named(
            "color",
            SyntaxShape::String,
            "Column mapped to color/grouping",
            Some('c'),
        )
        .named(
            "path-group",
            SyntaxShape::String,
            "Column grouping rows into one path/polygon (for `path` and `polygon`)",
            None,
        )
        .named(
            "transform",
            SyntaxShape::Any,
            "Transform(s) applied before the mark: one record or a list, in order. \
             ops: density | density-2d | contour | band | quantile-box | window | calculate",
            None,
        )
        .named(
            "output",
            SyntaxShape::String,
            "Save the chart to a file (.svg or .png)",
            Some('o'),
        )
        .named("title", SyntaxShape::String, "Chart title", Some('t'))
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
        .named("start-angle", SyntaxShape::Number, "Polar start angle in degrees (default 0)", None)
        .named("end-angle", SyntaxShape::Number, "Polar end angle in degrees (default 360)", None)
        .named(
            "stack",
            SyntaxShape::String,
            "Bar/area stacking: none | stacked | normalize | center",
            None,
        )
        .switch("normalize", "Normalize y values (hist/bar) to proportions", None)
        .named(
            "aggregate",
            SyntaxShape::String,
            "Aggregate y per x group: sum | mean | median | min | max | count",
            None,
        )
        .named(
            "x-expand",
            SyntaxShape::Any,
            "Padding on the x axis: a number (fraction of the range) or [low, high] (data units)",
            None,
        )
        .named(
            "y-expand",
            SyntaxShape::Any,
            "Padding on the y axis: a number (fraction of the range) or [low, high] (data units)",
            None,
        )
        .named("x-ticks", SyntaxShape::Any, "Explicit x tick values, e.g. [0 1 2 3]", None)
        .named("y-ticks", SyntaxShape::Any, "Explicit y tick values, e.g. [0 1 2 3]", None)
        .named(
            "margins",
            SyntaxShape::String,
            "Canvas margins as 'top,right,bottom,left' (fractions 0.0-1.0)",
            None,
        )
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
        .named("stroke", SyntaxShape::String, "Stroke color for the mark", None)
        .named("stroke-width", SyntaxShape::Number, "Stroke width for the mark", None)
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
        .named("y-format", SyntaxShape::Any, "Y tick label format (same forms as --x-format)", None)
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
        .named("width", SyntaxShape::Int, "Pixel width of the rendered chart", None)
        .named("height", SyntaxShape::Int, "Pixel height of the rendered chart", None)
        .named(
            "font-scale",
            SyntaxShape::Number,
            "Scale every chart font (title, labels, ticks, legend); default 1.5",
            None,
        )
        .named("scale", SyntaxShape::Number, "Raster pixel scale factor (default 2.0)", None)
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
            "layer",
            SyntaxShape::Any,
            "Extra layer(s): a geom name, a layer record, or a list of them",
            None,
        )
        .named("facet-wrap", SyntaxShape::String, "Wrap panels by this column", None)
        .named("facet-columns", SyntaxShape::Int, "Number of columns for --facet-wrap", None)
        .named("facet-row", SyntaxShape::String, "Row field for a facet grid", None)
        .named("facet-col", SyntaxShape::String, "Column field for a facet grid", None)
        .named(
            "facet-strategy",
            SyntaxShape::String,
            "Facet scale strategy: fixed | free (default fixed)",
            None,
        )
        .input_output_type(Type::Any, Type::Any)
}

/// The full signature for a mark: the shared flags plus only the flags that
/// mark understands.
fn mark_signature(name: &str, mark: &str) -> Signature {
    let sig = base_signature(name);
    match mark {
        "point" => sig
            .named("size", SyntaxShape::Number, "Mark size", None)
            .named(
                "shape",
                SyntaxShape::String,
                "Point shape: circle | square | triangle | star | diamond | pentagon | hexagon | octagon",
                None,
            )
            .named(
                "size-by",
                SyntaxShape::String,
                "Column mapped to point size (bubble charts)",
                None,
            )
            .named("shape-by", SyntaxShape::String, "Column mapped to point shape", None)
            .named("size-label", SyntaxShape::String, "Size legend label", None)
            .named("shape-label", SyntaxShape::String, "Shape legend label", None)
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
                "mark-width",
                SyntaxShape::Number,
                "Mark band width as a fraction 0.0-1.0",
                None,
            ),
        "line" => sig
            .named(
                "interpolation",
                SyntaxShape::String,
                "Line interpolation: linear | step | step-before",
                None,
            )
            .named("dash", SyntaxShape::String, "Dash pattern, e.g. '6,4' (on, off) in pixels", None)
            .switch("loess", "Smooth the line with LOESS", None)
            .named("loess-bandwidth", SyntaxShape::Number, "LOESS bandwidth 0.0-1.0", None),
        "bar" => sig
            .named(
                "mark-width",
                SyntaxShape::Number,
                "Bar width as a fraction 0.0-1.0",
                None,
            )
            .named(
                "y2",
                SyntaxShape::String,
                "Second y value; with -y it draws a floating bar from y to y2",
                None,
            ),
        "boxplot" => sig
            .named("mark-width", SyntaxShape::Number, "Box width as a fraction 0.0-1.0", None)
            .switch("no-outliers", "Hide boxplot outlier points", None)
            .named("outlier-size", SyntaxShape::Number, "Boxplot outlier point size", None)
            .named("outlier-color", SyntaxShape::String, "Outlier point color", None),
        "errorbar" => sig
            .named(
                "y2",
                SyntaxShape::String,
                "Upper bound column for the error bar",
                None,
            )
            .named("mark-width", SyntaxShape::Number, "Error bar width 0.0-1.0", None)
            .named("cap-length", SyntaxShape::Number, "Errorbar cap length in pixels", None)
            .switch("no-center", "Hide the center dot of error bars", None),
        "rule" => sig.named(
            "y2",
            SyntaxShape::String,
            "Second y value: draws a rule from -y to --y2",
            None,
        ),
        "hist" => sig.named("bins", SyntaxShape::Int, "Number of histogram bins", None),
        "density" => sig
            .named(
                "density-bandwidth",
                SyntaxShape::Number,
                "KDE bandwidth (in data units; default Scott's rule)",
                None,
            )
            .named(
                "density-kernel",
                SyntaxShape::String,
                "KDE kernel: normal | epanechnikov | uniform",
                None,
            )
            .switch("cumulative", "Cumulative density instead of density", None)
            .switch("counts", "Smoothed counts instead of probabilities", None),
        "density_2d" => sig
            .named(
                "bins",
                SyntaxShape::Int,
                "Grid size per axis for the 2-D density estimate",
                None,
            )
            .named(
                "cell-padding",
                SyntaxShape::Number,
                "Extra pixels per cell side: closes tile seams (default 0.5); negative opens a gap",
                None,
            ),
        "rect" => sig.named(
            "cell-padding",
            SyntaxShape::Number,
            "Extra pixels per cell side: closes tile seams (default 0.5); negative opens a gap",
            None,
        ),
        "contour" => sig
            .named(
                "z",
                SyntaxShape::String,
                "Value column; omit it to estimate a 2-D density first",
                None,
            )
            .named("bins", SyntaxShape::Int, "Number of iso-levels", None),
        "text" => sig
            .named("text", SyntaxShape::String, "Label column", None)
            .named("size", SyntaxShape::Number, "Font size", None)
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
            ),
        "tick" => sig.named("size", SyntaxShape::Number, "Tick thickness", None),
        "geo" => sig.named(
            "geojson",
            SyntaxShape::String,
            "GeoJSON file to render",
            None,
        ),
        _ => sig,
    }
}

/// Static examples for a mark. `Box::leak` is used so the example strings can
/// carry the subcommand name while still being `&'static str`; this runs only
/// during plugin registration / `help`.
fn mark_examples(name: &str, mark: &str) -> Vec<Example<'static>> {
    fn ex(description: &'static str, example: String) -> Example<'static> {
        Example {
            example: Box::leak(example.into_boxed_str()),
            description,
            result: None,
        }
    }
    let data = "[[x y g]; [1 2 a] [2 3 b] [3 5 a] [4 4 b]]";
    match mark {
        "point" => vec![ex(
            "Scatter plot of two columns",
            format!("{data} | {name} -x x -y y -c g"),
        )],
        "line" => vec![ex(
            "Line chart with a step interpolation",
            format!("{data} | {name} -x x -y y --interpolation step"),
        )],
        "bar" => vec![ex(
            "Bar chart per category",
            format!("{data} | {name} -x g -y y --aggregate mean"),
        )],
        "density" => vec![ex(
            "Kernel density estimate",
            format!("{data} | {name} -x y"),
        )],
        "hist" => vec![ex(
            "Histogram with 5 bins",
            format!("{data} | {name} -x y --bins 5"),
        )],
        _ => vec![],
    }
}

/// Build a completion list for a flag value.
fn complete_arg(arg_type: ArgType) -> Option<Vec<DynamicSuggestion>> {
    let ArgType::Flag(flag) = arg_type else {
        return None;
    };
    let values: &[(&str, &str)] = match flag.as_ref() {
        "coord" => &[("cartesian", "default"), ("polar", "rose / pie / donut")],
        "stack" => &[
            ("none", "no stacking"),
            ("stacked", "stack values"),
            ("normalize", "stack to proportions"),
            ("center", "centered stream"),
        ],
        "aggregate" => &[
            ("sum", ""),
            ("mean", ""),
            ("median", ""),
            ("min", ""),
            ("max", ""),
            ("count", ""),
        ],
        "density-kernel" => &[
            ("normal", "Gaussian"),
            ("epanechnikov", ""),
            ("uniform", ""),
        ],
        "shape" => &[
            ("circle", ""),
            ("square", ""),
            ("triangle", ""),
            ("star", ""),
            ("diamond", ""),
            ("pentagon", ""),
            ("hexagon", ""),
            ("octagon", ""),
        ],
        "interpolation" => &[("linear", ""), ("step", ""), ("step-before", "")],
        "anchor" => &[("start", ""), ("middle", ""), ("end", "")],
        "weight" => &[("normal", ""), ("bold", "")],
        "layout" => &[
            ("standard", ""),
            ("jitter", ""),
            ("beeswarm", ""),
            ("quasirandom", ""),
        ],
        "quasirandom-method" => &[("tukey", ""), ("pseudorandom", "")],
        "x-scale" | "y-scale" => &[
            ("linear", ""),
            ("log", ""),
            ("discrete", ""),
            ("temporal", ""),
        ],
        "legend" => &[
            ("left", ""),
            ("right", ""),
            ("top", ""),
            ("bottom", ""),
            ("none", "hide the legend"),
        ],
        "theme" => &[("auto", ""), ("light", ""), ("dark", "")],
        "inline-style" => &[
            ("auto", ""),
            ("halfblock", ""),
            ("iterm2", ""),
            ("kitty", ""),
            ("sixel", ""),
        ],
        "facet-strategy" => &[("fixed", ""), ("free", ""), ("free_x", ""), ("free_y", "")],
        "color-map" => &[
            ("viridis", ""),
            ("magma", ""),
            ("inferno", ""),
            ("plasma", ""),
            ("cividis", ""),
            ("blues", ""),
            ("greens", ""),
            ("ylgnbu", ""),
        ],
        "layer" => &[
            ("point", "add a point layer"),
            ("line", "add a line layer"),
            ("bar", "add a bar layer"),
            ("rule", "add a rule layer"),
            ("errorbar", "add an error bar layer"),
            ("text", "add a text layer"),
        ],
        _ => return None,
    };
    Some(
        values
            .iter()
            .map(|(value, description)| DynamicSuggestion {
                value: (*value).to_string(),
                description: (!description.is_empty()).then(|| (*description).to_string()),
                ..Default::default()
            })
            .collect(),
    )
}

pub(crate) fn run_chart(
    geom: &str,
    _plugin: &ChartonPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, LabeledError> {
    let span = call.head;

    let geom = geom.to_string();
    let cfg = Config::load(engine)
        .map_err(|e| LabeledError::new("Invalid charton config").with_label(e, span))?;

    let x: Option<String> = call.get_flag("x")?;
    let y: Option<String> = call.get_flag("y")?;
    let z: Option<String> = call.get_flag("z")?;
    let color: Option<String> = call.get_flag("color")?;
    let y2: Option<String> = call.get_flag("y2")?;
    let text: Option<String> = call.get_flag("text")?;
    let path_group: Option<String> = call.get_flag("path-group")?;
    let transforms = parse_transforms(call.get_flag::<Value>("transform")?, span)?;
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
        cell_padding: call.get_flag("cell-padding")?,
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
    let x_expand = parse_expansion(call.get_flag::<Value>("x-expand")?, "--x-expand", span)?;
    let y_expand = parse_expansion(call.get_flag::<Value>("y-expand")?, "--y-expand", span)?;
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
    // Every chart font is multiplied by this. Inline charts are fitted into
    // the terminal, so charton's base sizes come out small; 1.5 reads like the
    // shell's own text. Flag > config > default.
    let font_scale = call
        .get_flag::<f64>("font-scale")?
        .or(cfg.font_scale)
        .filter(|s| *s > 0.0)
        .unwrap_or(DEFAULT_FONT_SCALE);
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
        path_group: path_group.clone(),
        transforms: transforms.clone(),
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
        font_scale,
        span,
    };
    let chart = build_chart(&values, &opts, engine)?;

    // 4. Decide how to deliver the result.
    let can_inline = !raw
        && !no_inline
        && !engine.is_using_stdio()
        && (std::io::stdout().is_terminal() || force_inline);

    // 4a. Save to file if requested (works alongside inline display).
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

    // 4b. Inline terminal rendering. Only safe when stdout is the terminal
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
            let fitted = render::inline_scale(width, height, max_cols, max_rows, cell_w, cell_h);
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
                let (c, r) = render::fit_cells_with_cell(w, h, max_cols, max_rows, cell_w, cell_h);
                render::iterm2_image(&bytes, c, r)
            }
            render::InlineStyle::Kitty => {
                let (w, h) = render::png_dimensions(&bytes).map_err(|e| {
                    LabeledError::new("Inline rendering failed").with_label(e, span)
                })?;
                let (c, r) = render::fit_cells_with_cell(w, h, max_cols, max_rows, cell_w, cell_h);
                render::kitty_image(&bytes, c, r)
            }
            render::InlineStyle::Sixel => {
                render::sixel_image(&bytes, max_cols, max_rows, cell_w, cell_h)
                    .map_err(|e| LabeledError::new("Inline rendering failed").with_label(e, span))?
            }
            // `Auto` already resolved above; treat as the universal fallback.
            render::InlineStyle::HalfBlock | render::InlineStyle::Auto => {
                render::png_to_halfblock(&bytes, max_cols, max_rows)
                    .map_err(|e| LabeledError::new("Inline rendering failed").with_label(e, span))?
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

    // 4c. Raw data for further piping.
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

    // 4d. Saved already? return nothing, else hand back SVG text.
    if output.is_some() {
        Ok(Value::nothing(span).into_pipeline_data())
    } else {
        let svg = render::to_svg(&chart)
            .map_err(|e| LabeledError::new("SVG rendering failed").with_label(e, span))?;
        Ok(Value::string(svg, span).into_pipeline_data())
    }
}

/// One chart layer: the mark to draw and the columns that feed it.
///
/// Only `geom` is required. When a `--layer` overlay omits a field, it falls
/// back to the primary layer, so an overlay only states what it changes.
#[derive(Clone)]
struct LayerOpts {
    geom: String,
    x: Option<String>,
    y: Option<String>,
    /// Scalar field for `contour`.
    z: Option<String>,
    y2: Option<String>,
    color: Option<String>,
    text: Option<String>,
    /// Column grouping rows into one `path`/`polygon`.
    path_group: Option<String>,
    /// Ordered transforms applied to the table before the mark runs. Each one
    /// replaces the table, exactly like charton's public `transform_*` steps.
    transforms: Vec<TransformSpec>,
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
    /// `--x-expand`: see [`parse_expansion`].
    x_expand: Option<Expansion>,
    y_expand: Option<Expansion>,
    density_bandwidth: Option<f64>,
    density_kernel: Option<KernelType>,
    density_cumulative: bool,
    density_counts: bool,
}

/// One step of a `--transform` chain: a Nushell record mapped onto a charton
/// public transform. The vocabulary is closed — this is the whole list.
#[derive(Clone)]
enum TransformSpec {
    Density(DensityTransform),
    Density2D(Density2DTransform),
    Contour(ContourTransform),
    /// `transform_calculate`: add a column from a Nushell closure run per row.
    Calculate {
        as_: String,
        expr: Value,
    },
    /// `center` is the categorical column the band sits on. It is kept here so
    /// the encoder can label the x axis with it (`with_category_labels`), which
    /// is what lets a hand-written violin/ridge share a discrete scale with points.
    Band {
        transform: BandTransform,
        center: Option<String>,
    },
    QuantileBox {
        transform: QuantileBoxTransform,
        category: Option<String>,
    },
    Window(WindowTransform),
}

/// The whole chart: every layer, plus the options that are shared across them
/// (title, theme, facets, coordinate system, size) rather than belonging to one
/// mark.
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
    font_scale: f64,
    span: Span,
}

/// Mark-level visual overrides for one layer.
///
/// Every field is optional: `None` (or `false` for `loess`) keeps the mark's own
/// default. The primary layer fills these from its CLI flags; a `--layer` may
/// carry a `style` record that overrides them for that layer only. A flag a mark
/// does not declare is rejected by the CLI parser, and an unknown key in a
/// `style` record is rejected by [`parse_style_record`].
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
    cell_padding: Option<f64>,
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
    if let Some(p) = s.cell_padding {
        m = m.with_cell_padding(p);
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
        .with_label(format!("`charton {geom}` requires {what}"), span)
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

/// Check every column named by the layers against the pipeline table, so a typo
/// reports the available columns instead of a low-level charton error.
fn validate_columns(table: &Table, opts: &BuildOpts<'_>) -> Result<(), LabeledError> {
    let span = opts.span;
    let available: Vec<&str> = table
        .columns
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    let check = |col: &Option<String>, flag: &str| -> Result<(), LabeledError> {
        let Some(name) = col else { return Ok(()) };
        if !available.contains(&name.as_str()) {
            return Err(LabeledError::new("Unknown column").with_label(
                format!(
                    "`{flag}` refers to column '{name}', but the table only has: {}",
                    available.join(", ")
                ),
                span,
            ));
        }
        Ok(())
    };
    let mut layers: Vec<&LayerOpts> = vec![&opts.primary];
    layers.extend(opts.layers.iter());
    for layer in layers {
        // A transform chain rewrites the table, so its output columns
        // (density, x, path_group, ...) are not in the pipeline table yet.
        if !layer.transforms.is_empty() {
            continue;
        }
        check(&layer.x, "--x")?;
        check(&layer.y, "--y")?;
        check(&layer.z, "--z")?;
        check(&layer.y2, "--y2")?;
        check(&layer.path_group, "--path-group")?;
        check(&layer.color, "--color")?;
        check(&layer.text, "--text")?;
        check(&layer.size_by, "--size-by")?;
        check(&layer.shape_by, "--shape-by")?;
    }
    Ok(())
}

fn build_chart(
    values: &[Value],
    opts: &BuildOpts<'_>,
    engine: &EngineInterface,
) -> Result<LayeredChart, LabeledError> {
    let span = opts.span;

    // --- Geographic marks read from a GeoJSON file, not the pipeline. ---
    if matches!(opts.primary.geom.as_str(), "geo" | "geoshape") {
        if !opts.layers.is_empty() {
            return Err(LabeledError::new("Layers are not supported for geo charts")
                .with_label("remove `--layer` when using `charton geo`", span));
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
    validate_columns(&table, opts)?;
    let dataset = table.to_dataset()?;

    let mut layered = build_layer(dataset.clone(), &opts.primary, opts.coord, span, engine)?;
    for extra in &opts.layers {
        layered = layered.and(build_layer(
            dataset.clone(),
            extra,
            opts.coord,
            span,
            engine,
        )?);
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
    engine: &EngineInterface,
) -> Result<LayeredChart, LabeledError> {
    let geom = layer.geom.as_str();
    // `path`/`polygon` (and anything with an explicit `--transform` chain) are
    // drawn from the grammar primitives: apply the transforms, then the mark.
    if !layer.transforms.is_empty() || matches!(geom, "path" | "polygon") {
        return build_primitive_layer(dataset, layer, span, engine);
    }
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
        if let Some(exp) = x_expand {
            e = e.with_expansion(exp);
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
        if let Some(exp) = y_expand {
            e = e.with_expansion(exp);
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
            } else if let Some(y2col) = y2 {
                // Floating bar: the rectangle spans y to y2 (candlestick body,
                // waterfall step). See charton's `mark_bar`.
                let x = x.ok_or_else(|| missing(geom, "--x", span))?;
                let y = y.ok_or_else(|| missing(geom, "--y", span))?;
                let (xe, ye) = (make_x(x), make_y(y));
                match color {
                    Some(col) => c.encode((xe, ye, alt::y2(y2col), alt::color(col))),
                    None => c.encode((xe, ye, alt::y2(y2col))),
                }
                .map_err(|e| chart_err(span, e))?
                .into()
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

            // A violin is a density outline plus an inner inter-quartile box.
            // The outline is built here from `transform_density` (grouped by
            // category, optionally also by the colour group) and
            // `transform_band`; `--color` dodges one violin per group, without
            // it there is one violin per category.
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
        "density_2d" | "density-2d" | "density2d" | "density-heatmap" => {
            let x = x.ok_or_else(|| missing(geom, "--x", span))?;
            let y = y.ok_or_else(|| missing(geom, "--y", span))?;
            // A 2D kernel density drawn as a grid of cells. `--bins` is the
            // grid size per axis; the rect bin count is set to match, so each
            // cell holds exactly one grid node and nothing is merged.
            let grid = bins.unwrap_or(50);
            let mut density = Density2DTransform::new(x, y).with_grid_size(grid);
            if let Some(bw) = layer.density_bandwidth {
                density = density.with_bandwidth(BandwidthType::Fixed(bw));
            }
            let c = Chart::build(dataset)
                .map_err(|e| chart_err(span, e))?
                .transform_density_2d(density)
                .map_err(|e| chart_err(span, e))?
                .mark_rect()
                .map_err(|e| chart_err(span, e))?
                .configure_rect(|m| style_rect(m, style));
            c.encode((
                alt::x("x").with_bins(grid),
                alt::y("y").with_bins(grid),
                alt::color("density"),
            ))
            .map_err(|e| chart_err(span, e))?
            .with_x_label(x)
            .with_y_label(y)
        }
        other => {
            return Err(LabeledError::new("Unknown geom").with_label(
                format!(
                    "'{other}' is not a mark; try point, line, path, polygon, area, bar, \
                     boxplot, violin, errorbar, rule, tick, text, rect, hist, density, \
                     density_2d, contour, or geo"
                ),
                span,
            ));
        }
    };

    Ok(layered)
}

/// Draw a layer through the grammar primitives: apply the `--transform` chain
/// (each step replaces the table), then draw a Tier-1 mark from the columns the
/// transforms emitted. `path` and `polygon` also take this route because they
/// read a `--path-group` column.
fn build_primitive_layer(
    dataset: Dataset,
    layer: &LayerOpts,
    span: Span,
    engine: &EngineInterface,
) -> Result<LayeredChart, LabeledError> {
    let geom = layer.geom.as_str();
    let style = &layer.style;

    // 1. stat: each transform replaces the table, like charton's `transform_*`.
    let mut chart = Chart::build(dataset).map_err(|e| chart_err(span, e))?;
    for t in &layer.transforms {
        chart = match t {
            TransformSpec::Density(d) => chart.transform_density(d.clone()),
            TransformSpec::Density2D(d) => chart.transform_density_2d(d.clone()),
            TransformSpec::Contour(d) => chart.transform_contour(d.clone()),
            TransformSpec::Band { transform, .. } => chart.transform_band(transform.clone()),
            TransformSpec::QuantileBox { transform, .. } => {
                chart.transform_quantile_box(transform.clone())
            }
            TransformSpec::Window(d) => chart.transform_window(d.clone()),
            TransformSpec::Calculate { as_, expr } => chart
                .transform_calculate(as_.as_str(), |row| calculate_row(engine, expr, &row, span)),
        }
        .map_err(|e| chart_err(span, e))?;
    }

    // 2. encode: map the emitted columns to channels, then draw the mark.
    let x = layer
        .x
        .as_deref()
        .ok_or_else(|| missing(geom, "--x", span))?;
    let y = layer
        .y
        .as_deref()
        .ok_or_else(|| missing(geom, "--y", span))?;
    // A band or quantile-box places its shape on a categorical axis. Label the
    // x axis with that column (as charton's hand-written violin does) so the
    // layer shares a discrete scale with points and boxes.
    let category_label = layer.transforms.iter().find_map(|t| match t {
        TransformSpec::Band { center, .. } => center.as_deref(),
        TransformSpec::QuantileBox { category, .. } => category.as_deref(),
        _ => None,
    });
    let encoders = || {
        let mut xe = match layer.x_scale {
            Some(s) => alt::x(x).with_scale(s),
            None => alt::x(x),
        };
        if let Some(label) = category_label {
            xe = xe.with_category_labels(label);
        }
        if let Some(exp) = layer.x_expand {
            xe = xe.with_expansion(exp);
        }
        let mut ye = match layer.y_scale {
            Some(s) => alt::y(y).with_scale(s),
            None => alt::y(y),
        };
        if let Some(mode) = layer.stack.as_deref() {
            ye = ye.with_stack(mode);
        }
        if layer.normalize {
            ye = ye.with_normalize(true);
        }
        if let Some(op) = layer.aggregate.as_deref() {
            ye = ye.with_aggregate(op);
        }
        if let Some(exp) = layer.y_expand {
            ye = ye.with_expansion(exp);
        }
        (xe, ye)
    };

    // x plus whichever of path-group/color the layer uses.
    macro_rules! encode_xy {
        ($c:expr) => {{
            let (xe, ye) = encoders();
            let r = match (layer.path_group.as_deref(), layer.color.as_deref()) {
                (Some(pg), Some(c)) => $c.encode((xe, ye, alt::path_group(pg), alt::color(c))),
                (Some(pg), None) => $c.encode((xe, ye, alt::path_group(pg))),
                (None, Some(c)) => $c.encode((xe, ye, alt::color(c))),
                (None, None) => $c.encode((xe, ye)),
            };
            r.map_err(|e| chart_err(span, e))?
        }};
    }

    let layered: LayeredChart = match geom {
        "point" => {
            let c = chart
                .mark_point()
                .map_err(|e| chart_err(span, e))?
                .configure_point(|m| style_point(m, style));
            encode_xy!(c).into()
        }
        "line" => {
            let c = chart
                .mark_line()
                .map_err(|e| chart_err(span, e))?
                .configure_line(|m| style_line(m, style));
            encode_xy!(c).into()
        }
        "path" => {
            let c = chart
                .mark_path()
                .map_err(|e| chart_err(span, e))?
                .configure_path(|m| style_polygon(m, style));
            encode_xy!(c).into()
        }
        "polygon" => {
            let c = chart
                .mark_polygon()
                .map_err(|e| chart_err(span, e))?
                .configure_geoshape(|m| style_polygon(m, style));
            encode_xy!(c).into()
        }
        "bar" => {
            let c = chart
                .mark_bar()
                .map_err(|e| chart_err(span, e))?
                .configure_bar(|m| style_bar(m, style));
            encode_xy!(c).into()
        }
        "area" => {
            let c = chart
                .mark_area()
                .map_err(|e| chart_err(span, e))?
                .configure_area(|m| style_area(m, style));
            encode_xy!(c).into()
        }
        "rect" => {
            let c = chart
                .mark_rect()
                .map_err(|e| chart_err(span, e))?
                .configure_rect(|m| style_rect(m, style));
            encode_xy!(c).into()
        }
        "tick" => {
            let c = chart
                .mark_tick()
                .map_err(|e| chart_err(span, e))?
                .configure_tick(|m| style_tick(m, style));
            encode_xy!(c).into()
        }
        "rule" => {
            let c = chart
                .mark_rule()
                .map_err(|e| chart_err(span, e))?
                .configure_rule(|m| style_rule(m, style));
            let (xe, ye) = encoders();
            match (layer.y2.as_deref(), layer.color.as_deref()) {
                (Some(h), Some(col)) => c.encode((xe, ye, alt::y2(h), alt::color(col))),
                (Some(h), None) => c.encode((xe, ye, alt::y2(h))),
                (None, Some(col)) => c.encode((xe, ye, alt::color(col))),
                (None, None) => c.encode((xe, ye)),
            }
            .map_err(|e| chart_err(span, e))?
            .into()
        }
        "text" => {
            let label = layer
                .text
                .as_deref()
                .ok_or_else(|| missing(geom, "--text <column>", span))?;
            let c = chart
                .mark_text()
                .map_err(|e| chart_err(span, e))?
                .configure_text(|m| style_text(m, style));
            let (xe, ye) = encoders();
            match layer.color.as_deref() {
                Some(col) => c.encode((xe, ye, alt::text(label), alt::color(col))),
                None => c.encode((xe, ye, alt::text(label))),
            }
            .map_err(|e| chart_err(span, e))?
            .into()
        }
        other => {
            return Err(
                LabeledError::new("Unsupported mark for --transform").with_label(
                    format!("`{other}` cannot be combined with --transform; use a Tier-1 mark"),
                    span,
                ),
            );
        }
    };
    Ok(layered)
}

/// Evaluate a `calculate` closure against one charton row, returning its number.
///
/// The row is handed to the closure as a Nushell record (every column name to
/// its value), so `{|r| ... }` can read any column produced by an earlier
/// transform. Non-numeric or errored results become `None` (a null cell).
fn calculate_row(
    engine: &EngineInterface,
    expr: &Value,
    row: &RowAccessor<'_>,
    span: Span,
) -> Option<f64> {
    let Value::Closure { val, .. } = expr else {
        return None;
    };
    let mut record = Record::new();
    for (name, value) in row.entries() {
        record.push(name, any_to_value(&value, span));
    }
    let closure = Spanned {
        item: (**val).clone(),
        span: expr.span(),
    };
    let out = engine
        .eval_closure(&closure, vec![Value::record(record, span)], None)
        .ok()?;
    out.as_float()
        .ok()
        .or_else(|| out.as_int().ok().map(|i| i as f64))
}

/// Convert one charton cell into the closest Nushell value.
fn any_to_value(value: &AnyValue<'_>, span: Span) -> Value {
    match value {
        AnyValue::Null => Value::nothing(span),
        AnyValue::Boolean(b) => Value::bool(*b, span),
        AnyValue::Int8(i) => Value::int(*i as i64, span),
        AnyValue::Int16(i) => Value::int(*i as i64, span),
        AnyValue::Int32(i) => Value::int(*i as i64, span),
        AnyValue::Int64(i) => Value::int(*i, span),
        AnyValue::UInt32(u) => Value::int(*u as i64, span),
        AnyValue::UInt64(u) => Value::int(*u as i64, span),
        AnyValue::Float32(f) => Value::float(*f as f64, span),
        AnyValue::Float64(f) => Value::float(*f, span),
        AnyValue::String(s) => Value::string(*s, span),
        AnyValue::Date(d) => Value::int(*d as i64, span),
        AnyValue::Datetime(ts, _) => Value::int(*ts, span),
        AnyValue::Duration(d) => Value::int(*d, span),
        AnyValue::Time(t) => Value::int(*t, span),
    }
}

fn bad_transform(span: Span, msg: String) -> LabeledError {
    LabeledError::new("Invalid transform").with_label(msg, span)
}

/// Parse `--transform`: one record, or a list of records applied in order.
fn parse_transforms(value: Option<Value>, span: Span) -> Result<Vec<TransformSpec>, LabeledError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = match value {
        Value::List { vals, .. } => vals.into_owned(),
        other => vec![other],
    };
    items.iter().map(|v| parse_transform(v, span)).collect()
}

fn parse_transform(value: &Value, span: Span) -> Result<TransformSpec, LabeledError> {
    let Value::Record { val, .. } = value else {
        return Err(bad_transform(
            span,
            format!("expected a record, got {}", value.get_type()),
        ));
    };
    let sf = |k: &str| val.get(k).and_then(|v| v.as_str().ok().map(str::to_string));
    let list = |k: &str| -> Vec<String> {
        val.get(k)
            .and_then(|v| match v {
                Value::List { vals, .. } => Some(
                    vals.iter()
                        .filter_map(|x| x.as_str().ok().map(str::to_string))
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default()
    };
    let num = |k: &str| {
        val.get(k).and_then(|v| {
            v.as_float()
                .ok()
                .or_else(|| v.as_int().ok().map(|i| i as f64))
        })
    };
    let boolean = |k: &str| val.get(k).and_then(|v| v.as_bool().ok());
    let op = sf("op").ok_or_else(|| bad_transform(span, "transform is missing `op`".into()))?;
    let norm = op.to_ascii_lowercase().replace(['-', '_'], "");

    match norm.as_str() {
        "density" | "kde" => {
            let field =
                sf("field").ok_or_else(|| bad_transform(span, "density needs `field`".into()))?;
            let as_ = list("as");
            let ve = as_.first().cloned().unwrap_or_else(|| field.clone());
            let de = as_.get(1).cloned().unwrap_or_else(|| "density".into());
            let mut d = DensityTransform::new(field).with_as(ve, de);
            let groups = list("group-by");
            if !groups.is_empty() {
                d = d.with_groupbys(groups);
            }
            if let Some(bw) = num("bandwidth") {
                d = d.with_bandwidth(BandwidthType::Fixed(bw));
            }
            if let Some(k) = sf("kernel") {
                d = d.with_kernel(parse_kernel(&k, span)?);
            }
            if let Some(c) = boolean("cumulative") {
                d = d.with_cumulative(c);
            }
            if let Some(c) = boolean("counts") {
                d = d.with_counts(c);
            }
            if let Some(t) = boolean("trim") {
                d = d.with_trim(t);
            }
            Ok(TransformSpec::Density(d))
        }
        "density2d" => {
            let x = sf("x").ok_or_else(|| bad_transform(span, "density-2d needs `x`".into()))?;
            let y = sf("y").ok_or_else(|| bad_transform(span, "density-2d needs `y`".into()))?;
            let as_ = list("as");
            let mut d = Density2DTransform::new(x, y);
            if as_.len() >= 3 {
                d = d.with_as(as_[0].clone(), as_[1].clone(), as_[2].clone());
            }
            if let Some(g) = val.get("grid-size").and_then(|v| v.as_int().ok()) {
                d = d.with_grid_size(g.max(1) as usize);
            }
            if let Some(bw) = num("bandwidth") {
                d = d.with_bandwidth(BandwidthType::Fixed(bw));
            }
            if let Some(p) = num("padding") {
                d = d.with_padding(p);
            }
            Ok(TransformSpec::Density2D(d))
        }
        "contour" => {
            let x = sf("x").ok_or_else(|| bad_transform(span, "contour needs `x`".into()))?;
            let y = sf("y").ok_or_else(|| bad_transform(span, "contour needs `y`".into()))?;
            let z = sf("z").ok_or_else(|| bad_transform(span, "contour needs `z`".into()))?;
            let as_ = list("as");
            let mut d = ContourTransform::new(x, y, z);
            if as_.len() >= 3 {
                d = d.with_as(as_[0].clone(), as_[1].clone(), as_[2].clone());
            }
            if let Some(level_as) = sf("level-as") {
                d = d.with_level_as(level_as);
            }
            if let Some(n) = val.get("levels").and_then(|v| v.as_int().ok()) {
                d = d.with_levels(n.max(1) as usize);
            } else if let Some(vals) = val.get("levels").and_then(|v| match v {
                Value::List { vals, .. } => {
                    let out: Vec<f64> = vals.iter().filter_map(|x| x.as_float().ok()).collect();
                    (!out.is_empty()).then_some(out)
                }
                _ => None,
            }) {
                d = d.with_levels_values(vals);
            }
            Ok(TransformSpec::Contour(d))
        }
        "band" => {
            let value =
                sf("value").ok_or_else(|| bad_transform(span, "band needs `value`".into()))?;
            let width = sf("width")
                .ok_or_else(|| bad_transform(span, "band needs `width` (the column)".into()))?;
            let as_ = list("as");
            let mut d = BandTransform::new(value, width);
            if as_.len() >= 3 {
                d = d.with_as(as_[0].clone(), as_[1].clone(), as_[2].clone());
            }
            let center = sf("center");
            if let Some(c) = &center {
                d = d.with_center(c.clone());
            }
            let group = sf("group");
            if let Some(g) = &group {
                d = d.with_group(g.clone());
            }
            if let Some(side) = sf("side") {
                d = d.with_side(parse_band_side(&side));
            }
            if let Some(scale) = sf("scale") {
                d = d.with_scale(BandScale::from(scale.as_str()));
            }
            if let Some(w) = num("band-width") {
                d = d.with_width(w);
            }
            if let Some(o) = num("overlap") {
                d = d.with_overlap(o);
            }
            if let Some(s) = num("span") {
                d = d.with_span(s);
            }
            if let Some(sp) = boolean("split") {
                d = d.with_split(sp);
            }
            if sf("position").as_deref() == Some("dodge") {
                d = d.with_position(Position::dodge());
            }
            Ok(TransformSpec::Band {
                transform: d,
                center,
            })
        }
        "quantilebox" | "box" => {
            let value = sf("value")
                .ok_or_else(|| bad_transform(span, "quantile-box needs `value`".into()))?;
            let as_ = list("as");
            let mut d = QuantileBoxTransform::new(value);
            let category = sf("category");
            if let Some(c) = &category {
                d = d.with_category(c.clone());
            }
            let group = sf("group");
            if let Some(g) = &group {
                d = d.with_group(g.clone());
            }
            if as_.len() >= 3 {
                d = d.with_as(as_[0].clone(), as_[1].clone(), as_[2].clone());
            }
            if let Some(w) = num("width") {
                d = d.with_width(w);
            }
            if let Some(w) = num("box-width") {
                d = d.with_box_width(w);
            }
            if let Some(s) = num("span") {
                d = d.with_span(s);
            }
            if sf("position").as_deref() == Some("dodge") {
                d = d.with_position(Position::dodge());
            }
            Ok(TransformSpec::QuantileBox {
                transform: d,
                category,
            })
        }
        "window" => {
            let field =
                sf("field").ok_or_else(|| bad_transform(span, "window needs `field`".into()))?;
            let kind = sf("kind").unwrap_or_else(|| "cume-dist".into());
            let op = parse_window_op(&kind, span)?;
            let as_ = sf("as").unwrap_or_else(|| kind.clone());
            let mut d = WindowTransform::new(WindowFieldDef::new(&field, op, as_));
            let groups = list("group-by");
            if !groups.is_empty() {
                d = d.with_groupbys(groups);
            }
            if let Some(n) = boolean("normalize") {
                d = d.with_normalize(n);
            }
            Ok(TransformSpec::Window(d))
        }
        "calculate" | "calc" => {
            let as_ = sf("as").ok_or_else(|| bad_transform(span, "calculate needs `as`".into()))?;
            let expr = val
                .get("expr")
                .cloned()
                .ok_or_else(|| bad_transform(span, "calculate needs `expr` (a closure)".into()))?;
            if !matches!(expr, Value::Closure { .. }) {
                return Err(bad_transform(
                    span,
                    "calculate `expr` must be a closure, e.g. {|row| $row.a + $row.b}".into(),
                ));
            }
            Ok(TransformSpec::Calculate { as_, expr })
        }
        other => Err(bad_transform(
            span,
            format!(
                "unknown transform `{other}`; try density, density-2d, contour, band, \
                 quantile-box, window, or calculate"
            ),
        )),
    }
}

fn parse_window_op(spec: &str, span: Span) -> Result<WindowOnlyOp, LabeledError> {
    Ok(
        match spec.to_ascii_lowercase().replace(['-', '_'], "").as_str() {
            "rownumber" | "row" => WindowOnlyOp::RowNumber,
            "rank" => WindowOnlyOp::Rank,
            "denserank" => WindowOnlyOp::DenseRank,
            "percentrank" => WindowOnlyOp::PercentRank,
            "cumedist" | "ecdf" => WindowOnlyOp::CumeDist,
            "cumulativesum" | "cumsum" => WindowOnlyOp::CumulativeSum,
            other => {
                return Err(bad_transform(
                    span,
                    format!("unknown window kind `{other}`"),
                ));
            }
        },
    )
}

fn parse_band_side(spec: &str) -> BandSide {
    match spec.to_ascii_lowercase().as_str() {
        "left" => BandSide::Left,
        "right" => BandSide::Right,
        _ => BandSide::Both,
    }
}

/// Parse the `--layer` flag into extra layers. An item is either a mark name
/// (shorthand for `{geom: <name>}`) or a record; any field it omits — `x`, `y`,
/// `y2`, `color`, `text`, `path_group`, `transforms`, ... — falls back to the
/// primary layer.
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
        // `--layer point` is shorthand for `{geom: point}`: the layer inherits
        // every encoding it does not state from the primary layer.
        if let Value::String { val: geom_name, .. } = &record {
            out.push(LayerOpts {
                geom: geom_name.to_lowercase(),
                ..primary.clone()
            });
            continue;
        }
        let Value::Record { val, .. } = &record else {
            return Err(LabeledError::new("Invalid layer").with_label(
                format!("layer {i} must be a geom name or a record"),
                layer_span,
            ));
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
                parse_style_record(style_rec, &primary.style)?
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
            path_group: field("path_group")
                .or_else(|| field("path-group"))
                .or_else(|| primary.path_group.clone()),
            transforms: match val.get("transforms").or_else(|| val.get("transform")) {
                Some(v) => parse_transforms(Some(v.clone()), layer_span)?,
                None => primary.transforms.clone(),
            },
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

/// Parse `--coord`: `cartesian` (the default) or `polar`. Geographic charts use
/// the `geo` mark (`charton geo`), not this flag.
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

/// Parse `--x-expand` / `--y-expand`.
///
/// A single number pads both ends by that fraction of the data range (the
/// historical behavior). A `[low, high]` list instead pads the low and high
/// ends by that many data units, which is how a one-sided mark gets headroom on
/// the side it grows toward (a ridgeline's half-band, for example).
fn parse_expansion(
    value: Option<Value>,
    flag: &str,
    span: Span,
) -> Result<Option<Expansion>, LabeledError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let number = |v: &Value| {
        v.as_float()
            .ok()
            .or_else(|| v.as_int().ok().map(|i| i as f64))
    };
    let bad = || {
        LabeledError::new("Invalid axis expansion").with_label(
            format!(
                "{flag} expects a number or a [low, high] pair, got {}",
                value.get_type()
            ),
            span,
        )
    };
    match &value {
        Value::Int { .. } | Value::Float { .. } => {
            let v = number(&value).ok_or_else(bad)?;
            Ok(Some(Expansion {
                mult: (v, v),
                add: (0.0, 0.0),
            }))
        }
        Value::List { vals, .. } if vals.len() == 2 => {
            let lo = number(&vals[0]).ok_or_else(bad)?;
            let hi = number(&vals[1]).ok_or_else(bad)?;
            Ok(Some(Expansion {
                mult: (0.0, 0.0),
                add: (lo, hi),
            }))
        }
        _ => Err(bad()),
    }
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

/// Canonical keys of a `--layer` `style` record, in kebab-case. Snake-case
/// spellings are accepted too; the list is shown when a key is not recognised.
const STYLE_KEYS: &[&str] = &[
    "color",
    "opacity",
    "size",
    "stroke",
    "stroke-width",
    "shape",
    "dash",
    "interpolation",
    "outlier-color",
    "anchor",
    "weight",
    "layout",
    "quasirandom-method",
    "mark-width",
    "cell-padding",
    "cap-length",
    "center",
    "outliers",
    "outlier-size",
    "loess",
    "loess-bandwidth",
];

/// Coerce a style value to a string, or name the type that was expected.
fn style_string(value: &Value) -> Result<String, &'static str> {
    value.as_str().map(str::to_string).map_err(|_| "a string")
}

/// Coerce a style value to a number (int or float), or name the expected type.
fn style_number(value: &Value) -> Result<f64, &'static str> {
    value
        .as_float()
        .ok()
        .or_else(|| value.as_int().ok().map(|i| i as f64))
        .ok_or("a number")
}

/// Coerce a style value to a boolean, or name the expected type.
fn style_bool(value: &Value) -> Result<bool, &'static str> {
    value.as_bool().map_err(|_| "a boolean")
}

/// Build a per-layer [`Style`] by overriding `base` with the fields present in
/// a `--layer` `style` record.
///
/// Keys may be kebab-case or snake_case; they mirror the per-mark flags, with
/// the two negative switches written as booleans: `center: false` is
/// `--no-center`, and `outliers: false` is `--no-outliers`. A wrong value type
/// or an unknown key is an error, so a typo surfaces instead of leaving the
/// field at its default.
fn parse_style_record(rec: &Record, base: &Style) -> Result<Style, LabeledError> {
    let mut s = base.clone();
    for (key, value) in rec.iter() {
        let key = key.replace('-', "_");
        let bad = |expected: &str| {
            LabeledError::new("Invalid layer style")
                .with_label(format!("`{key}` expects {expected}"), value.span())
        };
        match key.as_str() {
            "color" => s.color = Some(style_string(value).map_err(bad)?),
            "opacity" => s.opacity = Some(style_number(value).map_err(bad)?),
            "size" => s.size = Some(style_number(value).map_err(bad)?),
            "stroke" => s.stroke = Some(style_string(value).map_err(bad)?),
            "stroke_width" => s.stroke_width = Some(style_number(value).map_err(bad)?),
            "shape" => s.shape = Some(style_string(value).map_err(bad)?),
            "dash" => {
                let pattern = style_string(value).map_err(bad)?;
                s.dash = Some(parse_dash(&pattern, value.span())?);
            }
            "interpolation" => s.interpolation = Some(style_string(value).map_err(bad)?),
            "outlier_color" => s.outlier_color = Some(style_string(value).map_err(bad)?),
            "anchor" => s.anchor = Some(style_string(value).map_err(bad)?),
            "weight" => s.weight = Some(style_string(value).map_err(bad)?),
            "layout" => s.layout = Some(style_string(value).map_err(bad)?),
            "quasirandom_method" => s.quasirandom_method = Some(style_string(value).map_err(bad)?),
            "mark_width" => s.mark_width = Some(style_number(value).map_err(bad)?),
            "cell_padding" => s.cell_padding = Some(style_number(value).map_err(bad)?),
            "cap_length" => s.cap_length = Some(style_number(value).map_err(bad)?),
            "center" => s.center = Some(style_bool(value).map_err(bad)?),
            "outliers" | "show_outliers" => s.show_outliers = Some(style_bool(value).map_err(bad)?),
            "outlier_size" => s.outlier_size = Some(style_number(value).map_err(bad)?),
            "loess" => s.loess = style_bool(value).map_err(bad)?,
            "loess_bandwidth" => s.loess_bandwidth = Some(style_number(value).map_err(bad)?),
            other => {
                return Err(LabeledError::new("Unknown layer style key").with_label(
                    format!(
                        "`{other}` is not a style field; try one of: {}",
                        STYLE_KEYS.join(", ")
                    ),
                    value.span(),
                ));
            }
        }
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
        if (opts.font_scale - 1.0).abs() > f64::EPSILON {
            t = t.with_font_scale(opts.font_scale);
        }
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
        let out = run("charton bar -x species -y petal_length")?;
        assert!(out.as_str()?.trim_start().starts_with("<svg"));
        Ok(())
    }

    #[test]
    fn theme_dark_uses_dark_background() -> Result<(), ShellError> {
        let out = run("charton bar -x species -y petal_length --theme dark")?;
        // Catppuccin Mocha base #1E1E2E renders as rgba(30,30,46,1.000).
        assert!(out.as_str()?.contains("rgba(30,30,46,1.000)"));
        Ok(())
    }

    #[test]
    fn font_scale_changes_the_render() -> Result<(), ShellError> {
        let base = run("charton bar -x species -y petal_length")?;
        let big = run("charton bar -x species -y petal_length --font-scale 2.0")?;
        assert_ne!(base.as_str()?, big.as_str()?);
        Ok(())
    }

    #[test]
    fn raincloud_layers_share_one_scale() -> Result<(), ShellError> {
        // A violin atom, a quantile-box layer, and the raw points. The box
        // transform's `category` labels the x axis as discrete, so the three
        // layers line up instead of conflicting on linear vs discrete.
        let out = run("charton violin -x species -y petal_length \
             --layer {geom: polygon, x: qx, y: qy, path_group: qpg, \
                      transforms: [{op: quantile-box, value: petal_length, category: species, as: [qx, qy, qpg]}]} \
             --layer {geom: point, x: species, y: petal_length}")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn calculate_transform_adds_a_column() -> Result<(), ShellError> {
        // `calculate` runs a Nushell closure per row; the row is a record built
        // from the current (transformed) table.
        let out = run("charton bar -x species -y doubled \
             --transform [{op: calculate, as: doubled, expr: {|r| $r.petal_length * 2}}]")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn calculate_sees_earlier_transform_output() -> Result<(), ShellError> {
        // A window transform adds `total`, and `calculate` can read it.
        let out = run("charton bar -x species -y base --y2 total --transform [\
             {op: window, field: petal_length, kind: cumulative-sum, as: total}, \
             {op: calculate, as: base, expr: {|r| $r.total - $r.petal_length}}]")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn theme_light_uses_white_background() -> Result<(), ShellError> {
        let out = run("charton bar -x species -y petal_length --theme light")?;
        assert!(out.as_str()?.contains("rgba(255,255,255,1.000)"));
        Ok(())
    }

    #[test]
    fn unknown_theme_is_an_error() {
        assert!(run("charton bar -x species -y petal_length --theme blue").is_err());
    }

    #[test]
    fn hist_returns_svg() -> Result<(), ShellError> {
        let out = run("charton hist -x petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn beeswarm_layout_returns_svg() -> Result<(), ShellError> {
        // A beeswarm is `point` plus a layout parameter, not a mark.
        let out = run("charton point -x species -y petal_length --layout beeswarm")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn area_returns_svg() -> Result<(), ShellError> {
        let out = run("charton area -x t -y petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn boxplot_returns_svg() -> Result<(), ShellError> {
        let out = run("charton boxplot -x species -y petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn contour_returns_svg() -> Result<(), ShellError> {
        let out = run_on(grid(), "charton contour -x x -y y --z z")?;
        assert!(out.as_str()?.contains("<svg"));
        // `--bins` sets the number of iso-levels.
        let out = run_on(grid(), "charton contour -x x -y y --z z --bins 4")?;
        assert!(out.as_str()?.contains("<svg"));
        // `--stroke` gives a single-colour contour instead of a level colormap.
        let out = run_on(grid(), "charton contour -x x -y y --z z --stroke black")?;
        assert!(out.as_str()?.contains("<svg"));
        // Without `--z`, a 2D density is estimated first (density contour).
        let out = run_on(grid(), "charton contour -x x -y y")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn density_2d_returns_svg() -> Result<(), ShellError> {
        // A 2D kernel density drawn as a heatmap.
        let out = run_on(grid(), "charton density-2d -x x -y y")?;
        assert!(out.as_str()?.contains("<svg"));
        // `--bins` sets the grid size per axis.
        let out = run_on(grid(), "charton density-2d -x x -y y --bins 20")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn violin_returns_svg() -> Result<(), ShellError> {
        // One violin per category.
        let out = run("charton violin -x species -y petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        // Grouped (dodged) violins, one per `grp`.
        let out = run("charton violin -x species -y petal_length -c grp")?;
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
        let out = run_on(table, "charton violin -x x -y y")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn errorbar_aggregates_without_y2() -> Result<(), ShellError> {
        let out = run("charton errorbar -x species -y petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn errorbar_accepts_explicit_bounds() -> Result<(), ShellError> {
        let out = run("charton errorbar -x t -y lo --y2 hi")?;
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
        let out = run(&format!("charton geo --geojson '{arg}'"));
        let _ = std::fs::remove_file(&path);
        assert!(out?.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn facet_wrap_returns_svg() -> Result<(), ShellError> {
        let out = run("charton point -x t -y petal_length --facet-wrap species")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn facet_grid_returns_svg() -> Result<(), ShellError> {
        let out = run("charton point -x t -y petal_length --facet-row species --facet-col grp")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn incomplete_facet_grid_is_an_error() {
        assert!(run("charton point -x t -y petal_length --facet-row species").is_err());
    }

    #[test]
    fn rect_returns_svg() -> Result<(), ShellError> {
        let out = run("charton rect -x species -y grp -c petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn rule_returns_svg() -> Result<(), ShellError> {
        let out = run("charton rule -x t -y lo --y2 hi")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn text_returns_svg() -> Result<(), ShellError> {
        let out = run("charton text -x t -y petal_length --text species")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn text_requires_text_column() {
        assert!(run("charton text -x t -y petal_length").is_err());
    }

    #[test]
    fn tick_returns_svg() -> Result<(), ShellError> {
        let out = run("charton tick -x t -y petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn layer_overlay_returns_svg() -> Result<(), ShellError> {
        let out = run("charton line -x t -y petal_length --layer {geom: point}")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn layer_list_and_bad_layer() -> Result<(), ShellError> {
        let out =
            run("charton line -x t -y petal_length --layer [{geom: point} {geom: rule, y2: hi}]")?;
        assert!(out.as_str()?.contains("<svg"));
        assert!(run("charton line -x t -y petal_length --layer {x: t}").is_err());
        Ok(())
    }

    #[test]
    fn labels_appear_in_svg() -> Result<(), ShellError> {
        let out = run(
            "charton bar -x species -y petal_length --x-label FooAxis --y-label BarAxis --color-label ColAxis -c grp",
        )?;
        let svg = out.as_str()?;
        assert!(svg.contains("FooAxis") && svg.contains("BarAxis") && svg.contains("ColAxis"));
        Ok(())
    }

    #[test]
    fn style_and_flip_render() -> Result<(), ShellError> {
        let out = run(
            "charton point -x t -y petal_length --opacity 0.5 --size 8 --mark-color red --flip",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn incomplete_domain_is_an_error() {
        assert!(run("charton point -x t -y petal_length --x-min 0").is_err());
        assert!(run("charton point -x t -y petal_length --y-max 10").is_err());
    }

    #[test]
    fn full_domain_renders() -> Result<(), ShellError> {
        let out =
            run("charton point -x t -y petal_length --x-min 0 --x-max 10 --y-min 0 --y-max 10")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn missing_xy_is_an_error() {
        assert!(run("charton bar").is_err());
    }

    #[test]
    fn unknown_geom_is_an_error() {
        assert!(run("charton pie -x species -y petal_length").is_err());
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
        let out = run("charton point -x t -y petal_length --y-scale log")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn invalid_scale_is_an_error() {
        assert!(run("charton point -x t -y petal_length --y-scale nope").is_err());
    }

    #[test]
    fn color_map_renders_and_rejects_unknown() -> Result<(), ShellError> {
        let out = run("charton rect -x species -y grp -c petal_length --color-map magma")?;
        assert!(out.as_str()?.contains("<svg"));
        assert!(run("charton rect -x species -y grp -c petal_length --color-map nope").is_err());
        Ok(())
    }

    #[test]
    fn label_format_preset_and_record_render() -> Result<(), ShellError> {
        let out = run("charton bar -x species -y petal_length --y-format compact")?;
        assert!(out.as_str()?.contains("<svg"));
        let out =
            run("charton bar -x species -y petal_length --y-format {compact: true, prefix: '$'}")?;
        assert!(out.as_str()?.contains("<svg"));
        assert!(run("charton bar -x species -y petal_length --y-format nope").is_err());
        Ok(())
    }

    #[test]
    fn mark_options_render() -> Result<(), ShellError> {
        let out = run("charton point -x t -y petal_length --shape square")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton line -x t -y petal_length --dash 6,4 --interpolation step")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton boxplot -x species -y petal_length --outlier-color red")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn invalid_dash_is_an_error() {
        assert!(run("charton line -x t -y petal_length --dash abc").is_err());
    }

    #[test]
    fn layer_style_override_renders() -> Result<(), ShellError> {
        let out = run(
            "charton bar -x species -y petal_length --layer {geom: point, style: {color: red, size: 4}}",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn documented_transform_records_render() -> Result<(), ShellError> {
        // Spot-check the transform examples in the docs: each must render
        // using only the default output names (`as` omitted).
        let out = run(
            "charton area -x petal_length -y density --transform {op: density, field: petal_length}",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run_on(
            grid(),
            "charton rect -x x -y y -c density --transform {op: density-2d, x: x, y: y}",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run_on(
            grid(),
            "charton path -x x -y y --path-group path_group -c level \
             --transform {op: contour, x: x, y: y, z: z}",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run(
            "charton polygon -x x -y y --path-group path_group \
             --transform {op: quantile-box, value: petal_length, category: species}",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        // The book's two-step scatter → density-2d → contour pipeline: the
        // second step reads the first step's default `density` column.
        let out = run_on(
            grid(),
            "charton path -x x -y y --path-group path_group -c level --transform [\
             {op: density-2d, x: x, y: y}, {op: contour, x: x, y: y, z: density}]",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn layer_inherits_the_primary_transform_chain() -> Result<(), ShellError> {
        // A layer that states no `transforms` reuses the primary's chain, so
        // an overlay can be added with just `--layer point` even when the
        // primary rewrites the table (`petal_length` -> `value` / `density`).
        let out = run(
            "charton area -x value -y density \
             --transform {op: density, field: petal_length, as: [value, density]} --layer point",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        // A window keeps the other columns, so `--layer point` draws the same
        // ranks as the line (the bump recipe).
        let out = run(
            "charton line -x t -y rank -c grp \
             --transform {op: window, field: petal_length, kind: rank, as: rank, group-by: [grp]} \
             --layer point",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn layer_with_empty_transforms_reads_original_rows() -> Result<(), ShellError> {
        // `transforms: []` replaces the inherited chain, so this point layer
        // reads the original `t` / `petal_length` columns even though the
        // primary rewrote the table into `value` / `density`.
        let out = run(
            "charton area -x value -y density \
             --transform {op: density, field: petal_length, as: [value, density]} \
             --layer {geom: point, x: t, y: petal_length, transforms: []}",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn layer_style_record_maps_every_field() {
        // A field settable by a CLI flag must be settable from a layer `style`
        // record too, or the two parsing paths drift apart. This covers the
        // fields that are not simple `color`/`size` overrides.
        let rec = record! {
            "mark-width" => Value::test_float(0.5),
            "cell-padding" => Value::test_float(-1.0),
            "cap-length" => Value::test_float(4.0),
            "center" => Value::test_bool(false),
            "outliers" => Value::test_bool(false),
            "outlier-size" => Value::test_float(3.0),
            "loess" => Value::test_bool(true),
            "loess-bandwidth" => Value::test_float(0.4),
        };
        let s = parse_style_record(&rec, &Style::default()).unwrap();
        assert_eq!(s.mark_width, Some(0.5));
        assert_eq!(s.cell_padding, Some(-1.0));
        assert_eq!(s.cap_length, Some(4.0));
        assert_eq!(s.center, Some(false));
        assert_eq!(s.show_outliers, Some(false));
        assert_eq!(s.outlier_size, Some(3.0));
        assert!(s.loess);
        assert_eq!(s.loess_bandwidth, Some(0.4));
    }

    #[test]
    fn layer_style_record_takes_kebab_or_snake_case() {
        let rec = record! {
            "mark_width" => Value::test_float(0.25),
            "outlier-color" => Value::test_string("red"),
            "quasirandom_method" => Value::test_string("tukey"),
        };
        let s = parse_style_record(&rec, &Style::default()).unwrap();
        assert_eq!(s.mark_width, Some(0.25));
        assert_eq!(s.outlier_color.as_deref(), Some("red"));
        assert_eq!(s.quasirandom_method.as_deref(), Some("tukey"));
    }

    #[test]
    fn layer_style_record_rejects_typos_and_wrong_types() {
        // An unknown key or a value of the wrong type is an error, not a field
        // that is silently left at its default.
        let typo = record! { "colour" => Value::test_string("red") };
        assert!(parse_style_record(&typo, &Style::default()).is_err());
        let wrong_type = record! { "opacity" => Value::test_string("lots") };
        assert!(parse_style_record(&wrong_type, &Style::default()).is_err());
    }

    #[test]
    fn background_flag_overrides_theme() -> Result<(), ShellError> {
        let out =
            run("charton bar -x species -y petal_length --theme dark --background '#ff0000'")?;
        assert!(out.as_str()?.contains("rgba(255,0,0,1.000)"));
        Ok(())
    }

    #[test]
    fn polar_charts_render() -> Result<(), ShellError> {
        // Rose / Nightingale: `-x` maps to the angle (bar mark in polar).
        let out = run("charton bar -x species -y petal_length --coord polar")?;
        assert!(out.as_str()?.contains("<svg"));
        // Donut adds an inner radius.
        let out = run("charton bar -x species -y petal_length --coord polar --inner-radius 0.5")?;
        assert!(out.as_str()?.contains("<svg"));
        // A partial angular span (rose / nightingale style).
        let out = run(
            "charton bar -x species -y petal_length --coord polar --start-angle 0 --end-angle 270",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn polar_pie_and_donut_without_x() -> Result<(), ShellError> {
        // Omitting `-x` selects charton's pie mode: the y values become the
        // slices and the colour column separates them, so percentage labels
        // are drawn.
        let pie = run("charton bar -y petal_length -c species --coord polar")?;
        assert!(pie.as_str()?.contains('%'), "pie mode should draw % labels");
        let donut = run("charton bar -y petal_length -c species --coord polar --inner-radius 0.5")?;
        assert!(donut.as_str()?.contains("<svg"));
        // Outside polar coordinates, an omitted x is still an error.
        assert!(run("charton bar -y petal_length").is_err());
        Ok(())
    }

    #[test]
    fn invalid_coord_is_an_error() {
        assert!(run("charton bar -x species -y petal_length --coord spherical").is_err());
    }

    #[test]
    fn stacking_renders() -> Result<(), ShellError> {
        let out = run("charton bar -x species -y petal_length -c grp --stack stacked")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton area -x t -y petal_length -c grp --stack normalize")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton bar -x species -y petal_length -c grp --normalize")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn invalid_stack_is_an_error() {
        assert!(run("charton bar -x species -y petal_length -c grp --stack wobble").is_err());
    }

    #[test]
    fn mark_geometry_options_render() -> Result<(), ShellError> {
        let out = run("charton bar -x species -y petal_length --mark-width 0.5")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton errorbar -x t -y lo --y2 hi --cap-length 4 --no-center")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton boxplot -x species -y petal_length --no-outliers --outlier-size 3")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn size_and_shape_encoding_render() -> Result<(), ShellError> {
        let out =
            run("charton point -x t -y petal_length --size-by petal_length --size-label Weight")?;
        assert!(out.as_str()?.contains("<svg"));
        let out =
            run("charton point -x t -y petal_length --shape-by species --shape-label Species")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn unsupported_output_extension_is_an_error() {
        let path = std::env::temp_dir().join("charton_test_output.pdf");
        let arg = path.to_string_lossy().replace('\\', "/");
        let res = run(&format!(
            "charton bar -x species -y petal_length -o '{arg}'"
        ));
        let _ = std::fs::remove_file(&path);
        assert!(
            res.is_err(),
            "a .pdf output must be rejected, not written as SVG"
        );
    }

    #[test]
    fn density_and_ecdf_render() -> Result<(), ShellError> {
        let out = run("charton density -x petal_length")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton density -x petal_length -c species --cumulative")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton density -x petal_length --density-kernel uniform")?;
        assert!(out.as_str()?.contains("<svg"));
        // ECDF is the CumeDist window transform + a step line, not a mark.
        let out = run(
            "charton line -x petal_length -y ecdf -c species --interpolation step --transform {op: window, field: petal_length, kind: cume-dist, as: ecdf, group-by: [species]}",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn density_opacity_is_opt_in() -> Result<(), ShellError> {
        // No implicit transparency: like charton's KDE example, the caller asks
        // for a translucent fill explicitly.
        let out = run("charton density -x petal_length -c species")?;
        assert!(!out.as_str()?.contains("fill-opacity=\"0.500\""));
        // --opacity is applied as written.
        let out = run("charton density -x petal_length -c species --opacity 0.5")?;
        assert!(out.as_str()?.contains("fill-opacity=\"0.500\""));
        Ok(())
    }

    #[test]
    fn invalid_density_kernel_is_an_error() {
        assert!(run("charton density -x petal_length --density-kernel cosine").is_err());
    }

    #[test]
    fn aggregate_bins_and_loess_render() -> Result<(), ShellError> {
        let out = run("charton bar -x species -y petal_length --aggregate mean")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton hist -x petal_length --bins 5")?;
        assert!(out.as_str()?.contains("<svg"));
        let out = run("charton line -x t -y petal_length --loess --loess-bandwidth 0.5")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn invalid_aggregate_is_an_error() {
        assert!(run("charton bar -x species -y petal_length --aggregate mode").is_err());
    }

    #[test]
    fn layout_overrides_render() -> Result<(), ShellError> {
        let out = run(
            "charton point -x t -y petal_length --margins 0.1,0.1,0.1,0.1 --x-expand 0.2 --y-ticks [0 2 4 6]",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        assert!(run("charton point -x t -y petal_length --margins 1,2,3").is_err());
        assert!(run("charton point -x t -y petal_length --y-ticks a").is_err());
        Ok(())
    }

    #[test]
    fn axis_expansion_accepts_number_or_pair() {
        assert!(parse_expansion(None, "--x-expand", Span::test_data())
            .unwrap()
            .is_none());
        // A number stays a symmetric fraction of the range.
        let number = parse_expansion(
            Some(Value::test_float(0.1)),
            "--x-expand",
            Span::test_data(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(number.mult, (0.1, 0.1));
        assert_eq!(number.add, (0.0, 0.0));
        // A pair is an asymmetric additive padding, in axis units.
        let pair = parse_expansion(
            Some(Value::test_list(vec![
                Value::test_float(0.4),
                Value::test_float(1.5),
            ])),
            "--x-expand",
            Span::test_data(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(pair.mult, (0.0, 0.0));
        assert_eq!(pair.add, (0.4, 1.5));
        // A string, or a list that is not a pair, is rejected.
        assert!(
            parse_expansion(Some(Value::test_string("wide")), "--x-expand", Span::test_data())
                .is_err()
        );
        assert!(
            parse_expansion(
                Some(Value::test_list(vec![Value::test_float(1.0)])),
                "--x-expand",
                Span::test_data(),
            )
            .is_err()
        );
    }

    #[test]
    fn primitive_path_honours_axis_expansion() -> Result<(), ShellError> {
        // `--x-expand` must also reach the `--transform` / `path` / `polygon`
        // route, not just the direct mark route.
        let base =
            "charton point -x x -y y --transform {op: density, field: petal_length, as: [x, y]}";
        let expanded = "charton point -x x -y y --x-expand [0.4, 1.5] \
                        --transform {op: density, field: petal_length, as: [x, y]}";
        assert_ne!(run(base)?.as_str()?, run(expanded)?.as_str()?);
        Ok(())
    }

    #[test]
    fn waterfall_calculate_with_if_else_renders() -> Result<(), ShellError> {
        // The README's waterfall record: a `calculate` closure with an if/else
        // must parse (the record needs its own closing brace).
        let out = run(
            "charton bar -x species -y base --y2 total --transform [\
             {op: window, field: petal_length, kind: cumulative-sum, as: total}, \
             {op: calculate, as: base, expr: {|row| if $row.species == 'setosa' { 0 } else { $row.total - $row.petal_length }}}]",
        )?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    /// Tier-3 recipes are not marks: each is written here from the grammar
    /// primitives (`--transform` + `--layer`) and must render to SVG.
    #[test]
    fn compositions_render_svg() -> Result<(), ShellError> {
        for src in [
            // ridgeline: a one-sided band over a per-category density. The
            // `--x-expand` pair gives the top ridge headroom (books' recipe).
            "charton polygon -x x -y y --path-group pg -c species --flip --x-expand [0.4, 1.5] \
             --transform [{op: density, field: petal_length, as: [value, density], group-by: [species]}, \
              {op: band, value: value, width: density, center: species, side: right, scale: per-group, overlap: 2.5, band-width: 1.0, as: [x, y, pg]}]",
            // candlestick: a low--high rule wick plus a floating bar body.
            "charton rule -x species -y lo --y2 hi -c grp --layer {geom: bar, y: lo, y2: hi}",
            // dumbbell: one rule joined by a dot at each end.
            "charton rule -x species -y lo --y2 hi --layer [{geom: point, y: lo}, {geom: point, y: hi}]",
            // slope: a line with its points on top.
            "charton line -x t -y petal_length -c grp --layer point",
            // range: just a rule.
            "charton rule -x species -y lo --y2 hi",
        ] {
            let out = run(src)?;
            assert!(out.as_str()?.contains("<svg"), "no SVG for `{src}`");
        }
        Ok(())
    }

    #[test]
    fn removed_recipe_names_are_not_commands() {
        // The old convenience geoms are gone; the mark must be composed.
        for name in ["ridge", "candlestick", "waterfall", "ecdf", "beeswarm"] {
            assert!(run(&format!("charton {name} -x species -y petal_length")).is_err());
        }
    }

    #[test]
    fn layer_geom_name_shorthand_renders() -> Result<(), ShellError> {
        let out = run("charton line -x t -y petal_length --layer point")?;
        assert!(out.as_str()?.contains("<svg"));
        Ok(())
    }

    #[test]
    fn unknown_column_lists_available_columns() {
        let err = run("charton bar -x nope -y petal_length").unwrap_err();
        // The details live in the label, which `Display` does not include.
        let msg = format!("{err:?}");
        assert!(msg.contains("nope"), "{msg}");
        assert!(msg.contains("petal_length"), "{msg}");
    }

    #[test]
    fn completion_offers_flag_values() {
        use nu_protocol::engine::ArgType;
        let suggestions = complete_arg(ArgType::Flag(std::borrow::Cow::Borrowed("interpolation")))
            .expect("interpolation should complete");
        assert!(suggestions.iter().any(|s| s.value == "step"));
    }
}
