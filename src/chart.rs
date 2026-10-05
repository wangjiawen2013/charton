pub mod area_chart;
pub mod bar_chart;
pub mod box_chart;
pub mod contour_chart;
pub mod density_2d_chart;
pub mod density_chart;
pub mod errorbar_chart;
pub mod geo_chart;
pub mod hist_chart;
pub mod line_chart;
pub mod point_chart;
pub mod rect_chart;
pub mod rule_chart;
pub mod text_chart;
pub mod tick_chart;
pub mod violin_chart;

use crate::TEMP_SUFFIX;
use crate::coordinate::CoordinateTrait;
use crate::core::aesthetics::GlobalAesthetics;
use crate::core::data::{Dataset, FacetPartition, SemanticType, ToDataset};
use crate::core::layer::{Layer, MarkRenderer};
use crate::encode::{Channel, Encoding, IntoEncoding, y::StackMode};
use crate::error::ChartonError;
use crate::mark::{
    Mark, area::MarkArea, bar::MarkBar, boxplot::MarkBoxplot, contour::MarkContour,
    density::MarkDensity, density_2d::MarkDensity2D, errorbar::MarkErrorBar, geo_path::MarkGeoPath,
    histogram::MarkHist, line::MarkLine, no_mark::NoMark, point::MarkPoint, rect::MarkRect,
    rule::MarkRule, text::MarkText, tick::MarkTick, violin::MarkViolin,
};
use crate::scale::{Expansion, Scale, ScaleDomain};
use ahash::AHashMap;
use std::sync::Arc;

/// Generic Chart structure representing a single visualization layer.
///
/// This struct acts as a state machine. It begins in the `NoMark` state where
/// data and visual encodings are defined. It can then be transitioned into a
/// specific chart type (like `Chart<MarkPoint>`) or a faceted view.
///
/// # Type Parameters
///
/// * `T` - The mark type implementing the [Mark] trait. Defaults to `NoMark`,
///   enabling the "Base Chart" pattern similar to Altair.
///
/// # Fields
///
/// * `data` - The underlying data source.
/// * `encoding` - Mapping between data fields and visual channels (x, y, color, etc.).
/// * `mark` - The specific visual mark configuration. Is `None` when `T` is `NoMark`.
#[derive(Clone)]
pub struct Chart<T: Mark = NoMark> {
    pub(crate) data: Dataset,
    pub(crate) encoding: Encoding,
    pub(crate) mark: Option<T>,

    /// The dataset as it was *before* the mark-implied statistics ran.
    ///
    /// Faceted charts use it to re-run the statistic on each panel's own rows,
    /// so a stat never mixes data across panels. It is `None` for charts that
    /// have no statistical transform (or that were built directly as a subset).
    pub(crate) source_data: Option<Dataset>,
}

/// Returns `true` when a mark runs a statistical transform whose result must be
/// recomputed inside each facet panel (binning, stacking, density, ...).
///
/// Marks that only draw the data they are given (line, rule, text, polygon,
/// tick) do not need the extra pre-statistic copy.
fn mark_has_stat(mark_type: &str) -> bool {
    matches!(
        mark_type,
        "point"
            | "boxplot"
            | "errorbar"
            | "rect"
            | "bar"
            | "hist"
            | "area"
            | "violin"
            | "contour"
            | "density"
            | "density_2d"
    )
}

impl Chart<NoMark> {
    /// Create a new base chart instance with the provided Dataset.
    ///
    /// This is the standard entry point for the "Base Chart" pattern. It initializes
    /// a `Chart<NoMark>` which can be configured with encodings and subsequently
    /// converted into specific mark types or faceted.
    ///
    /// # Arguments
    ///
    /// * `source` - Anything that can be converted into a `Dataset`.
    pub fn build<S>(source: S) -> Result<Self, ChartonError>
    where
        S: ToDataset,
    {
        // Convert input (e.g., Vec<f64>, CSV strings) into our internal Dataset
        let dataset = source.to_dataset()?;

        Ok(Self {
            data: dataset,
            encoding: Encoding::new(),
            mark: None,
            source_data: None,
        })
    }

    /// Transitions the base chart into a Point chart.
    ///
    /// This consumes the NoMark chart and returns a `Chart<MarkPoint>`.
    pub fn mark_point(self) -> Result<Chart<MarkPoint>, ChartonError> {
        let chart = Chart::<MarkPoint> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkPoint::default()),
            source_data: self.source_data,
        };

        // If the user called .encode() before .mark_point(),
        // we need to trigger the validation logic here.
        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Line chart.
    ///
    /// A line is one connected curve **per colour group**, with the points
    /// sorted by x. That is the right shape for a time series whose rows may be
    /// out of order. When the row order itself is the curve — a trajectory, a
    /// contour line, a network edge — use [`Chart::mark_path`] instead, which
    /// does not sort and groups by `path_group` rather than colour.
    pub fn mark_line(self) -> Result<Chart<MarkLine>, ChartonError> {
        let chart = Chart::<MarkLine> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkLine::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Bar chart.
    pub fn mark_bar(self) -> Result<Chart<MarkBar>, ChartonError> {
        let chart = Chart::<MarkBar> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkBar::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Area chart.
    pub fn mark_area(self) -> Result<Chart<MarkArea>, ChartonError> {
        let chart = Chart::<MarkArea> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkArea::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Text chart.
    pub fn mark_text(self) -> Result<Chart<MarkText>, ChartonError> {
        let chart = Chart::<MarkText> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkText::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Rule chart.
    pub fn mark_rule(self) -> Result<Chart<MarkRule>, ChartonError> {
        let chart = Chart::<MarkRule> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkRule::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Boxplot chart.
    pub fn mark_boxplot(self) -> Result<Chart<MarkBoxplot>, ChartonError> {
        let chart = Chart::<MarkBoxplot> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkBoxplot::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Violin chart.
    ///
    /// Draws the density of a numeric column as a symmetric outline. `y` is the
    /// value (required), `x` an optional category, and `color` an optional group
    /// that splits a category into side-by-side lanes (dodge) or, with
    /// `configure_violin(..).with_split(true)`, the two halves of one violin.
    ///
    /// ```rust,ignore
    /// chart!(penguins)?
    ///     .mark_violin()?
    ///     .encode((alt::x("Sex"), alt::y("Body Mass (g)"), alt::color("Species")))?;
    /// ```
    pub fn mark_violin(self) -> Result<Chart<MarkViolin>, ChartonError> {
        let chart = Chart::<MarkViolin> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkViolin::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Contour chart.
    ///
    /// Draws iso-lines of the scalar field `z` on the regular grid given by the
    /// `x` and `y` encodings. `z` is named explicitly because the crate has no
    /// `z` channel; it may be a raw column or the `density` column produced by
    /// `transform_density_2d`. The lines are coloured by their level by default;
    /// use `configure_contour(..).with_color_by_level(false)` for one colour.
    ///
    /// ```rust,ignore
    /// chart!(x, y, z)?
    ///     .mark_contour("z")?
    ///     .encode((alt::x("x"), alt::y("y")))?;
    /// ```
    pub fn mark_contour(self, z: impl Into<String>) -> Result<Chart<MarkContour>, ChartonError> {
        let chart = Chart::<MarkContour> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkContour::new(z)),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Density chart.
    ///
    /// Draws the kernel density of the numeric `x` column as a smooth area, with
    /// one curve per `color` group. The estimate is written to the `y` axis.
    ///
    /// ```rust,ignore
    /// chart!(iris)?
    ///     .mark_density()?
    ///     .encode((alt::x("sepal_length"), alt::color("species")))?;
    /// ```
    pub fn mark_density(self) -> Result<Chart<MarkDensity>, ChartonError> {
        let chart = Chart::<MarkDensity> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkDensity::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a 2-D density heatmap.
    ///
    /// Estimates the joint density of the `x` and `y` columns and paints the
    /// grid as heatmap cells, coloured by density. The bin count is set to the
    /// grid size automatically.
    ///
    /// ```rust,ignore
    /// chart!(iris)?
    ///     .mark_density_2d()?
    ///     .encode((alt::x("sepal_length"), alt::y("petal_length")))?;
    /// ```
    pub fn mark_density_2d(self) -> Result<Chart<MarkDensity2D>, ChartonError> {
        let chart = Chart::<MarkDensity2D> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkDensity2D::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Histogram chart.
    pub fn mark_hist(self) -> Result<Chart<MarkHist>, ChartonError> {
        let chart = Chart::<MarkHist> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkHist::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Rect chart.
    pub fn mark_rect(self) -> Result<Chart<MarkRect>, ChartonError> {
        let chart = Chart::<MarkRect> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkRect::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Errorbart chart.
    pub fn mark_errorbar(self) -> Result<Chart<MarkErrorBar>, ChartonError> {
        let chart = Chart::<MarkErrorBar> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkErrorBar::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Geographic Path chart.
    ///
    /// This is the closed, filled form — the geographic name of
    /// [`Chart::mark_polygon`]. Rows sharing a `path_group` are connected in
    /// row order, the loop is closed and the interior is filled. For an **open**
    /// line use [`Chart::mark_path`].
    pub fn mark_geoshape(self) -> Result<Chart<MarkGeoPath>, ChartonError> {
        let chart = Chart::<MarkGeoPath> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkGeoPath::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a polygon chart.
    ///
    /// A polygon is defined by grouping rows with the same `path_group` value
    /// and connecting them in order. The shape is then **closed** (an extra edge
    /// from the last point back to the first) and **filled**. This is the
    /// geometry behind violin outlines, custom shapes, region maps and filled
    /// contour bands. It is the same renderer as [`Chart::mark_geoshape`], under
    /// a domain-neutral name.
    ///
    /// Its open counterpart is [`Chart::mark_path`]: same grouping, but it stops
    /// at the last point and strokes instead of filling. The difference is
    /// exactly the `closed` flag.
    pub fn mark_polygon(self) -> Result<Chart<MarkGeoPath>, ChartonError> {
        self.mark_geoshape()
    }

    /// Transitions the base chart into an **open** path (a polyline).
    ///
    /// Rows sharing a `path_group` are connected in **row order** — no sorting,
    /// no inserted points — and the shape is **not** closed and **not** filled.
    /// This is a line, not a region: trajectories, contour lines, network edges,
    /// parallel coordinates.
    ///
    /// Compare [`Chart::mark_polygon`]: same grouping, but it joins the last
    /// point back to the first and paints the interior. "Trajectory" and
    /// "outline" differ by exactly that.
    pub fn mark_path(self) -> Result<Chart<MarkGeoPath>, ChartonError> {
        let chart = Chart::<MarkGeoPath> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(
                MarkGeoPath::new()
                    .with_closed(false)
                    .with_fill("none")
                    .with_stroke("black")
                    .with_stroke_width(1.0),
            ),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Tick chart.
    pub fn mark_tick(self) -> Result<Chart<MarkTick>, ChartonError> {
        let chart = Chart::<MarkTick> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkTick::default()),
            source_data: self.source_data,
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }
}

impl<T: Mark> Chart<T> {
    /// Apply encoding mappings to the chart.
    ///
    /// This method defines how data fields map to visual properties (channels).
    /// If the chart is in the `NoMark` state, mappings are stored without immediate
    /// validation to allow for late-binding of the mark type.
    ///
    /// If a specific mark type is already assigned (e.g., `Chart<MarkPoint>`), this
    /// method will immediately trigger the validation pipeline, including:
    /// 1. Mandatory channel checks (e.g., Bar charts need X and Y).
    /// 2. Semantic type validation (e.g., Histogram X must be continuous).
    /// 3. Statistical transformations for the mark (binning, aggregation for
    ///    box plots, dodging lanes for points, ...).
    ///
    /// Missing values are not removed here: a null is handled per channel when
    /// scales and marks resolve it, so a missing position drops its row while a
    /// missing colour is kept as the reserved
    /// [`MISSING_CATEGORY`](crate::core::data::MISSING_CATEGORY) level.
    ///
    /// # Arguments
    ///
    /// * `enc` - An encoding specification that implements [IntoEncoding].
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the updated Chart instance.
    pub fn encode<U>(mut self, enc: U) -> Result<Self, ChartonError>
    where
        U: IntoEncoding,
    {
        // 1. Update the internal encoding mappings
        enc.apply(&mut self.encoding);

        // 2. Check if we are in the base (NoMark) state.
        // If so, we defer validation until a specific mark is assigned via .mark_xxx().
        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<NoMark>() {
            return Ok(self);
        }

        // 3. If a specific mark is present, execute the validation and transformation pipeline.
        self.validate_and_transform()
    }

    /// The core validation and data processing pipeline.
    ///
    /// This internal method orchestrates the transformation of raw data into a render-ready state
    /// by following a strict sequence of operations:
    ///
    /// 1. **Identification**: Verifies the mark type and mandatory encoding channels.
    /// 2. **Initial Resolution**: Infers data types for existing source columns.
    /// 3. **Schema Validation**: Ensures source data matches mark requirements.
    /// 4. **Transformation**: Executes mark-specific processing (binning, windows, etc.).
    /// 5. **Final Resolution**: Resolves types for newly generated/transformed columns.
    /// 6. **Visual Refinement**: Applies final aesthetic defaults (zero-baselines, padding).
    pub(crate) fn validate_and_transform(mut self) -> Result<Self, ChartonError> {
        // --- Step 1: Mark Identification ---
        let mark_type = self
            .mark
            .as_ref()
            .map(|m| m.mark_type().to_string())
            .ok_or_else(|| ChartonError::Mark("A mark is required for validation".into()))?;

        // Remember the rows as they are *before* the mark-implied statistic runs.
        // A faceted chart re-runs the statistic on each panel's own subset of
        // these rows, so no statistic can mix data across panels.
        //
        // Only marks that actually run a statistic keep the extra copy; a plain
        // line, rule or text layer continues to hold a single dataset.
        if self.source_data.is_none() && mark_has_stat(&mark_type) {
            self.source_data = Some(self.data.clone());
        }

        // --- Step 2: Mandatory Encoding Validation ---
        self.validate_mandatory_encodings(&mark_type)?;

        // --- Step 3: First Pass Semantic Resolution ---
        // Injects inferred or user-defined Scales into self.encoding
        self.resolve_semantic_types()?;

        // --- Step 4: Scale-to-Mark Validation ---
        // Checks that the mark (for example a bar or a violin) can work with the
        // resolved scale on each channel (for example a discrete x axis).
        self.validate_scale_compatibility(&mark_type)?;

        // --- Step 5: Statistical Transformations ---
        self.resolve_pre_transform_encodings()?;

        match mark_type.as_str() {
            "point" => self = self.transform_point_data()?,
            "boxplot" => self = self.transform_boxplot_data()?,
            "violin" => self = self.transform_violin_data()?,
            "contour" => self = self.transform_contour_data()?,
            "density" => self = self.transform_density_data()?,
            "density_2d" => self = self.transform_density_2d_data()?,
            "errorbar" if self.encoding.y2.is_none() => {
                self = self.transform_errorbar_data()?;
            }
            "rect" => self = self.transform_rect_data()?,
            "bar" => self = self.transform_bar_data()?,
            "hist" => self = self.transform_histogram_data()?,
            "area" => self = self.transform_area_data()?,
            _ => {}
        }

        // --- Step 6: Second Pass Semantic Resolution ---
        // Resolve scales for generated columns (count, ecdf, etc.)
        //
        // NOTE: This pass also acts as the final guardrail for data-scale consistency.
        // If a transform (in Step 5) was forced to coerce data into a Discrete format
        // (e.g., to handle grouping), but the Scale resolved in Step 3/4 remains
        // Continuous (Linear), this will trigger a type mismatch error.
        //
        // This ensures that we never render "dodged" marks (errorbars, or
        // swarm points) on a continuous axis, as shifting their positions would
        // invalidate the numerical meaning of the X-coordinate.
        self.resolve_semantic_types()?;

        // --- Step 7: Visual Refinement ---
        self.apply_visual_defaults()?;

        Ok(self)
    }

    /// Verifies that the required visual channels are present for the chosen mark.
    fn validate_mandatory_encodings(&self, mark_type: &str) -> Result<(), ChartonError> {
        match mark_type {
            "errorbar" | "bar" | "hist" | "line" | "point" | "area" | "boxplot" | "text"
            | "rule" | "tick" | "geo_path" | "contour" => {
                if self.encoding.x.is_none() || self.encoding.y.is_none() {
                    return Err(ChartonError::Encoding(format!(
                        "{} chart requires both x and y encodings",
                        mark_type
                    )));
                }
            }
            "rect" => {
                if self.encoding.x.is_none()
                    || self.encoding.y.is_none()
                    || self.encoding.color.is_none()
                {
                    return Err(ChartonError::Encoding(
                        "Rect chart requires x, y, and color encodings".into(),
                    ));
                }
            }
            "none" => {}
            "violin" => {
                // Only the value column is required; the category and group are
                // optional. x is generated by the recipe.
                if self.encoding.y.is_none() {
                    return Err(ChartonError::Encoding(
                        "violin chart requires a y encoding (the value column)".into(),
                    ));
                }
            }
            "density" => {
                // The value column is required; the density is generated onto y.
                if self.encoding.x.is_none() {
                    return Err(ChartonError::Encoding(
                        "density chart requires an x encoding (the value column)".into(),
                    ));
                }
            }
            "density_2d" => {
                // Both axes are required; the density is generated as colour.
                if self.encoding.x.is_none() || self.encoding.y.is_none() {
                    return Err(ChartonError::Encoding(
                        "density_2d chart requires both x and y encodings".into(),
                    ));
                }
            }
            _ => {
                return Err(ChartonError::Mark(format!(
                    "Unknown mark type: {}",
                    mark_type
                )));
            }
        }
        Ok(())
    }

    /// Infers or validates the semantic scale type (Linear, Discrete, or Temporal)
    /// for all active encoding channels.
    ///
    /// This implementation follows three key principles:
    /// 1. **User Intent First**: If a user manually set a `scale_type`, we respect it.
    /// 2. **Type Safety**: We validate that the data column is compatible with the
    ///    chosen scale (e.g., prevent String data from using a Linear scale).
    /// 3. **Transformation Awareness**: If a field is missing (generated later by
    ///    stats), we skip it for a second pass.
    fn resolve_semantic_types(&mut self) -> Result<(), ChartonError> {
        // A helper to determine the final scale type based on user intent and data reality.
        let resolve_channel_scale = |field: &str,
                                     manual_scale: Option<Scale>|
         -> Result<Option<Scale>, ChartonError> {
            // Handle virtual/placeholder columns
            if field.is_empty() {
                return Ok(Some(Scale::Discrete));
            }

            // If field doesn't exist yet, it might be a generated column (e.g., binning, ecdf).
            // Defer inference to the next pass.
            if !self.data.schema.contains_key(field) {
                return Ok(manual_scale);
            }

            let col = self.data.column(field)?;
            let inferred = match col.semantic_type() {
                SemanticType::Continuous => Scale::Linear,
                SemanticType::Discrete => Scale::Discrete,
                SemanticType::Temporal => Scale::Temporal,
            };

            // --- VALIDATION LOGIC ---
            if let Some(requested) = manual_scale {
                match (col.semantic_type(), &requested) {
                    // ILLEGAL: String/Categorical data cannot be mapped to a continuous mathematical axis.
                    (SemanticType::Discrete, Scale::Linear)
                    | (SemanticType::Discrete, Scale::Log)
                    | (SemanticType::Discrete, Scale::Temporal) => {
                        Err(ChartonError::Encoding(format!(
                            "Field '{}' is categorical (String) and cannot be used with a continuous Scale ({:?}).",
                            field, requested
                        )))
                    }
                    // LEGAL: Numbers can be treated as Discrete categories (e.g., Year 2024 -> "2024").
                    // LEGAL: Temporal data can be treated as Linear (using timestamps) or Discrete.
                    _ => Ok(Some(requested)),
                }
            } else {
                // No user override, use the inferred type from data.
                Ok(Some(inferred))
            }
        };

        // Apply the resolution logic to all active encoding channels.
        // We update the Option<Scale> in place.

        if let Some(ref mut x) = self.encoding.x {
            x.scale_type = resolve_channel_scale(&x.field, x.scale_type)?;
            // A numeric position axis with explicit category labels behaves like
            // a discrete axis: integer ticks, categorical labels.
            if x.category_field.is_some() {
                x.scale_type = Some(Scale::Discrete);
            }
        }

        if let Some(ref mut y) = self.encoding.y {
            y.scale_type = resolve_channel_scale(&y.field, y.scale_type)?;
            if y.category_field.is_some() {
                y.scale_type = Some(Scale::Discrete);
            }
        }

        if let Some(ref mut color) = self.encoding.color {
            color.scale_type = resolve_channel_scale(&color.field, color.scale_type)?;
        }

        if let Some(ref mut size) = self.encoding.size {
            size.scale_type = resolve_channel_scale(&size.field, size.scale_type)?;
        }

        if let Some(ref mut shape) = self.encoding.shape {
            shape.scale_type = resolve_channel_scale(&shape.field, shape.scale_type)?;
        }

        if let Some(ref mut pg) = self.encoding.path_group {
            pg.scale_type = resolve_channel_scale(&pg.field, pg.scale_type)?;
        }

        Ok(())
    }

    /// Helper method to perform the actual validation based on get_expected_scale_types
    fn validate_scale_compatibility(&self, mark_type: &str) -> Result<(), ChartonError> {
        let expectations = self.get_expected_scale_types(mark_type);

        // We iterate through our defined expectations
        for (channel, allowed_scales) in expectations {
            // Use a helper to get the Scale from the encoding (x, y, color, etc.)
            if let Some(actual_scale) = self.encoding.get_scale_by_channel(channel)
                && !allowed_scales.contains(&actual_scale)
            {
                return Err(ChartonError::Encoding(format!(
                    "{} chart expects {:?} scale for channel {:?}, but found {:?}",
                    mark_type, allowed_scales, channel, actual_scale
                )));
            }
        }
        Ok(())
    }

    /// Returns the required Scale types for specific channels based on the mark type.
    /// This ensures the chosen visualization (Mark) is mathematically compatible
    /// with how the data is being projected (Scale).
    fn get_expected_scale_types(&self, mark_type: &str) -> AHashMap<Channel, Vec<Scale>> {
        let mut expected = AHashMap::new();

        // --- GLOBAL CONSTRAINTS ---
        // Shape encoding is fundamentally categorical.
        expected.insert(Channel::Shape, vec![Scale::Discrete]);

        // Size encoding usually maps to a continuous range (area/length).
        expected.insert(
            Channel::Size,
            vec![Scale::Linear, Scale::Log, Scale::Temporal],
        );

        // PathGroup encoding is always categorical (it identifies polygon membership).
        expected.insert(Channel::PathGroup, vec![Scale::Discrete]);

        // --- MARK-SPECIFIC AXIS CONSTRAINTS ---
        match mark_type {
            "bar" | "boxplot" => {
                // Standard Bar/Box: One axis must be discrete (categories),
                // the other must be quantitative (height/value).
                expected.insert(Channel::X, vec![Scale::Discrete]);
                expected.insert(Channel::Y, vec![Scale::Linear, Scale::Log, Scale::Temporal]);
            }
            "hist" => {
                // Histograms require a quantitative X-axis to perform binning.
                expected.insert(Channel::X, vec![Scale::Linear, Scale::Log, Scale::Temporal]);
                // Y is usually the generated 'count' (Linear).
                expected.insert(Channel::Y, vec![Scale::Linear]);
            }
            "rect" => {
                // Rect/Heatmap: X and Y can be anything,
                // but the Color channel typically represents a magnitude.
                expected.insert(
                    Channel::Color,
                    vec![Scale::Linear, Scale::Log, Scale::Temporal],
                );
            }
            "line" | "area" => {
                // Lines/Areas usually represent trends over time or continuous intervals.
                expected.insert(
                    Channel::X,
                    vec![Scale::Linear, Scale::Temporal, Scale::Discrete],
                );
                expected.insert(Channel::Y, vec![Scale::Linear, Scale::Log]);
            }
            "errorbar" | "rule" => {
                // Rules and Error bars are geometric intervals.
                expected.insert(Channel::Y, vec![Scale::Linear, Scale::Log, Scale::Temporal]);
            }
            "text" => {
                // Text marks usually just need a position.
                // The Label itself doesn't have a scale, but the X/Y do.
                expected.insert(
                    Channel::X,
                    vec![Scale::Linear, Scale::Discrete, Scale::Temporal],
                );
                expected.insert(
                    Channel::Y,
                    vec![Scale::Linear, Scale::Discrete, Scale::Temporal],
                );
            }
            "geo_path" | "violin" => {
                // Geo paths: X and Y are continuous by default (longitude/latitude).
                // X also accepts a discrete position axis so a polygon can carry
                // integer categories (used by violin plots).
                expected.insert(
                    Channel::X,
                    vec![Scale::Linear, Scale::Temporal, Scale::Discrete],
                );
                expected.insert(Channel::Y, vec![Scale::Linear, Scale::Temporal]);
                // Color is typically a continuous magnitude for choropleth.
                expected.insert(
                    Channel::Color,
                    vec![Scale::Linear, Scale::Log, Scale::Discrete],
                );
            }
            "contour" => {
                // The contour grid is numeric on both axes.
                expected.insert(Channel::X, vec![Scale::Linear, Scale::Log, Scale::Temporal]);
                expected.insert(Channel::Y, vec![Scale::Linear, Scale::Log, Scale::Temporal]);
            }
            "density" => {
                // The value column must be quantitative to estimate a density.
                expected.insert(Channel::X, vec![Scale::Linear, Scale::Log, Scale::Temporal]);
            }
            "density_2d" => {
                // Both axes must be quantitative to estimate a joint density.
                expected.insert(Channel::X, vec![Scale::Linear, Scale::Log, Scale::Temporal]);
                expected.insert(Channel::Y, vec![Scale::Linear, Scale::Log, Scale::Temporal]);
            }
            _ => {}
        }

        expected
    }

    /// Resolves binning configuration required before data transformation.
    ///
    /// For marks that require data aggregation (like histograms or heatmaps),
    /// this method calculates the optimal number of bins if not explicitly
    /// provided by the user.
    fn resolve_pre_transform_encodings(&mut self) -> Result<(), ChartonError> {
        // Access the mark type to determine if binning is applicable.
        let mt = self.mark.as_ref().unwrap().mark_type();

        // Only "rect" (heatmaps) and "hist" (histograms) require pre-transform binning.
        if !["rect", "hist"].contains(&mt) {
            return Ok(());
        }

        // Safely extract mutable references to X and Y encodings.
        let x_enc = self.encoding.x.as_mut().ok_or(ChartonError::Encoding(
            "X encoding is required for binned marks".to_string(),
        ))?;
        let y_enc = self.encoding.y.as_mut().ok_or(ChartonError::Encoding(
            "Y encoding is required for binned marks".to_string(),
        ))?;

        // Helper closure to calculate bin count based on data semantics and unique value distribution.
        let calculate_bins = |field: &str| -> Result<usize, ChartonError> {
            let series = self.data.column(field)?;
            let unique_count = series.n_unique();

            // Determine bins based on the semantic interpretation of the column data.
            match series.semantic_type() {
                SemanticType::Continuous | SemanticType::Temporal => {
                    if unique_count <= 1 {
                        Ok(1)
                    } else {
                        // Use the Square-root choice rule for automatic binning,
                        // constrained between a reasonable range (5 to 50) for visualization.
                        let suggested = (unique_count as f64).sqrt() as usize;
                        Ok(suggested.clamp(5, 50))
                    }
                }
                // For discrete data (categories), each unique value typically gets its own bin.
                SemanticType::Discrete => Ok(unique_count),
            }
        };

        // --- RESOLVE X-AXIS BINS ---
        if x_enc.bins.is_none() {
            x_enc.bins = Some(calculate_bins(&x_enc.field)?);
        }

        // --- RESOLVE Y-AXIS BINS ---
        // Y-axis binning is only necessary for 2D density plots (rect/heatmap).
        // For standard histograms, Y is the resulting count/frequency.
        if mt == "rect" && y_enc.bins.is_none() {
            y_enc.bins = Some(calculate_bins(&y_enc.field)?);
        }

        Ok(())
    }

    /// Refines visual properties like axis baselines and padding after data
    /// transformations are complete.
    fn apply_visual_defaults(&mut self) -> Result<(), ChartonError> {
        let mt = self.mark.as_ref().unwrap().mark_type();

        // We ensure x and y exist; unwrap is safe due to Mandatory Encoding Validation step.
        let x_enc = self.encoding.x.as_mut().unwrap();
        let y_enc = self.encoding.y.as_mut().unwrap();

        // --- 1. STATISTICAL INTEGRITY & MAGNITUDE BASELINES ---
        // Marks representing magnitude (Bar, Area, Hist) should generally start
        // at zero. A *floating* bar does not: with a `y2` bound it spans an
        // interval (a candlestick body, a waterfall step), and forcing zero
        // would squash that interval against the axis.
        let is_floating_bar = mt == "bar" && self.encoding.y2.is_some();
        if y_enc.scale_type == Some(Scale::Linear)
            && ["area", "bar", "hist", "density"].contains(&mt)
            && !is_floating_bar
        {
            // Force zero baseline unless the user explicitly disabled it.
            if y_enc.zero.is_none() {
                y_enc.zero = Some(true);
            }

            // PIE MODE DETECTION: An empty X field implies a radial projection of the Y axis.
            let is_pie_mode = x_enc.field.is_empty();

            // A mirrored (violin) or centered (streamgraph) area is symmetric
            // about the baseline rather than a magnitude that grows away from
            // it. Its domain is trained from the stacked bounds (which reach
            // below zero), so it needs padding on *both* ends: otherwise the
            // mark sits flush against the axis, as a lone violin does.
            let is_symmetric_stack =
                mt == "area" && matches!(y_enc.stack, StackMode::Mirror | StackMode::Center);

            // Calculate directional expansion based on data bounds.
            if let Ok(y_col) = self.data.column(&y_enc.field) {
                let (y_min, y_max) = y_col.min_max();

                if y_enc.expansion.is_none() {
                    y_enc.expansion = Some(if is_pie_mode {
                        // Force zero expansion for Pie charts to prevent "cracks" in the circle.
                        Expansion {
                            mult: (0.0, 0.0),
                            add: (0.0, 0.0),
                        }
                    } else if is_symmetric_stack {
                        // Symmetric buffer on both ends for mirrored bands.
                        Expansion::default()
                    } else if y_min >= 0.0 {
                        // Buffer at the top for positive distributions (5% mult).
                        Expansion {
                            mult: (0.0, 0.05),
                            add: (0.0, 0.0),
                        }
                    } else if y_max <= 0.0 {
                        // Buffer at the bottom for negative distributions (5% mult).
                        Expansion {
                            mult: (0.05, 0.0),
                            add: (0.0, 0.0),
                        }
                    } else {
                        // Default padding for data crossing zero (usually 5% on both ends).
                        Expansion::default()
                    });
                }
            }
        }

        // --- 2. HALF-STEP PADDING FOR DISCRETE AXES ---
        // Categorical marks with thickness (Bar, Boxplot, Rect) need 0.5 units of padding
        // to center the marks and prevent them from clipping against axis lines.
        let needs_discrete_padding = ["bar", "boxplot", "rect", "geo_path", "violin"].contains(&mt);
        if needs_discrete_padding {
            if x_enc.scale_type == Some(Scale::Discrete) && x_enc.expansion.is_none() {
                x_enc.expansion = Some(Expansion {
                    mult: (0.0, 0.0),
                    add: (0.5, 0.5),
                });
            }
            if y_enc.scale_type == Some(Scale::Discrete) && y_enc.expansion.is_none() {
                y_enc.expansion = Some(Expansion {
                    mult: (0.0, 0.0),
                    add: (0.5, 0.5),
                });
            }
        }

        // --- 3. FLUSH CONTINUOUS RECTANGLES (HEATMAPS) ---
        // Heatmaps on continuous scales should touch the edges of the plotting area.
        if mt == "rect" || mt == "density_2d" {
            if x_enc.scale_type != Some(Scale::Discrete) && x_enc.expansion.is_none() {
                x_enc.expansion = Some(Expansion {
                    mult: (0.0, 0.0),
                    add: (0.0, 0.0),
                });
            }
            if y_enc.scale_type != Some(Scale::Discrete) && y_enc.expansion.is_none() {
                y_enc.expansion = Some(Expansion {
                    mult: (0.0, 0.0),
                    add: (0.0, 0.0),
                });
            }
        }

        Ok(())
    }
}

// Implementation of Layer trait for Chart<T> allowing any chart to be used as a layer.
// This follows the "Composition over Inheritance" principle.
impl<T> Layer for Chart<T>
where
    T: crate::mark::Mark + Send + Sync,
    Chart<T>: MarkRenderer + Clone,
{
    /// Determines if this specific layer needs coordinate axes.
    fn requires_axes(&self) -> bool {
        // Aesthetic rule: Pie charts (MarkArc) don't use standard Cartesian axes.
        self.mark.as_ref().map(|m| m.mark_type()) != Some("arc")
    }

    /// Retrieves the field name for a specific channel.
    /// Redirects to the central Encoding container.
    fn get_field(&self, channel: Channel) -> Option<String> {
        self.encoding
            .get_field_by_channel(channel)
            .map(|s| s.to_string())
    }

    /// Retrieves the display label for a specific channel, if the encoding set
    /// one.
    fn get_label(&self, channel: Channel) -> Option<String> {
        self.encoding
            .get_label_by_channel(channel)
            .map(|s| s.to_string())
    }

    /// Returns a reference to the underlying Dataset.
    ///
    /// This provides direct access to the layer's data for faceting operations,
    /// such as extracting unique facet values and filtering data by panel.
    fn get_dataset(&self) -> &Dataset {
        // Facet factors must be read from the data *before* the statistic ran,
        // because a statistic may drop or re-shape the columns it consumes.
        self.source_data.as_ref().unwrap_or(&self.data)
    }

    /// Retrieves user-configured scale types (e.g., Linear vs Log).
    fn get_scale(&self, channel: Channel) -> Option<Scale> {
        self.encoding.get_scale_by_channel(channel)
    }

    /// Retrieves padding/expansion preferences.
    fn get_expand(&self, channel: Channel) -> Option<Expansion> {
        self.encoding.get_expand_by_channel(channel)
    }

    /// Whether this layer asked for a reversed positional axis.
    fn get_reverse(&self, channel: Channel) -> bool {
        self.encoding.get_reverse_by_channel(channel)
    }

    /// Calculates the raw data boundaries for any visual channel.
    ///
    /// This unified implementation supports:
    /// 1. **Standard Encodings**: Simple 1-to-1 mappings (e.g., x, y).
    /// 2. **Explicit Intervals**: Mappings with secondary fields (e.g., y2).
    /// 3. **Implicit Intervals**: Statistical transforms that generate hidden columns
    ///    (e.g., __charton_temp_{field}_min/max for ErrorBars or Area charts).
    fn get_data_bounds(&self, channel: Channel) -> Result<ScaleDomain, ChartonError> {
        // Determine which data field is mapped to this visual channel (X, Y, Color, etc.)
        let field_name = self.encoding.get_field_by_channel(channel).ok_or_else(|| {
            ChartonError::Data(format!("No field mapped to channel {:?}", channel))
        })?;

        let primary_series = self.data.column(field_name)?;

        // A discrete position axis can take its categories from a separate
        // column while `field_name` holds numeric positions. When that is the
        // case the axis domain is the category list, not the positions.
        if let Some(category_field) = self.encoding.get_category_field_by_channel(channel) {
            let categories = self.data.column(category_field)?;
            return Ok(ScaleDomain::Discrete(categories.unique_values()));
        }

        // --- Determine the active scale type (User Override > Inferred) ---
        // This ensures if a user sets alt::color("year").scale_type(Scale::Discrete),
        // we treat it as Discrete here.
        let active_scale = self.encoding.get_scale_by_channel(channel).ok_or_else(|| {
            ChartonError::Internal(format!(
                "Scale type for channel {:?} must be resolved before calling get_data_bounds",
                channel
            ))
        })?;

        match active_scale {
            // --- DISCRETE DOMAIN ---
            // Triggered if Scale is Discrete (even if data is numeric).
            Scale::Discrete => {
                // Colour is non-positional: a missing value is kept as the
                // reserved grey level (placed last) instead of being dropped.
                // Positional channels drop missing values, so they use the
                // plain list.
                let mut labels = if channel == Channel::Color {
                    primary_series.labels_with_missing()
                } else {
                    primary_series.unique_values()
                };
                // Define the exact internal tags used during data transformation for boxplot.
                let boundary_tag = format!("{}_boundary", TEMP_SUFFIX);
                let default_tag = format!("{}_default", TEMP_SUFFIX);

                // Precise Filtering: Remove only the exact internal markers.
                // This ensures that system-injected categories remain invisible to the
                // user-facing components like Legends and Axis labels.
                labels.retain(|l| l != &boundary_tag && l != &default_tag);
                Ok(ScaleDomain::Discrete(labels))
            }

            // --- TEMPORAL DOMAIN ---
            // Triggered if Scale is Temporal. Returns a range (min, max) in Unix timestamps.
            Scale::Temporal => {
                let (min_ts, max_ts) = primary_series.min_max();
                Ok(ScaleDomain::Temporal(min_ts as i64, max_ts as i64))
            }

            // --- CONTINUOUS DOMAIN (Linear, Log, Sqrt, etc.) ---
            _ => {
                let mut global_min = f64::INFINITY;
                let mut global_max = f64::NEG_INFINITY;
                let mut found_data = false;

                let mark_type = self.mark.as_ref().map(|m| m.mark_type());
                let is_area = matches!(mark_type, Some("area"));
                let is_errorbar = matches!(mark_type, Some("errorbar"));
                let is_boxplot = matches!(mark_type, Some("boxplot"));

                // --- STEP 1: Priority Check for Pre-computed Columns (Area & ErrorBar & Boxplot) ---
                if (is_area || is_errorbar || is_boxplot) && channel == Channel::Y {
                    let y_field = &self.encoding.y.as_ref().unwrap().field;
                    let temp_min_col = format!("{}_{}_min", TEMP_SUFFIX, y_field);
                    let temp_max_col = format!("{}_{}_max", TEMP_SUFFIX, y_field);

                    for col_name in [&temp_min_col, &temp_max_col] {
                        if let Ok(series) = self.data.column(col_name) {
                            let (m_min, m_max) = series.min_max();
                            if !m_min.is_nan() {
                                global_min = global_min.min(m_min);
                                found_data = true;
                            }
                            if !m_max.is_nan() {
                                global_max = global_max.max(m_max);
                                found_data = true;
                            }
                        }
                    }
                }

                // --- STEP 2: Fallback to Dynamic Stacking or Standard Scan ---
                if !found_data {
                    let is_y_stacked = channel == Channel::Y
                        && self
                            .encoding
                            .y
                            .as_ref()
                            .is_some_and(|e| e.stack != StackMode::None)
                        && self.encoding.color.is_some();

                    if is_y_stacked {
                        let x_field = &self.encoding.x.as_ref().unwrap().field;
                        let y_field = &self.encoding.y.as_ref().unwrap().field;

                        let x_series = self.data.column(x_field)?;
                        let y_series = self.data.column(y_field)?;

                        let mut stacks: AHashMap<String, f64> = AHashMap::new();
                        for i in 0..x_series.len() {
                            if let (Some(x_val), Some(y_val)) =
                                (x_series.get(i).to_string(), y_series.get(i).to_f64())
                            {
                                let entry = stacks.entry(x_val).or_insert(0.0);
                                *entry += y_val;
                            }
                        }

                        for &sum in stacks.values() {
                            global_min = global_min.min(sum);
                            global_max = global_max.max(sum);
                            found_data = true;
                        }
                    } else {
                        let mut columns_to_scan = Vec::new();
                        columns_to_scan.push(field_name.to_string());

                        if channel == Channel::Y
                            && let Some(y2_enc) = &self.encoding.y2
                        {
                            columns_to_scan.push(y2_enc.field.clone());
                        }

                        for col_name in &columns_to_scan {
                            if let Ok(series) = self.data.column(col_name) {
                                let (m_min, m_max) = series.min_max();
                                if !m_min.is_nan() {
                                    global_min = global_min.min(m_min);
                                    found_data = true;
                                }
                                if !m_max.is_nan() {
                                    global_max = global_max.max(m_max);
                                    found_data = true;
                                }
                            }
                        }
                    }
                }

                // --- STEP 3: Final Fallbacks and Scale Adjustments ---
                if !found_data {
                    let (p_min, p_max) = primary_series.min_max();
                    global_min = p_min;
                    global_max = p_max;
                }

                // --- HALF-BIN COMPENSATION ---
                let bins = match channel {
                    Channel::X => self.encoding.x.as_ref().and_then(|e| e.bins),
                    Channel::Y => self.encoding.y.as_ref().and_then(|e| e.bins),
                    _ => None,
                };

                if let Some(n_bins) = bins
                    && n_bins > 1
                    && global_max > global_min
                {
                    let bin_width = (global_max - global_min) / (n_bins as f64 - 1.0);
                    global_min -= bin_width / 2.0;
                    global_max += bin_width / 2.0;
                }

                // --- ZERO BASELINE ---
                if self.encoding.get_zero_by_channel(channel) {
                    global_min = global_min.min(0.0);
                    global_max = global_max.max(0.0);
                }

                Ok(ScaleDomain::Continuous(global_min, global_max))
            }
        }
    }

    /// Injects resolved scales into the Optional encoding channels.
    ///
    /// This method traverses each defined visual channel (X, Y, Color, etc.)
    /// and populates their internal `RwLock<Option<Arc<dyn ScaleTrait>>>` with
    /// the results from the global resolution phase.
    fn inject_resolved_scales(
        &self,
        coord: Arc<dyn CoordinateTrait>,
        aesthetics: &GlobalAesthetics,
    ) {
        // 1. Inject Position Scales (X & Y)
        // We only inject if the channel was actually configured by the user.
        if let Some(ref x_enc) = self.encoding.x
            && let Ok(mut guard) = x_enc.resolved_scale.0.write()
        {
            *guard = Some(coord.get_x_arc());
        }

        if let Some(ref y_enc) = self.encoding.y
            && let Ok(mut guard) = y_enc.resolved_scale.0.write()
        {
            *guard = Some(coord.get_y_arc());
        }

        // 2. Inject Aesthetic Scales (Color, Shape, Size)
        // We perform a "Field Match" check to ensure the global scale matches this layer's intent.

        // --- Color Channel ---
        // Use .as_ref() to match against a reference instead of moving the value
        if let (Some(enc), Some(map)) = (self.encoding.color.as_ref(), aesthetics.color.as_ref())
            && enc.field == map.field
            && let Ok(mut guard) = enc.resolved_scale.0.write()
        {
            *guard = Some(map.scale_impl.clone());
        }

        // --- Shape Channel ---
        if let (Some(enc), Some(map)) = (self.encoding.shape.as_ref(), aesthetics.shape.as_ref())
            && enc.field == map.field
            && let Ok(mut guard) = enc.resolved_scale.0.write()
        {
            *guard = Some(map.scale_impl.clone());
        }

        // --- Size Channel ---
        if let (Some(enc), Some(map)) = (self.encoding.size.as_ref(), aesthetics.size.as_ref())
            && enc.field == map.field
            && let Ok(mut guard) = enc.resolved_scale.0.write()
        {
            *guard = Some(map.scale_impl.clone());
        }
    }

    /// Partitions this chart's dataset by the given facet `fields`.
    ///
    /// See [`Layer::facet_partition`].
    /// A field missing from the dataset is reported as an error.
    fn facet_partition(&self, fields: &[&str]) -> Result<Option<FacetPartition>, ChartonError> {
        // Partition the rows *before* the statistic, so the panel subset can be
        // re-transformed. A statistic may drop the very column we facet on.
        let source = self.source_data.as_ref().unwrap_or(&self.data);

        // A facet field missing from this layer is almost always a typo, so
        // report it instead of silently leaving the panel unfiltered.
        for field in fields {
            if !source.schema.contains_key(*field) {
                return Err(ChartonError::Data(format!(
                    "Facet field '{}' not found in the layer's dataset",
                    field
                )));
            }
        }

        Ok(Some(source.partition_by(fields)?))
    }

    /// Returns a copy of this chart containing only `rows`, preserving the mark
    /// and encodings so the subset renders with the same resolved scales as the
    /// full layer.
    ///
    /// See [`Layer::subset_rows`].
    fn subset_rows(&self, rows: &[usize]) -> Result<Option<Arc<dyn Layer>>, ChartonError> {
        // When a statistic was applied, the panel must see the rows *before*
        // that statistic and re-run it. Otherwise a cumulative stat (stacking,
        // normalisation, ...) would keep the values it computed on the whole
        // dataset instead of the panel's own rows.
        let base = self.source_data.as_ref().unwrap_or(&self.data);
        let data = base.take_rows(rows)?;

        let subset = Chart {
            data,
            encoding: self.encoding.clone(),
            mark: self.mark.clone(),
            source_data: None,
        };

        // Re-apply the mark-implied statistic on the panel subset. The resolved
        // scales are injected later by the engine, exactly as for a full layer.
        let subset = if self.source_data.is_some() {
            subset.validate_and_transform()?
        } else {
            subset
        };

        Ok(Some(Arc::new(subset)))
    }

    /// Returns this layer with its pre-statistic snapshot released.
    ///
    /// See [`Layer::without_source_data`].
    fn without_source_data(&self) -> Arc<dyn Layer> {
        let mut released = self.clone();
        released.source_data = None;
        Arc::new(released)
    }
}
