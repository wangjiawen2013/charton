//! Turning a validated spec into a chart.
//!
//! The only module that knows both the wire format and the typed builder.
//! Everything goes through `Chart` and `LayeredChart`, the same path a Rust user
//! takes, so one document cannot render two different ways.
//!
//! Lowering assumes the document is valid. It returns `Result` so a bug cannot
//! become a panic, but every failure here is `E_INTERNAL`: the fix is a missing
//! validation check plus a negative test, not a new error for the caller.
use crate::chart::Chart;
use crate::core::composite::LayeredChart;
use crate::core::conversion::IntoLayered;
use crate::core::data::Dataset;
use crate::encode::{
    Encoding, IntoEncoding, color::Color, path_group::PathGroup, shape::Shape, size::Size,
    text::Text, x::X, y::Y, y2::Y2,
};
use crate::error::ChartonError;
use crate::mark::point::PointLayout;
use crate::mark::{
    area::MarkArea, bar::MarkBar, boxplot::MarkBoxplot, contour::MarkContour, density::MarkDensity,
    density_2d::MarkDensity2D, errorbar::MarkErrorBar, histogram::MarkHist, line::MarkLine,
    path::MarkPath, point::MarkPoint, polygon::MarkPolygon, rect::MarkRect, rule::MarkRule,
    text::MarkText, tick::MarkTick, violin::MarkViolin,
};
use crate::scale::{Scale, ScaleDomain};
use crate::spec::error::{SpecError, codes};
use crate::spec::types::{
    AggregateSpec, BinSpec, DomainSpec, ExtensionMark, FieldDef, MarkType, ScaleTypeSpec, Spec,
    SpecMark, StackKind, StackSpec,
};

/// Extension marks this build can lower.
///
/// Kept next to the dispatch below, because adding an arm to that match is the
/// only thing that may extend this list. That keeps one property true at all
/// times: if the validator accepts a document, lowering can build it.
pub const SUPPORTED_EXTENSION_MARKS: &[ExtensionMark] = &[
    ExtensionMark::Polygon,
    ExtensionMark::Path,
    ExtensionMark::Violin,
    ExtensionMark::Density,
    ExtensionMark::Density2d,
    ExtensionMark::Contour,
    ExtensionMark::Beeswarm,
    ExtensionMark::Hist,
];

/// Turn a validated spec plus its data into a chart.
///
/// The caller owns the data: reading a file, a pipeline table or a request body
/// is the entry point's job, and this function never learns which it was.
///
/// # Errors
///
/// Only [`codes::INTERNAL`] — a spec that validation accepted but that could not
/// be built. See the module docs.
pub fn lower(spec: &Spec, data: Dataset) -> Result<LayeredChart, Vec<SpecError>> {
    build(spec, data).map_err(|error| vec![internal(error, spec)])
}

/// The shared build path, in core error terms.
fn build(spec: &Spec, data: Dataset) -> Result<LayeredChart, ChartonError> {
    // Transforms run on the rows before any mark statistic does, in the order
    // the document lists them. The very same function runs during validation,
    // so the two stages cannot disagree.
    let data = crate::spec::transforms::apply(spec, &data)
        .map_err(|errors| ChartonError::Data(errors[0].message.clone()))?;

    let mut chart = build_tree(spec, data, None)?;

    if let Some(facet) = spec.facet.as_ref() {
        chart = chart.facet(facet_spec(facet));
    }

    Ok(with_metadata(chart, spec))
}

/// Lower one level of the document tree.
///
/// A document is either one mark or a list of layers. Layers share the root's
/// data and encoding; a layer's own encoding wins channel by channel, which is
/// why the inherited encoding is passed down instead of being merged into the
/// document.
fn build_tree(
    spec: &Spec,
    data: Dataset,
    inherited: Option<&crate::spec::Encoding>,
) -> Result<LayeredChart, ChartonError> {
    if !spec.layer.is_empty() {
        let mut merged = spec.encoding.clone();
        if let Some(parent) = inherited {
            merged.inherit(parent);
        }

        let mut accumulated: Option<LayeredChart> = None;
        for child in &spec.layer {
            let child_chart = build_tree(child, data.clone(), Some(&merged))?;
            accumulated = Some(match accumulated {
                Some(existing) => existing.and(child_chart),
                None => child_chart,
            });
        }

        return accumulated.ok_or_else(|| ChartonError::Mark("the layer list is empty".into()));
    }

    let mark = spec
        .resolved_mark()
        .ok_or_else(|| ChartonError::Mark("the document names no single mark".into()))?;
    let encoding = encoding_of(&spec.encoding, mark, inherited);
    let color = spec.mark_color().map(str::to_owned);

    let base = Chart::build(data)?;

    // Mark first, then its fixed properties, then the encoding: that is the
    // order the public examples use, and it is the order in which the mark
    // configuration is available to the transform that `encode` triggers.
    let chart: LayeredChart = match mark {
        SpecMark::Vega(MarkType::Point) => base
            .mark_point()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Vega(MarkType::Line) => base
            .mark_line()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Vega(MarkType::Bar) => base
            .mark_bar()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Vega(MarkType::Area) => base
            .mark_area()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Vega(MarkType::Rect) => base
            .mark_rect()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Vega(MarkType::Rule) => base
            .mark_rule()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Vega(MarkType::Text) => base
            .mark_text()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Vega(MarkType::Tick) => base
            .mark_tick()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Vega(MarkType::Boxplot) => base
            .mark_boxplot()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Vega(MarkType::Errorbar) => base
            .mark_errorbar()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Extension(ExtensionMark::Violin) => base
            .mark_violin()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Extension(ExtensionMark::Density) => base
            .mark_density()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Extension(ExtensionMark::Density2d) => base
            .mark_density_2d()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Extension(ExtensionMark::Hist) => base
            .mark_hist()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Extension(ExtensionMark::Polygon) => base
            .mark_polygon()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Extension(ExtensionMark::Path) => base
            .mark_path()?
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Extension(ExtensionMark::Beeswarm) => base
            .mark_point()?
            // A swarm is a point mark with a placement strategy, not a new
            // mark: the layout decides where the symbols go, the mark draws them.
            .configure_point(|mark| mark.with_layout(PointLayout::Beeswarm))
            .paint(color.as_deref())
            .encode(Whole(encoding))?
            .into(),
        SpecMark::Extension(ExtensionMark::Contour) => {
            let scalar = spec
                .scalar_field()
                .ok_or_else(|| ChartonError::Mark("contour needs a scalar field".into()))?;
            base.mark_contour(scalar)?
                .paint(color.as_deref())
                .encode(Whole(encoding))?
                .into()
        }
        SpecMark::Extension(mark) => {
            return Err(ChartonError::Unimplemented(format!(
                "Charton mark `{}` is accepted by the validator but has no lowering yet",
                mark.as_str()
            )));
        }
    };

    Ok(chart)
}

/// The facet description as the core's.
fn facet_spec(facet: &crate::spec::FacetDef) -> crate::facets::FacetSpec {
    if let Some(field) = facet.field.as_ref() {
        let spec = crate::facets::FacetSpec::wrap(field);
        return match facet.columns {
            Some(columns) => spec.with_columns(columns),
            None => spec,
        };
    }

    crate::facets::FacetSpec::grid(
        facet.row.as_deref().unwrap_or_default(),
        facet.column.as_deref().unwrap_or_default(),
    )
}

/// Hands a fully-built encoding to the chart.
///
/// The public builder API exposes one entry point per channel, because a Rust
/// user writes one channel at a time. A spec arrives complete, so it is applied
/// in one step instead of being re-split into builder calls that would have to
/// re-encode the exact same fields.
struct Whole(Encoding);

impl IntoEncoding for Whole {
    fn apply(self, target: &mut Encoding) {
        *target = self.0;
    }
}

fn with_metadata(chart: LayeredChart, spec: &Spec) -> LayeredChart {
    let mut chart = chart;
    if let Some(title) = spec.title.as_ref() {
        chart = chart.with_title(title.clone());
    }
    if let Some(description) = spec.description.as_ref() {
        chart = chart.with_description(description.clone());
    }
    if let Some(theme) = spec.theme {
        chart = chart.with_theme(theme.to_core());
    }
    chart
}

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

fn encoding_of(
    source: &crate::spec::Encoding,
    mark: SpecMark,
    inherited: Option<&crate::spec::Encoding>,
) -> Encoding {
    let mut merged = source.clone();
    if let Some(parent) = inherited {
        merged.inherit(parent);
    }
    let source = &merged;

    let mut encoding = Encoding {
        x: source.x.as_ref().map(build_x),
        y: source.y.as_ref().map(build_y),
        y2: source.y2.as_ref().map(build_y2),
        color: source.color.as_ref().map(build_color),
        size: source.size.as_ref().map(build_size),
        shape: source.shape.as_ref().map(build_shape),
        text: source.text.as_ref().map(build_text),
        path_group: source.path_group.as_ref().map(build_path_group),
    };

    // The histogram mark *is* a count over x: it creates the count column and
    // writes it to y. When the document leaves y out, naming that generated
    // column here is the mark's own semantics, not a guess about the data.
    if matches!(mark, SpecMark::Extension(ExtensionMark::Hist)) && encoding.y.is_none() {
        encoding.y = Some(Y::new("count"));
    }

    encoding
}

/// Scale settings shared by every channel that has a scale.
struct ScaleParts {
    kind: Option<Scale>,
    domain: Option<ScaleDomain>,
    zero: Option<bool>,
    reverse: bool,
}

/// Resolve a channel's scale once, so every channel reads the same rules.
///
/// An explicit `scale.type` wins over a declared field `type`, which wins over
/// inference from the data.
fn scale_parts(def: &FieldDef) -> ScaleParts {
    let scale = def.scale.as_ref();
    let kind = scale
        .and_then(|scale| scale.scale_type)
        .or_else(|| def.data_type.map(scale_type_of))
        .map(|kind| match kind {
            ScaleTypeSpec::Linear => Scale::Linear,
            ScaleTypeSpec::Log => Scale::Log,
            ScaleTypeSpec::Band | ScaleTypeSpec::Point | ScaleTypeSpec::Ordinal => Scale::Discrete,
            ScaleTypeSpec::Time | ScaleTypeSpec::Utc => Scale::Temporal,
        });

    ScaleParts {
        kind,
        domain: scale.and_then(|scale| scale.domain.as_ref()).map(domain_of),
        zero: scale.and_then(|scale| scale.zero),
        reverse: scale.and_then(|scale| scale.reverse).unwrap_or(false),
    }
}

/// A declared field type is a scale hint: it says how the values should be
/// read, which is exactly what a scale type decides.
const fn scale_type_of(data_type: crate::spec::types::DataType) -> ScaleTypeSpec {
    match data_type {
        crate::spec::types::DataType::Quantitative => ScaleTypeSpec::Linear,
        crate::spec::types::DataType::Nominal | crate::spec::types::DataType::Ordinal => {
            ScaleTypeSpec::Band
        }
        crate::spec::types::DataType::Temporal => ScaleTypeSpec::Time,
    }
}

fn domain_of(domain: &DomainSpec) -> ScaleDomain {
    match domain {
        DomainSpec::Continuous([min, max]) => ScaleDomain::Continuous(*min, *max),
        DomainSpec::Discrete(labels) => ScaleDomain::Discrete(labels.clone()),
    }
}

/// `bin: true` means "let Charton choose", which is what it does for the marks
/// that bin at all (`hist` on x, `rect` on x and y).
fn bin_count(def: &FieldDef) -> Option<usize> {
    match def.bin.as_ref()? {
        BinSpec::Enabled(true) => None,
        BinSpec::Enabled(false) => None,
        BinSpec::Params(params) => params.maxbins,
    }
}

fn build_x(def: &FieldDef) -> X {
    let parts = scale_parts(def);
    let mut x = X::new(&def.field);
    x.label = def.title.clone();
    x.scale_type = parts.kind;
    x.domain = parts.domain;
    x.zero = parts.zero;
    x.reverse = parts.reverse;
    x.bins = bin_count(def);
    x
}

fn build_y(def: &FieldDef) -> Y {
    let parts = scale_parts(def);
    let mut y = Y::new(&def.field);
    y.label = def.title.clone();
    y.aggregate = def
        .aggregate
        .map(AggregateSpec::to_core)
        .unwrap_or_default();
    y.scale_type = parts.kind;
    y.domain = parts.domain;
    y.zero = parts.zero;
    y.reverse = parts.reverse;
    y.bins = bin_count(def);
    y.stack = def.stack.map(stack_of).unwrap_or_default();
    y
}

fn build_y2(def: &FieldDef) -> Y2 {
    Y2::new(&def.field)
}

fn build_color(def: &FieldDef) -> Color {
    let parts = scale_parts(def);
    let mut color = Color::new(&def.field);
    color.label = def.title.clone();
    color.aggregate = def
        .aggregate
        .map(AggregateSpec::to_core)
        .unwrap_or_default();
    color.scale_type = parts.kind;
    color.domain = parts.domain;
    color
}

fn build_size(def: &FieldDef) -> Size {
    let parts = scale_parts(def);
    let mut size = Size::new(&def.field);
    size.label = def.title.clone();
    size.scale_type = parts.kind;
    size.domain = parts.domain;
    size
}

fn build_shape(def: &FieldDef) -> Shape {
    let parts = scale_parts(def);
    let mut shape = Shape::new(&def.field);
    shape.label = def.title.clone();
    shape.scale_type = parts.kind;
    shape.domain = parts.domain;
    shape
}

fn build_text(def: &FieldDef) -> Text {
    Text::new(&def.field)
}

fn build_path_group(def: &FieldDef) -> PathGroup {
    let parts = scale_parts(def);
    let mut group = PathGroup::new(&def.field);
    group.scale_type = parts.kind;
    group.domain = parts.domain;
    group
}

/// Vega-Lite's stacking modes, mapped to the core's.
///
/// `"zero"` is Vega-Lite's default for bar and area, and it is the same thing as
/// the core's `Stacked`: a slice grows from the accumulated baseline.
const fn stack_of(stack: StackSpec) -> crate::encode::y::StackMode {
    use crate::encode::y::StackMode;

    match stack {
        StackSpec::Flag(true) => StackMode::Stacked,
        StackSpec::Flag(false) => StackMode::None,
        StackSpec::Mode(StackKind::Zero) => StackMode::Stacked,
        StackSpec::Mode(StackKind::Normalize) => StackMode::Normalize,
        StackSpec::Mode(StackKind::Center) => StackMode::Center,
        StackSpec::Mode(StackKind::None) => StackMode::None,
    }
}

// ---------------------------------------------------------------------------
// Fixed mark colour
// ---------------------------------------------------------------------------

/// A mark that can take a fixed colour.
///
/// The public API spells this `configure_bar(|mark| mark.with_color(..))`: one
/// closure per mark type, because each mark has its own option struct. Lowering
/// has a single colour for every mark, so the difference is captured once here
/// rather than repeated in ten match arms.
trait PaintMark: Sized {
    fn paint(self, color: Option<&str>) -> Self;
}

macro_rules! impl_paint_mark {
    ($($chart:ty => $configure:ident),+ $(,)?) => {
        $(
            impl PaintMark for $chart {
                fn paint(self, color: Option<&str>) -> Self {
                    self.$configure(|mark| match color {
                        Some(color) => mark.with_color(color),
                        None => mark,
                    })
                }
            }
        )+
    };
}

impl PaintMark for Chart<MarkContour> {
    fn paint(self, color: Option<&str>) -> Self {
        self.configure_contour(|mark| match color {
            Some(color) => mark.with_stroke(color),
            None => mark,
        })
    }
}

impl PaintMark for Chart<MarkPath> {
    fn paint(self, color: Option<&str>) -> Self {
        self.configure_path(|mark| match color {
            Some(color) => mark.with_stroke(color),
            None => mark,
        })
    }
}

impl PaintMark for Chart<MarkPolygon> {
    fn paint(self, color: Option<&str>) -> Self {
        self.configure_polygon(|mark| match color {
            Some(color) => mark.with_fill(color),
            None => mark,
        })
    }
}

impl_paint_mark!(
    Chart<MarkViolin> => configure_violin,
    Chart<MarkDensity> => configure_density,
    Chart<MarkDensity2D> => configure_density_2d,
    Chart<MarkHist> => configure_hist,
    Chart<MarkPoint> => configure_point,
    Chart<MarkLine> => configure_line,
    Chart<MarkBar> => configure_bar,
    Chart<MarkArea> => configure_area,
    Chart<MarkRect> => configure_rect,
    Chart<MarkRule> => configure_rule,
    Chart<MarkText> => configure_text,
    Chart<MarkTick> => configure_tick,
    Chart<MarkBoxplot> => configure_boxplot,
    Chart<MarkErrorBar> => configure_errorbar,
);

/// Turn a core failure into the diagnostic that says "this is our bug".
fn internal(error: ChartonError, spec: &Spec) -> SpecError {
    let mark = spec.resolved_mark().map_or("unknown", SpecMark::as_str);
    SpecError::new(
        codes::INTERNAL,
        "",
        format!("Spec was accepted but `{mark}` could not be built: {error}"),
    )
    .with_hint(
        "This is a bug in Charton's validator, not in the document. \
         Please report it together with the spec.",
    )
}
