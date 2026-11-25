//! The spec's data model.
//!
//! Field names are Vega-Lite v5's and keep Vega-Lite's meaning; the only
//! additions sit behind the top-level `charton` key (see [`ChartonExt`]).
//!
//! Every struct denies unknown fields: a misspelled key is an error rather than
//! an ignored property, which is what makes a document reviewable.
use crate::core::data::SemanticType;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The schema URL this build writes into newly emitted specs.
///
/// The URL is Charton's own, not Vega-Lite's: pointing at the official schema
/// would make editors and agents validate against the *full* language and then
/// fail inside Charton. The string also carries the spec version, which is
/// intentionally decoupled from the crate version.
pub const SPEC_SCHEMA_URL: &str = "https://charton.dev/schema/charton-v0.schema.json";

/// The spec version this build understands.
pub const SUPPORTED_SPEC_VERSION: &str = "v0";

/// A chart definition.
///
/// ```json
/// {
///   "$schema": "https://charton.dev/schema/charton-v0.schema.json",
///   "data": { "url": "sales.csv" },
///   "mark": "line",
///   "encoding": { "x": { "field": "month" }, "y": { "field": "revenue" } }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Spec {
    /// Spec version carrier. Absent means "the version this build supports";
    /// present and unknown is an error, never a silent reinterpretation.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Where the rows come from.
    ///
    /// Required on the document root; layers inherit it and must not repeat it.
    /// Reading the rows is the caller's job — the core only ever sees an
    /// already-normalised dataset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Data>,

    /// What to draw, in Vega-Lite's vocabulary.
    ///
    /// Optional because Charton-only marks are written under
    /// [`ChartonExt::mark`] instead. Exactly one of the two must be present;
    /// writing both is an error rather than a precedence rule nobody remembers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<Mark>,

    /// Which field goes where. Defaulted so `--emit-spec` can omit it.
    #[serde(default)]
    pub encoding: Encoding,

    /// Layers drawn on one set of axes.
    ///
    /// A document is either one mark or a list of layers, never both. Layers
    /// inherit `data`, `transform` and `encoding` from the root; a layer's own
    /// encoding wins channel by channel.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer: Vec<Spec>,

    /// Split the chart into panels by a field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facet: Option<FacetDef>,

    /// Data transformations, applied in order before the mark draws.
    ///
    /// An array because order is meaning: filtering before aggregating is not
    /// the same chart as aggregating before filtering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transform: Vec<Transform>,

    /// Charton extensions, always explicit and always namespaced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charton: Option<ChartonExt>,

    /// Human-readable chart title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Accessible description, also written into the SVG's `<desc>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Built-in colour scheme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<ThemeName>,
}

impl Spec {
    /// The mark this document asks for, when it asks for exactly one.
    ///
    /// `None` means the document is ambiguous (both or neither); [`crate::spec::validate`]
    /// reports that as `E_MARK_SOURCE`.
    #[must_use]
    pub fn resolved_mark(&self) -> Option<SpecMark> {
        match (
            self.mark.as_ref(),
            self.charton.as_ref().and_then(|ext| ext.mark),
        ) {
            (Some(Mark::Type(mark)), None) => Some(SpecMark::Vega(*mark)),
            (Some(Mark::Def(def)), None) => Some(SpecMark::Vega(def.mark_type)),
            (None, Some(mark)) => Some(SpecMark::Extension(mark)),
            _ => None,
        }
    }

    /// The extension scalar: the field named by `charton.z`.
    #[must_use]
    pub fn scalar_field(&self) -> Option<&str> {
        self.charton.as_ref()?.z.as_deref()
    }

    /// The fixed mark colour, from either spelling of `mark`.
    #[must_use]
    pub fn mark_color(&self) -> Option<&str> {
        match self.mark.as_ref()? {
            Mark::Def(def) => def.color.as_deref(),
            Mark::Type(_) => None,
        }
    }

    /// The spec version named by `$schema`, or [`SUPPORTED_SPEC_VERSION`] when
    /// the document omits it.
    ///
    /// Returns the raw URL when it is not a recognised Charton schema URL; the
    /// caller turns that into a diagnostic with the supported range.
    #[must_use]
    pub fn declared_version(&self) -> Option<&str> {
        let schema = self.schema.as_deref()?;
        Some(schema.rsplit('/').next().unwrap_or(schema))
    }
}

/// The mark a document resolved to.
///
/// Exactly one of `mark` and `charton.mark` may be present, so a resolved mark
/// is unambiguous by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecMark {
    /// A mark that shares Vega-Lite's name and meaning.
    Vega(MarkType),
    /// A mark that only Charton has.
    Extension(ExtensionMark),
}

impl SpecMark {
    /// The wire name, for diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vega(mark) => mark.as_str(),
            Self::Extension(mark) => mark.as_str(),
        }
    }
}

/// The built-in themes.
///
/// A closed set, because a theme is a whole visual system rather than a pile of
/// colours; anything finer belongs in the Rust API's builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    /// Dark ink on a white page.
    Light,
    /// Light ink on a dark page.
    Dark,
}

impl ThemeName {
    /// Every theme a document may name.
    pub const ALL: &[Self] = &[Self::Light, Self::Dark];

    /// The wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// The core theme.
    #[must_use]
    pub fn to_core(self) -> crate::theme::Theme {
        match self {
            Self::Light => crate::theme::Theme::light(),
            Self::Dark => crate::theme::Theme::dark(),
        }
    }
}

/// How a chart is split into panels.
///
/// Two layouts, both from Vega-Lite: `field` wraps into a grid, `row` + `column`
/// makes an explicit matrix. Anything else is refused rather than approximated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FacetDef {
    /// Wrap panels by this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,

    /// Rows of the matrix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<String>,

    /// Columns of the matrix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<String>,

    /// Column count for the wrap layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<usize>,
}

/// A data reference.
///
/// Exactly one of `url` and `values` must be present. `url` is a *path*, not a
/// web address: arbitrary URLs are rejected, and a relative path resolves
/// against the spec file's directory, not the process's working directory.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Data {
    /// Path to a CSV or NDJSON file, relative to the spec file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Inline rows, size-limited. Flat scalar cells only — nesting is not a
    /// shape a chart can encode, and allowing it invites reading it as a
    /// program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<InlineRow>>,
}

/// One inline row: column name to scalar cell.
pub type InlineRow = std::collections::BTreeMap<String, InlineValue>;

/// A scalar cell of inline data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum InlineValue {
    /// JSON `null`, i.e. a missing value.
    Null(()),
    /// A boolean.
    Bool(bool),
    /// A number.
    Number(f64),
    /// A string, including ISO dates.
    Text(String),
}

/// How a mark is written.
///
/// Both Vega-Lite spellings are accepted: the shorthand `"mark": "line"` and the
/// object form that can carry mark properties.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Mark {
    /// `"mark": "line"`
    Type(MarkType),
    /// `"mark": {"type": "line", "color": "#4c78a8"}`
    Def(MarkDef),
}

// Hand-written on purpose. A derived untagged enum reports
// "data did not match any variant of untagged enum Mark", which tells the
// reader nothing and tells an agent even less. Naming the rejected mark and
// listing what is valid is the whole point of the diagnostic.
impl<'de> Deserialize<'de> for Mark {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        match serde_json::Value::deserialize(deserializer)? {
            serde_json::Value::String(name) => {
                MarkType::from_wire(&name).map(Mark::Type).ok_or_else(|| {
                    D::Error::custom(format!(
                        "unknown mark `{name}`; expected one of: {}",
                        candidate_list(MarkType::ALL.iter().map(|mark| mark.as_str()))
                    ))
                })
            }
            value @ serde_json::Value::Object(_) => serde_json::from_value(value)
                .map(Mark::Def)
                .map_err(D::Error::custom),
            other => Err(D::Error::custom(format!(
                "`mark` must be a mark name or an object, found {}",
                json_type_name(&other)
            ))),
        }
    }
}

/// Rendering helper shared by the hand-written deserializers.
fn candidate_list<'a>(names: impl Iterator<Item = &'a str>) -> String {
    names.collect::<Vec<_>>().join(", ")
}

/// A human name for a JSON value's kind, for `expected X, found Y` messages.
const fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}

/// Marks that share Vega-Lite's name and meaning.
///
/// Charton-only marks are **not** listed here; they belong to
/// [`ChartonExt::mark`], so a name can never be read two ways.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MarkType {
    /// A scatter-plot symbol.
    Point,
    /// A connected line through the rows in x order.
    Line,
    /// A bar from the baseline to the value.
    Bar,
    /// A filled area between the line and the baseline.
    Area,
    /// A rectangle covering a bin or a category cell.
    Rect,
    /// A single line segment, horizontal or vertical.
    Rule,
    /// A text label at the encoded position.
    Text,
    /// A short tick at the encoded position.
    Tick,
    /// A composite mark: quartile box plus whiskers.
    Boxplot,
    /// A composite mark: a point estimate with its interval.
    Errorbar,
}

impl MarkType {
    /// Every supported Vega-Lite mark name, in the order help text shows them.
    pub const ALL: &[Self] = &[
        Self::Point,
        Self::Line,
        Self::Bar,
        Self::Area,
        Self::Rect,
        Self::Rule,
        Self::Text,
        Self::Tick,
        Self::Boxplot,
        Self::Errorbar,
    ];

    /// The wire name, identical to Vega-Lite's.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Point => "point",
            Self::Line => "line",
            Self::Bar => "bar",
            Self::Area => "area",
            Self::Rect => "rect",
            Self::Rule => "rule",
            Self::Text => "text",
            Self::Tick => "tick",
            Self::Boxplot => "boxplot",
            Self::Errorbar => "errorbar",
        }
    }

    /// Parse a wire name, or `None` when it is not a supported mark.
    ///
    /// Charton-only marks deliberately do not resolve here: they belong to
    /// [`ChartonExt::mark`] and must not be readable as a Vega-Lite mark.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|mark| mark.as_str() == name)
    }
}

/// The object form of a mark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MarkDef {
    /// Which mark to draw.
    #[serde(rename = "type")]
    pub mark_type: MarkType,

    /// Fixed fill/stroke colour, e.g. `"#4c78a8"` or `"steelblue"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// Fixed mark size: radius for points, width for bars, stroke width for lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,

    /// Fixed opacity in `0.0..=1.0`. A property here, not an encoding channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
}

/// Charton-only capabilities, in their own namespace.
///
/// A document with a `charton` key is deliberately **not** valid Vega-Lite, so
/// no other tool will silently misread it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChartonExt {
    /// A mark that Vega-Lite has no name for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<ExtensionMark>,

    /// The scalar field a mark needs but no encoding channel carries.
    ///
    /// Only `contour` uses it: iso-lines need a height, and `x`/`y` are already
    /// the two grid axes. Required for `contour`, refused everywhere else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z: Option<String>,
}

/// Marks that exist only in Charton.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ExtensionMark {
    /// An arbitrary closed polygon from encoded vertices.
    #[serde(rename = "polygon")]
    Polygon,
    /// An open path from encoded vertices.
    #[serde(rename = "path")]
    Path,
    /// A kernel density estimate plus its mirrored band.
    #[serde(rename = "violin")]
    Violin,
    /// A one-dimensional kernel density estimate.
    #[serde(rename = "density")]
    Density,
    /// A two-dimensional density field.
    #[serde(rename = "density_2d")]
    Density2d,
    /// Iso-lines of a two-dimensional density field.
    #[serde(rename = "contour")]
    Contour,
    /// Points displaced to avoid overlap.
    #[serde(rename = "beeswarm")]
    Beeswarm,
    /// A histogram on a continuous position.
    #[serde(rename = "hist")]
    Hist,
    /// The quantile box used by the composite `boxplot` mark, on its own.
    #[serde(rename = "quantile_box")]
    QuantileBox,
}

impl ExtensionMark {
    /// Every supported extension mark name.
    pub const ALL: &[Self] = &[
        Self::Polygon,
        Self::Path,
        Self::Violin,
        Self::Density,
        Self::Density2d,
        Self::Contour,
        Self::Beeswarm,
        Self::Hist,
        Self::QuantileBox,
    ];

    /// The wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Polygon => "polygon",
            Self::Path => "path",
            Self::Violin => "violin",
            Self::Density => "density",
            Self::Density2d => "density_2d",
            Self::Contour => "contour",
            Self::Beeswarm => "beeswarm",
            Self::Hist => "hist",
            Self::QuantileBox => "quantile_box",
        }
    }
}

/// The encoding block: which data field feeds which visual channel.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Encoding {
    /// Horizontal position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<FieldDef>,
    /// Vertical position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<FieldDef>,
    /// The far end of a vertical span (`area`, `rule`, `errorbar`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y2: Option<FieldDef>,
    /// Colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<FieldDef>,
    /// Symbol area.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<FieldDef>,
    /// Symbol shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<FieldDef>,
    /// Label content, for the `text` mark.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<FieldDef>,
    /// Which line a row belongs to, when one line renderer draws many.
    ///
    /// A Charton extension channel: Vega-Lite expresses the same thing through
    /// `detail`/`order`, which this subset does not implement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_group: Option<FieldDef>,
}

impl Encoding {
    /// Fills empty channels from a parent encoding.
    ///
    /// A layer states only what differs from its parent; everything it leaves
    /// out is inherited. The child always wins where both set a channel, so
    /// there is no precedence rule to remember beyond that.
    pub(crate) fn inherit(&mut self, parent: &Self) {
        macro_rules! fill {
            ($($field:ident),+ $(,)?) => {
                $(
                    if self.$field.is_none() {
                        self.$field = parent.$field.clone();
                    }
                )+
            };
        }
        fill!(x, y, y2, color, size, shape, text, path_group);
    }

    /// Iterate over every channel that is present, with its JSON key.
    pub fn present(&self) -> impl Iterator<Item = (&'static str, &FieldDef)> {
        [
            ("x", self.x.as_ref()),
            ("y", self.y.as_ref()),
            ("y2", self.y2.as_ref()),
            ("color", self.color.as_ref()),
            ("size", self.size.as_ref()),
            ("shape", self.shape.as_ref()),
            ("text", self.text.as_ref()),
            ("pathGroup", self.path_group.as_ref()),
        ]
        .into_iter()
        .filter_map(|(name, def)| def.map(|def| (name, def)))
    }
}

/// One channel's field reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FieldDef {
    /// Column name in the dataset.
    pub field: String,

    /// Optional declared type.
    ///
    /// Not required — Charton infers the type from the data, as Vega-Lite does.
    /// When written, it is enforced: a mismatch between the declaration and the
    /// data is an `E_DATA_TYPE`. That way a spec can be pinned when it has to
    /// be reproducible against data that may later change.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub data_type: Option<DataType>,

    /// Summarise the field before encoding it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate: Option<AggregateSpec>,

    /// Bin the field before encoding it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bin: Option<BinSpec>,

    /// Stack this channel across the colour dimension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<StackSpec>,

    /// Scale configuration for this channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<ScaleDef>,

    /// Axis or legend title, when it differs from the field name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl FieldDef {
    /// A channel bound to `field`, with every option left to the data.
    #[must_use]
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            data_type: None,
            aggregate: None,
            bin: None,
            stack: None,
            scale: None,
            title: None,
        }
    }
}

/// Vega-Lite's declared field types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    /// Continuous numeric.
    Quantitative,
    /// Unordered categorical.
    Nominal,
    /// Ordered categorical.
    Ordinal,
    /// Point in time.
    Temporal,
}

/// Aggregate operators this subset implements.
///
/// Vega-Lite's `average` is accepted as an alias of `mean` because both are
/// common in existing documents; the rest of Vega-Lite's operators are rejected
/// as unsupported rather than guessed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AggregateSpec {
    /// Number of rows in the group.
    Count,
    /// Arithmetic mean.
    Mean,
    /// Alias of [`AggregateSpec::Mean`], as used by Vega-Lite.
    Average,
    /// Sum over the group.
    Sum,
    /// Middle value.
    Median,
    /// Smallest value.
    Min,
    /// Largest value.
    Max,
}

impl AggregateSpec {
    /// Every accepted name.
    pub const ALL: &[Self] = &[
        Self::Count,
        Self::Mean,
        Self::Average,
        Self::Sum,
        Self::Median,
        Self::Min,
        Self::Max,
    ];

    /// The core operation.
    #[must_use]
    pub(crate) const fn to_core(self) -> crate::core::data::AggregateOp {
        use crate::core::data::AggregateOp;

        match self {
            Self::Count => AggregateOp::Count,
            Self::Mean | Self::Average => AggregateOp::Mean,
            Self::Sum => AggregateOp::Sum,
            Self::Median => AggregateOp::Median,
            Self::Min => AggregateOp::Min,
            Self::Max => AggregateOp::Max,
        }
    }

    /// The wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Mean => "mean",
            Self::Average => "average",
            Self::Sum => "sum",
            Self::Median => "median",
            Self::Min => "min",
            Self::Max => "max",
        }
    }
}

/// `bin: true` or `bin: {"maxbins": 20}`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum BinSpec {
    /// Bin with the default resolution.
    Enabled(bool),
    /// Bin with explicit parameters.
    Params(BinParams),
}

/// Bin resolution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BinParams {
    /// Upper bound on the number of bins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maxbins: Option<usize>,
    /// Fixed bin width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
}

/// `stack: "zero" | "normalize" | "center" | "none" | true | false`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum StackSpec {
    /// `true` is `"zero"`, `false` is `"none"`, as in Vega-Lite.
    Flag(bool),
    /// The named modes.
    Mode(StackKind),
}

/// Named stacking modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum StackKind {
    /// Stack from zero (the Vega-Lite default for bar and area).
    Zero,
    /// Scale each slice to sum to one.
    Normalize,
    /// Centre the stack around the baseline.
    Center,
    /// Do not stack.
    None,
}

/// Scale configuration for one channel.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScaleDef {
    /// Which scale family to use.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub scale_type: Option<ScaleTypeSpec>,
    /// Explicit domain, as two numbers or a list of categories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<DomainSpec>,
    /// Force the domain to include zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zero: Option<bool>,
    /// Flip the axis direction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,
}

/// Scale types this subset implements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScaleTypeSpec {
    /// Continuous, linear.
    Linear,
    /// Continuous, logarithmic.
    Log,
    /// Categorical, one band per category.
    Band,
    /// Categorical, one point per category.
    Point,
    /// Ordered categorical.
    Ordinal,
    /// Time, in the local zone.
    Time,
    /// Time, in UTC.
    Utc,
}

impl ScaleTypeSpec {
    /// Every accepted name.
    pub const ALL: &[Self] = &[
        Self::Linear,
        Self::Log,
        Self::Band,
        Self::Point,
        Self::Ordinal,
        Self::Time,
        Self::Utc,
    ];

    /// The wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Log => "log",
            Self::Band => "band",
            Self::Point => "point",
            Self::Ordinal => "ordinal",
            Self::Time => "time",
            Self::Utc => "utc",
        }
    }
}

/// An explicit domain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DomainSpec {
    /// `[min, max]`
    Continuous([f64; 2]),
    /// A category list, in tick order.
    Discrete(Vec<String>),
}

// ---------------------------------------------------------------------------
// Transforms
// ---------------------------------------------------------------------------

/// One entry of the `transform` array.
///
/// A sum type, not a bag of optional fields: an entry is exactly one transform,
/// and the type says so. Deserialisation is hand-written so that a transform
/// Charton recognises but cannot run is reported as *unsupported* rather than as
/// an unknown field — the two mean very different things to whoever is fixing
/// the document.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Transform {
    /// Keep only the rows a predicate accepts.
    Filter(FilterEntry),
    /// Turn columns into a name/value pair.
    Fold(FoldEntry),
    /// Collapse rows into one per group.
    Aggregate(AggregateEntry),
    /// Write the edges of each row's bin as two columns.
    Bin(BinEntry),
    /// Add a column computed from each row's position in its group.
    Window(WindowEntry),
}

impl Transform {
    /// The transform's name, for diagnostics.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Filter(_) => "filter",
            Self::Fold(_) => "fold",
            Self::Aggregate(_) => "aggregate",
            Self::Bin(_) => "bin",
            Self::Window(_) => "window",
        }
    }
}

/// `{"filter": {...}}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FilterEntry {
    /// The predicate.
    pub filter: FilterExpr,
}

/// `{"fold": ["a", "b"], "as": ["key", "value"]}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FoldEntry {
    /// Columns to fold, in output order.
    pub fold: Vec<String>,
    /// Names for the two output columns. Defaults to `["key", "value"]`.
    #[serde(rename = "as", default, skip_serializing_if = "Option::is_none")]
    pub as_names: Option<[String; 2]>,
}

/// `{"aggregate": [...], "groupby": [...]}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AggregateEntry {
    /// The measures to compute.
    pub aggregate: Vec<AggregateFieldSpec>,
    /// Columns that define a group. Empty means "one group for the whole table".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groupby: Vec<String>,
}

/// One measure inside an `aggregate` entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AggregateFieldSpec {
    /// The operation.
    pub op: AggregateSpec,
    /// The column to read. Omitted only for `count`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Output column name.
    #[serde(rename = "as")]
    pub as_name: String,
}

/// `{"bin": true, "field": "x", "as": ["bin_x", "bin_x_end"]}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BinEntry {
    /// Bin resolution: `true` for the default, or `{"maxbins": n}`.
    pub bin: BinSpec,
    /// The column to bin.
    pub field: String,
    /// Names for the two output columns. Default to `bin_<field>` and
    /// `bin_<field>_end`.
    #[serde(rename = "as", default, skip_serializing_if = "Option::is_none")]
    pub as_names: Option<[String; 2]>,
}

/// `{"window": [...], "groupby": [...]}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WindowEntry {
    /// The window functions to add, applied in order.
    pub window: Vec<WindowFieldSpec>,
    /// Columns that define a window. Empty means "one window for the whole table".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groupby: Vec<String>,
}

/// One window function inside a `window` entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WindowFieldSpec {
    /// The operation.
    pub op: WindowOpSpec,
    /// The column the operation reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Output column name.
    #[serde(rename = "as")]
    pub as_name: String,
}

/// Window operations.
///
/// The set mirrors the core's. Operations the core knows but has not implemented
/// are listed so a document using one gets "recognised, not implemented" rather
/// than "unknown operation".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum WindowOpSpec {
    /// Position in the group, ties share a rank and the next rank skips.
    #[serde(rename = "rank")]
    Rank,
    /// One counter per row, in group order.
    #[serde(rename = "row_number")]
    RowNumber,
    /// Cumulative distribution: the ECDF step.
    #[serde(rename = "cume_dist")]
    CumeDist,
    /// Running total, accumulated in row order.
    #[serde(rename = "cumulative_sum")]
    CumulativeSum,
    /// Recognised; the core does not implement it yet.
    #[serde(rename = "dense_rank")]
    DenseRank,
    /// Recognised; the core does not implement it yet.
    #[serde(rename = "percent_rank")]
    PercentRank,
    /// Recognised; the core does not implement it yet.
    #[serde(rename = "first_value")]
    FirstValue,
    /// Recognised; the core does not implement it yet.
    #[serde(rename = "last_value")]
    LastValue,
}

impl WindowOpSpec {
    /// Operations this build can actually run.
    pub const IMPLEMENTED: &[Self] = &[
        Self::Rank,
        Self::RowNumber,
        Self::CumeDist,
        Self::CumulativeSum,
    ];

    /// The wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rank => "rank",
            Self::RowNumber => "row_number",
            Self::CumeDist => "cume_dist",
            Self::CumulativeSum => "cumulative_sum",
            Self::DenseRank => "dense_rank",
            Self::PercentRank => "percent_rank",
            Self::FirstValue => "first_value",
            Self::LastValue => "last_value",
        }
    }

    /// Whether this build can run it.
    #[must_use]
    pub fn is_implemented(self) -> bool {
        Self::IMPLEMENTED.contains(&self)
    }

    /// The core operation.
    #[must_use]
    pub(crate) const fn to_core(self) -> crate::transform::window_transform::WindowOnlyOp {
        use crate::transform::window_transform::WindowOnlyOp;

        match self {
            Self::Rank => WindowOnlyOp::Rank,
            Self::RowNumber => WindowOnlyOp::RowNumber,
            Self::CumeDist => WindowOnlyOp::CumeDist,
            Self::CumulativeSum => WindowOnlyOp::CumulativeSum,
            Self::DenseRank => WindowOnlyOp::DenseRank,
            Self::PercentRank => WindowOnlyOp::PercentRank,
            Self::FirstValue => WindowOnlyOp::FirstValue,
            Self::LastValue => WindowOnlyOp::LastValue,
        }
    }
}

impl<'de> Deserialize<'de> for Transform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        const KINDS: &[&str] = &["filter", "fold", "aggregate", "bin", "window"];
        const UNSUPPORTED: &[&str] = &["joinaggregate", "stack"];

        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("each transform must be an object with one key"))?;
        let keys: Vec<&str> = object.keys().map(String::as_str).collect();

        // The red line first: a document must never be able to run code.
        if keys.contains(&"calculate") {
            return Err(D::Error::custom(
                "expression transform `calculate` is not supported: a spec is data, not a program",
            ));
        }
        if let Some(other) = keys.iter().find(|key| UNSUPPORTED.contains(key)) {
            return Err(D::Error::custom(format!(
                "unsupported transform `{other}`: recognised, not implemented yet"
            )));
        }

        let kinds: Vec<&str> = keys
            .iter()
            .copied()
            .filter(|key| KINDS.contains(key))
            .collect();

        match kinds.as_slice() {
            [] => {
                if keys.contains(&"as") {
                    return Err(D::Error::custom(
                        "misplaced `as`: it names the columns `fold` creates, and nothing is being folded",
                    ));
                }
                let other = keys.first().copied().unwrap_or("(empty)");
                Err(D::Error::custom(format!(
                    "unknown transform `{other}`; expected one of: filter, fold, aggregate, bin, window"
                )))
            }
            [kind] => {
                let entry = serde_json::Value::Object(object.clone());

                match *kind {
                    "filter" => {
                        if entry["filter"].is_string() {
                            return Err(D::Error::custom(
                                "expression filter strings are not supported; write {field, equal|range|oneOf}",
                            ));
                        }
                        parse_entry::<FilterEntry>(entry)
                            .map(Transform::Filter)
                            .map_err(D::Error::custom)
                    }
                    "fold" => parse_entry::<FoldEntry>(entry)
                        .map(Transform::Fold)
                        .map_err(D::Error::custom),
                    "aggregate" => parse_entry::<AggregateEntry>(entry)
                        .map(Transform::Aggregate)
                        .map_err(D::Error::custom),
                    "bin" => parse_entry::<BinEntry>(entry)
                        .map(Transform::Bin)
                        .map_err(D::Error::custom),
                    _ => {
                        // A sort or frame on a window would be silently ignored
                        // by the core, so it is refused instead.
                        if let Some(option) =
                            keys.iter().find(|key| matches!(**key, "sort" | "frame"))
                        {
                            return Err(D::Error::custom(format!(
                                "unsupported window option `{option}`: these window functions read their own order and frame"
                            )));
                        }
                        parse_entry::<WindowEntry>(entry)
                            .map(Transform::Window)
                            .map_err(D::Error::custom)
                    }
                }
            }
            _ => Err(D::Error::custom(
                "one transform per entry: an object cannot be two transforms at once",
            )),
        }
    }
}

/// Deserialise one transform entry, keeping the path to the failing field.
///
/// The outer deserialiser only knows the path down to the entry; without this,
/// an unknown operator would be reported at `/transform/0` instead of at
/// `/transform/0/aggregate/0/op`. The inner path is prefixed onto the message and
/// turned into a pointer by the error mapper.
fn parse_entry<T: serde::de::DeserializeOwned>(entry: serde_json::Value) -> Result<T, String> {
    serde_path_to_error::deserialize::<_, T>(entry).map_err(|error| {
        let path = error.path().to_string();
        format!("at {path}: {}", error.into_inner())
    })
}

/// The predicate form of a `filter` transform.
///
/// Exactly one of `equal`, `range` and `oneOf` must be present. Kept as flat
/// optional fields rather than a tagged enum so the JSON stays exactly as
/// Vega-Lite writes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FilterExpr {
    /// Column to test.
    pub field: String,

    /// Keep rows whose cell equals this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equal: Option<FilterValue>,

    /// Keep rows whose cell falls inside `[min, max]`, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[FilterValue; 2]>,

    /// Keep rows whose cell equals any of these values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<FilterValue>>,
}

/// A scalar a filter compares against: a number or a string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FilterValue {
    /// A number.
    Number(f64),
    /// Text.
    Text(String),
}

impl FilterValue {
    /// The value as a number, if it is one.
    #[must_use]
    pub const fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Text(_) => None,
        }
    }
}

pub(crate) const fn semantic_name(semantic: SemanticType) -> &'static str {
    match semantic {
        SemanticType::Continuous => "continuous",
        SemanticType::Discrete => "categorical",
        SemanticType::Temporal => "temporal",
    }
}

pub(crate) const fn declared_name(data_type: DataType) -> &'static str {
    match data_type {
        DataType::Quantitative => "quantitative",
        DataType::Nominal => "nominal",
        DataType::Ordinal => "ordinal",
        DataType::Temporal => "temporal",
    }
}

// ---------------------------------------------------------------------------
// Path and message helpers
// ---------------------------------------------------------------------------
