//! Quartile-box geometry: the inter-quartile box and median as polygons.
//!
//! Given a numeric column, this transform reduces each `(category, group)` cell
//! to its quartiles and emits two closed polygons: the inter-quartile box (from
//! q1 to q3) and a thin median bar. The result is an ordinary
//! `(x, y, path_group)` table, so the box can be layered with anything else
//! that shares its scales.
//!
//! The "raincloud" is the clearest use — three views of one distribution, each
//! a layer:
//!
//! ```text
//!     transform_density + transform_band   →  the density outline
//!     transform_quantile_box               →  the box and median
//!     mark_point                           →  the raw observations
//! ```
//!
//! This is the composable counterpart of the `mark_boxplot` mark: the mark draws
//! a finished box plot (whiskers, outliers, median) by itself, whereas this
//! transform hands you the geometry so it can sit among other layers. The lane
//! layout is shared with [`crate::transform::band_transform`], so a box always
//! stays centred over its band.

use super::lane_layout::build_lane_layout;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, get_quantile};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::position::Position;

/// Configuration for [`Chart::transform_quantile_box`].
#[derive(Debug, Clone)]
pub struct QuantileBoxTransform {
    /// Numeric column holding the measurements.
    pub(crate) value: String,

    /// Optional categorical column giving each box's x position.
    pub(crate) category: Option<String>,

    /// Optional categorical column splitting the category into lanes.
    pub(crate) group: Option<String>,

    /// Names of the produced columns: `[x, y, path_group]`.
    pub(crate) as_: [String; 3],

    /// How several lanes inside one category are arranged.
    pub(crate) position: Position,

    /// Total width of one category's box group, in category steps.
    pub(crate) span: f64,

    /// The widest a single box lane may be, in category steps.
    pub(crate) max_width: f64,

    /// Box width as a fraction of the resolved lane (0.0–1.0).
    pub(crate) box_width: f64,
}

impl QuantileBoxTransform {
    /// Creates a box transform for `value`.
    ///
    /// Defaults: one centred box, a span of `0.7`, a maximum width of `0.5`
    /// and a box width of `0.2` of the lane — the violin's inner box.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            category: None,
            group: None,
            as_: ["x".to_string(), "y".to_string(), "path_group".to_string()],
            position: Position::Identity,
            span: 0.7,
            max_width: 0.5,
            box_width: 0.2,
        }
    }

    /// Sets the categorical column that gives each box its x position.
    pub fn with_category(mut self, field: impl Into<String>) -> Self {
        self.category = Some(field.into());
        self
    }

    /// Sets the categorical column that splits a category into lanes.
    pub fn with_group(mut self, field: impl Into<String>) -> Self {
        self.group = Some(field.into());
        self
    }

    /// Renames the produced `[x, y, path_group]` columns.
    pub fn with_as(
        mut self,
        x: impl Into<String>,
        y: impl Into<String>,
        path_group: impl Into<String>,
    ) -> Self {
        self.as_ = [x.into(), y.into(), path_group.into()];
        self
    }

    /// Chooses how several boxes inside one category are arranged.
    pub const fn with_position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Sets the total width of a category's box group, in category steps.
    pub const fn with_span(mut self, span: f64) -> Self {
        self.span = span.clamp(0.0, 1.0);
        self
    }

    /// Sets the maximum width of a single box lane, in category steps.
    pub const fn with_width(mut self, width: f64) -> Self {
        self.max_width = width.clamp(0.0, 1.0);
        self
    }

    /// Sets the box width as a fraction of the resolved lane (0.0–1.0).
    pub const fn with_box_width(mut self, width: f64) -> Self {
        self.box_width = width.clamp(0.0, 1.0);
        self
    }
}

impl<T: Mark> Chart<T> {
    /// Rewrites the dataset into inter-quartile box and median polygons.
    ///
    /// ```rust,ignore
    /// chart!(ds)?
    ///     .transform_quantile_box(
    ///         QuantileBoxTransform::new("score")
    ///             .with_category("category")
    ///             .with_group("group")
    ///             .with_position(Position::dodge()),
    ///     )?
    ///     .mark_polygon()?
    ///     .encode((
    ///         alt::x("x").with_category_labels("category"),
    ///         alt::y("y"),
    ///         alt::path_group("path_group"),
    ///     ))?;
    /// ```
    pub fn transform_quantile_box(
        mut self,
        params: QuantileBoxTransform,
    ) -> Result<Self, ChartonError> {
        let value_col = self.data.column(&params.value)?;

        // Group the raw observations by `(category, lane)` and solve the lanes,
        // exactly as `transform_band` does, so boxes sit over their bands.
        let layout = build_lane_layout(
            &self.data,
            params.category.as_deref(),
            params.group.as_deref(),
            &params.position,
            params.span,
            params.max_width,
            false,
            |i| {
                let value = value_col.get(i).to_f64()?;
                value.is_finite().then_some(value)
            },
        )?;

        let box_half = (layout.slot * params.box_width / 2.0).max(0.0);
        // A median is a very thin rectangle so it can be a polygon too.
        let median_half = (layout.slot * 0.02 / 2.0).max(0.0);

        let mut out_x = Vec::new();
        let mut out_y = Vec::new();
        let mut out_id = Vec::new();
        let mut out_part = Vec::new();
        let mut out_category = Vec::new();
        let mut out_group = Vec::new();

        for cell in &layout.cells {
            let center = layout.center(cell);
            let mut values = cell.values.clone();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let q1 = get_quantile(&values, 0.25);
            let median = get_quantile(&values, 0.5);
            let q3 = get_quantile(&values, 0.75);

            let mut push = |x: f64, y: f64, part: &str, id: &str| {
                out_x.push(x);
                out_y.push(y);
                out_id.push(id.to_string());
                out_part.push(part.to_string());
                out_category.push(cell.category_label.clone());
                out_group.push(cell.group_label.clone());
            };

            let base = format!("{}::{}", cell.category_label, cell.group_label);

            // The inter-quartile box: a thin rectangle from q1 to q3.
            let box_id = format!("{base}::box");
            push(center - box_half, q1, "box", &box_id);
            push(center + box_half, q1, "box", &box_id);
            push(center + box_half, q3, "box", &box_id);
            push(center - box_half, q3, "box", &box_id);
            push(center - box_half, q1, "box", &box_id);

            // The median: a very thin rectangle so it can be a polygon too.
            let median_id = format!("{base}::median");
            push(
                center - box_half,
                median - median_half,
                "median",
                &median_id,
            );
            push(
                center + box_half,
                median - median_half,
                "median",
                &median_id,
            );
            push(
                center + box_half,
                median + median_half,
                "median",
                &median_id,
            );
            push(
                center - box_half,
                median + median_half,
                "median",
                &median_id,
            );
            push(
                center - box_half,
                median - median_half,
                "median",
                &median_id,
            );
        }

        let mut new_data = Dataset::new();
        new_data.add_column(
            &params.as_[0],
            ColumnVector::Float64 {
                data: out_x,
                validity: None,
            },
        )?;
        new_data.add_column(
            &params.as_[1],
            ColumnVector::Float64 {
                data: out_y,
                validity: None,
            },
        )?;
        new_data.add_column(
            &params.as_[2],
            ColumnVector::String {
                data: out_id,
                validity: None,
            },
        )?;
        new_data.add_column(
            "box_part",
            ColumnVector::String {
                data: out_part,
                validity: None,
            },
        )?;
        if let Some(field) = &params.category {
            new_data.add_column(
                field,
                ColumnVector::String {
                    data: out_category,
                    validity: None,
                },
            )?;
        }
        if let Some(field) = &params.group {
            new_data.add_column(
                field,
                ColumnVector::String {
                    data: out_group,
                    validity: None,
                },
            )?;
        }

        self.data = new_data;
        Ok(self)
    }
}
