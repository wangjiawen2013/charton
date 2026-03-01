pub mod area_chart;
pub mod bar_chart;
pub mod box_chart;
pub mod errorbar_chart;
pub mod hist_chart;
pub mod line_chart;
pub mod point_chart;
pub mod rect_chart;
pub mod rule_chart;
pub mod text_chart;

use crate::TEMP_SUFFIX;
use crate::coordinate::CoordinateTrait;
use crate::core::aesthetics::GlobalAesthetics;
use crate::core::data::*;
use crate::core::layer::{Layer, MarkRenderer};
use crate::encode::{Channel, Encoding, IntoEncoding};
use crate::error::ChartonError;
use crate::mark::{
    Mark, area::MarkArea, bar::MarkBar, boxplot::MarkBoxplot, errorbar::MarkErrorBar,
    histogram::MarkHist, line::MarkLine, no_mark::NoMark, point::MarkPoint, rect::MarkRect,
    rule::MarkRule, text::MarkText,
};
use crate::scale::{Expansion, Scale, ScaleDomain};
use polars::prelude::*;
use std::sync::Arc;

/// Generic Chart structure representing a single visualization layer.
///
/// This struct acts as a state machine. It begins in the [NoMark] state where
/// data and visual encodings are defined. It can then be transitioned into a
/// specific chart type (like `Chart<MarkPoint>`) or a faceted view.
///
/// # Type Parameters
///
/// * `T` - The mark type implementing the [Mark] trait. Defaults to [NoMark],
///         enabling the "Base Chart" pattern similar to Altair.
///
/// # Fields
///
/// * `data` - The underlying data source (normalized to f64 for numeric columns).
/// * `encoding` - Mapping between data fields and visual channels (x, y, color, etc.).
/// * `mark` - The specific visual mark configuration. Is `None` when `T` is [NoMark].
pub struct Chart<T: Mark = NoMark> {
    pub(crate) data: DataFrameSource,
    pub(crate) encoding: Encoding,
    pub(crate) mark: Option<T>,
}

/// Manual Clone for the Base Chart.
/// We only need to implement this for NoMark to support the "Base Chart" pattern.
impl Clone for Chart<NoMark> {
    fn clone(&self) -> Self {
        Self {
            // DataFrameSource usually contains an Arc<DataFrame>, so this is shallow and fast.
            data: self.data.clone(),

            // Encoding now derives Clone successfully thanks to our ResolvedScale wrapper.
            encoding: self.encoding.clone(),

            // NoMark is always None.
            mark: None,
        }
    }
}

impl Chart<NoMark> {
    /// Create a new base chart instance with the provided data source.
    ///
    /// This is the standard entry point for the "Base Chart" pattern. It initializes
    /// a `Chart<NoMark>` which can be configured with encodings and subsequently
    /// converted into specific mark types or faceted.
    ///
    /// # Arguments
    ///
    /// * `source` - Anything that can be converted into a `DataFrameSource`.
    ///
    /// # Example
    ///
    /// ```
    /// let df = df!["x" => [1, 2], "y" => [3, 4]]?;
    /// // Returns a Chart<NoMark>
    /// let base = Chart::build(&df)?;
    /// ```
    pub fn build<S>(source: S) -> Result<Self, ChartonError>
    where
        S: IntoChartonSource,
    {
        let source = source.into_source()?;

        let mut chart = Self {
            data: source,
            encoding: Encoding::new(),
            mark: None,
        };

        // Standardize numeric columns to f64 for consistent scale calculation
        chart.data = convert_numeric_types(chart.data.clone())?;

        Ok(chart)
    }

    /// Transitions the base chart into a Point chart.
    ///
    /// This consumes the NoMark chart and returns a Chart<MarkPoint>.
    pub fn mark_point(self) -> Result<Chart<MarkPoint>, ChartonError> {
        let chart = Chart::<MarkPoint> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkPoint::default()),
        };

        // If the user called .encode() before .mark_point(),
        // we need to trigger the validation logic here.
        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    /// Transitions the base chart into a Line chart.
    pub fn mark_line(self) -> Result<Chart<MarkLine>, ChartonError> {
        let chart = Chart::<MarkLine> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkLine::default()),
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
        };

        if !chart.encoding.is_empty() {
            return chart.validate_and_transform();
        }

        Ok(chart)
    }

    // Creates a faceted view of the chart based on a specific data field.
    //
    // Faceting (also known as small multiples) splits the data into multiple subsets
    // based on the unique values of the provided `field`, creating a grid of sub-charts.
    //
    // Since faceting is a structural transformation of the data rather than a visual
    // mark property, this method is defined on the base [Chart<NoMark>]. This allows
    // you to define global encodings once and then apply a specific mark to all facets.
    //
    // # Arguments
    //
    // * `field` - The name of the column in the DataFrame to use for partitioning the data.
    //
    // # Returns
    //
    // Returns a `Result` containing a `FacetChart` or a `ChartonError` if the field
    // does not exist in the data source.
    //
    // # Example
    //
    //pub fn facet(self, _field: &str) -> Result<FacetChart, ChartonError> {
    // TODO: Implement the FacetChart structure and partitioning logic.
    // This will likely involve grouping the Polars DataFrame and
    // mapping each group to a sub-chart layer.
    //Err(ChartonError::NotImplemented("Faceting is not yet implemented".into()))
    //}
}

impl<T: Mark> Chart<T> {
    /// Apply encoding mappings to the chart.
    ///
    /// This method defines how data fields map to visual properties (channels).
    /// If the chart is in the [NoMark] state, mappings are stored without immediate
    /// validation to allow for late-binding of the mark type.
    ///
    /// If a specific mark type is already assigned (e.g., `Chart<MarkPoint>`), this
    /// method will immediately trigger the validation pipeline, including:
    /// 1. Mandatory channel checks (e.g., Bar charts need X and Y).
    /// 2. Semantic type validation (e.g., Histogram X must be continuous).
    /// 3. Data cleaning (dropping nulls from active encoding columns).
    /// 4. Statistical transformations (binning, aggregation for Boxplots, etc.).
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
    /// This internal method synchronizes the data with the encoding rules
    /// specific to the assigned mark type. It ensures schema integrity
    /// and prepares the data for rendering.
    pub(crate) fn validate_and_transform(mut self) -> Result<Self, ChartonError> {
        // --- Step 1: Mark Identification ---
        // We ensure the mark is present before proceeding with mark-specific rules.
        let mark_type = self
            .mark
            .as_ref()
            .map(|m| m.mark_type().to_string())
            .ok_or_else(|| ChartonError::Mark("A mark is required for validation".into()))?;

        // --- Step 2: Mandatory Encoding Validation ---
        // Verify that the minimum required visual channels are mapped.
        match mark_type.as_str() {
            "errorbar" | "bar" | "hist" | "line" | "point" | "area" | "boxplot" | "text"
            | "rule" => {
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
            "none" => return Ok(self), // NoMark state requires no further validation
            _ => {
                return Err(ChartonError::Mark(format!(
                    "Unknown mark type: {}",
                    mark_type
                )));
            }
        }

        // --- Step 3: Semantic Type & Schema Validation ---
        // Check if data columns exist and their types match the mark's requirements.
        let mut active_fields = self.encoding.active_fields();
        let mut expected_semantics = std::collections::HashMap::new();

        // Handle virtual columns for specific transformations
        let x_field = self
            .encoding
            .x
            .as_ref()
            .map(|x| x.field.clone())
            .unwrap_or_default();
        if x_field.is_empty() {
            active_fields.retain(|&field| !field.is_empty());
        }

        if mark_type.as_str() == "hist" {
            let y_field = self.encoding.y.as_ref().unwrap().field.as_str();
            active_fields.retain(|&field| field != y_field); // Y is generated by hist transform
        }

        // Assign semantic requirements (Discrete/Continuous) based on mark rules
        if let Some(shape_enc) = &self.encoding.shape {
            expected_semantics.insert(shape_enc.field.as_str(), vec![SemanticType::Discrete]);
        }
        if let Some(size_enc) = &self.encoding.size {
            expected_semantics.insert(size_enc.field.as_str(), vec![SemanticType::Continuous]);
        }

        match mark_type.as_str() {
            "bar" | "boxplot" => {
                expected_semantics.insert(
                    self.encoding.x.as_ref().unwrap().field.as_str(),
                    vec![SemanticType::Discrete],
                );
                expected_semantics.insert(
                    self.encoding.y.as_ref().unwrap().field.as_str(),
                    vec![SemanticType::Continuous],
                );
            }
            "hist" => {
                expected_semantics.insert(
                    self.encoding.x.as_ref().unwrap().field.as_str(),
                    vec![SemanticType::Continuous],
                );
            }
            "rect" => {
                expected_semantics.insert(
                    self.encoding.color.as_ref().unwrap().field.as_str(),
                    vec![SemanticType::Continuous],
                );
            }
            "errorbar" | "rule" => {
                expected_semantics.insert(
                    self.encoding.y.as_ref().unwrap().field.as_str(),
                    vec![SemanticType::Continuous],
                );
                if let Some(y2) = &self.encoding.y2 {
                    expected_semantics.insert(y2.field.as_str(), vec![SemanticType::Continuous]);
                }
            }
            "text" => {
                if let Some(text_enc) = &self.encoding.text {
                    expected_semantics
                        .insert(text_enc.field.as_str(), vec![SemanticType::Discrete]);
                }
            }
            _ => {}
        }

        check_schema(&mut self.data.df, &active_fields, &expected_semantics)?;

        // --- Step 4: Data Cleaning ---
        // Drop rows with null values in any column used for encoding.
        let filtered_df = self.data.df.drop_nulls(Some(
            &active_fields
                .iter()
                .map(|&s| s.to_string())
                .collect::<Vec<_>>(),
        ))?;

        if filtered_df.height() == 0 {
            self.data = DataFrameSource { df: filtered_df };
            return Ok(self);
        }
        self.data = DataFrameSource { df: filtered_df };

        // --- Step 5: Statistical Transformations ---
        // Resolve bins (required before transformations like histograms)
        self.resolve_pre_transform_encodings()?;

        // Apply mark-specific data transformations
        match mark_type.as_str() {
            "boxplot" => self = self.transform_boxplot_data()?,
            "errorbar" => {
                if self.encoding.y2.is_none() {
                    self = self.transform_errorbar_data()?;
                }
            }
            "rect" => self = self.transform_rect_data()?,
            "bar" => self = self.transform_bar_data()?,
            "hist" => self = self.transform_histogram_data()?,
            _ => {}
        }

        // Apply defaults for scales/axes based on the transformed data
        self.apply_post_transform_defaults()?;

        Ok(self)
    }

    /// Resolves binning configuration required before data transformation.
    fn resolve_pre_transform_encodings(&mut self) -> Result<(), ChartonError> {
        let mt = self.mark.as_ref().unwrap().mark_type();
        let x_enc = self.encoding.x.as_mut().unwrap();
        let y_enc = self.encoding.y.as_mut().unwrap();

        // --- RESOLVE BINS ---
        // Histograms and Heatmaps need bin counts to group the data.
        if ["rect", "hist"].contains(&mt) {
            // Resolve X-axis bins
            if x_enc.bins.is_none() {
                let series = self.data.column(&x_enc.field)?;
                match interpret_semantic_type(series.dtype()) {
                    SemanticType::Continuous => {
                        let unique_count = series.n_unique()?;
                        x_enc.bins = Some(if unique_count <= 1 {
                            1
                        } else {
                            ((unique_count as f64).sqrt() as usize).clamp(5, 50)
                        });
                    }
                    SemanticType::Discrete => x_enc.bins = Some(series.n_unique()?),
                    _ => {}
                }
            }

            // Resolve Y-axis bins (Only for Rect/Heatmaps)
            if mt == "rect" && y_enc.bins.is_none() {
                let series = self.data.column(&y_enc.field)?;
                match interpret_semantic_type(series.dtype()) {
                    SemanticType::Continuous => {
                        let unique_count = series.n_unique()?;
                        y_enc.bins = Some(if unique_count <= 1 {
                            1
                        } else {
                            ((unique_count as f64).sqrt() as usize).clamp(5, 50)
                        });
                    }
                    SemanticType::Discrete => y_enc.bins = Some(series.n_unique()?),
                    _ => {}
                }
            }
        }
        Ok(())
    }
    /// Completes the chart's encoding configuration by inferring missing metadata.
    fn apply_post_transform_defaults(&mut self) -> Result<(), ChartonError> {
        // Determine the mark type early to apply specific defaults (e.g., Rect, Hist)
        let mt = self.mark.as_ref().unwrap().mark_type();

        let x_enc = self.encoding.x.as_mut().unwrap();
        let y_enc = self.encoding.y.as_mut().unwrap();

        // --- 1. RESOLVE SCALE TYPES ---
        // Infer the semantic scale type (Linear, Discrete, or Temporal) based on the column's DataType
        if x_enc.scale_type.is_none() {
            let x_dtype = self.data.df.schema().get(&x_enc.field).unwrap();
            x_enc.scale_type = Some(match interpret_semantic_type(x_dtype) {
                SemanticType::Continuous => Scale::Linear,
                SemanticType::Discrete => Scale::Discrete,
                SemanticType::Temporal => Scale::Temporal,
            });
        }

        if y_enc.scale_type.is_none() {
            let y_dtype = self.data.df.schema().get(&y_enc.field).unwrap();
            y_enc.scale_type = Some(match interpret_semantic_type(y_dtype) {
                SemanticType::Continuous => Scale::Linear,
                SemanticType::Discrete => Scale::Discrete,
                SemanticType::Temporal => Scale::Temporal,
            });
        }

        // --- 2. RESOLVE SPECIAL PADDING & BASELINES ---
        // Apply chart-specific visual rules to ensure statistical integrity and optimal layout.
        if y_enc.scale_type == Some(Scale::Linear) {
            // These mark types represent magnitudes and generally require a zero-based coordinate system.
            if ["area", "bar", "hist"].contains(&mt) {
                // Ensure the scale includes zero to avoid misleading truncated axes.
                y_enc.zero = Some(true);

                // Detection of Pie/Donut mode:
                // In this framework, an empty X field signifies that all data points are
                // mapped to a single angular slot, which characterizes a Pie chart.
                let is_pie_mode = x_enc.field.is_empty();

                if let Ok(y_series) = self.data.column(&y_enc.field) {
                    let y_min = y_series.min::<f64>()?.unwrap_or(0.0);
                    let y_max = y_series.max::<f64>()?.unwrap_or(0.0);

                    // Configure Scale Expansion (Padding):
                    y_enc.expansion = Some(if is_pie_mode {
                        // CRITICAL: Pie charts map the Y-axis to the angular span (0 to 2π).
                        // Any expansion (e.g., the standard 5% padding) would create a
                        // "gap" or "crack" in the circle because the data sum wouldn't
                        // reach the expanded scale maximum. We force zero expansion here.
                        Expansion {
                            mult: (0.0, 0.0),
                            add: (0.0, 0.0),
                        }
                    } else if y_min >= 0.0 {
                        // For standard bars, add a 5% buffer at the top to prevent
                        // the marks from touching the chart boundary.
                        Expansion {
                            mult: (0.0, 0.05),
                            add: (0.0, 0.0),
                        }
                    } else if y_max <= 0.0 {
                        // Add buffer at the bottom for negative-only charts.
                        Expansion {
                            mult: (0.05, 0.0),
                            add: (0.0, 0.0),
                        }
                    } else {
                        // Use default behavior for charts spanning across zero.
                        Expansion::default()
                    });
                }
            }
        }

        // --- 3. HALF-STEP EXPANSION FOR DISCRETE AXES ---
        // For marks with "thickness" (Bar, Boxplot, Rect) on a Discrete axis, we add a 0.5
        // unit padding. This ensures the first and last marks have enough space and
        // don't overlap with the axis lines.
        let needs_discrete_padding = ["bar", "boxplot", "rect"].contains(&mt);
        if needs_discrete_padding {
            // Apply to X axis (Common for Bar/Boxplot)
            if x_enc.scale_type == Some(Scale::Discrete) && x_enc.expansion.is_none() {
                x_enc.expansion = Some(Expansion {
                    mult: (0.0, 0.0),
                    add: (0.5, 0.5),
                });
            }
            // Apply to Y axis (Specific to Discrete Heatmaps)
            if y_enc.scale_type == Some(Scale::Discrete) && y_enc.expansion.is_none() {
                y_enc.expansion = Some(Expansion {
                    mult: (0.0, 0.0),
                    add: (0.5, 0.5),
                });
            }
        }

        // --- 4. FLUSH EXPANSION FOR CONTINUOUS RECT ---
        // Rect charts (Heatmaps) on continuous axes should flush to the edges.
        // Since we already apply "Half-bin Compensation" in get_data_bounds,
        // we set Expansion to zero here to avoid double-padding.
        if mt == "rect" {
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

        // --- 5. RESOLVE OPTIONAL COLOR CHANNEL ---
        if let Some(ref mut color_enc) = self.encoding.color {
            // Only infer the color scale type if it isn't predefined
            if color_enc.scale_type.is_none() {
                let c_dtype = self.data.df.schema().get(&color_enc.field).unwrap();
                color_enc.scale_type = Some(match interpret_semantic_type(c_dtype) {
                    SemanticType::Continuous => Scale::Linear,
                    SemanticType::Discrete => Scale::Discrete,
                    SemanticType::Temporal => Scale::Temporal,
                });
            }
        }

        // --- 6. RESOLVE OPTIONAL SHAPE CHANNEL ---
        if let Some(ref mut shape_enc) = self.encoding.shape {
            // Only infer the shape scale type if it isn't predefined
            if shape_enc.scale_type.is_none() {
                let s_dtype = self.data.df.schema().get(&shape_enc.field).unwrap();
                shape_enc.scale_type = Some(match interpret_semantic_type(s_dtype) {
                    SemanticType::Continuous => Scale::Linear,
                    SemanticType::Discrete => Scale::Discrete,
                    SemanticType::Temporal => Scale::Temporal,
                });
            }
        }

        // --- 7. RESOLVE OPTIONAL SIZE CHANNEL ---
        if let Some(ref mut size_enc) = self.encoding.size {
            // Only infer the size scale type if it isn't predefined
            if size_enc.scale_type.is_none() {
                let s_dtype = self.data.df.schema().get(&size_enc.field).unwrap();
                size_enc.scale_type = Some(match interpret_semantic_type(s_dtype) {
                    SemanticType::Continuous => Scale::Linear,
                    SemanticType::Discrete => Scale::Discrete,
                    SemanticType::Temporal => Scale::Temporal,
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
    Chart<T>: MarkRenderer,
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

    /// Retrieves user-configured scale types (e.g., Linear vs Log).
    fn get_scale(&self, channel: Channel) -> Option<Scale> {
        self.encoding.get_scale_by_channel(channel)
    }

    /// Retrieves user-defined domain overrides.
    fn get_domain(&self, channel: Channel) -> Option<ScaleDomain> {
        self.encoding.get_domain_by_channel(channel)
    }

    /// Retrieves padding/expansion preferences.
    fn get_expand(&self, channel: Channel) -> Option<Expansion> {
        self.encoding.get_expand_by_channel(channel)
    }

    /// Calculates the raw data boundaries for any visual channel.
    ///
    /// This unified implementation supports:
    /// 1. **Standard Encodings**: Simple 1-to-1 mappings (e.g., x, y).
    /// 2. **Explicit Intervals**: Mappings with secondary fields (e.g., y2).
    /// 3. **Implicit Intervals**: Statistical transforms that generate hidden columns
    ///    (e.g., __charton_temp_{field}_min/max).
    fn get_data_bounds(&self, channel: Channel) -> Result<ScaleDomain, ChartonError> {
        let field_name = self.encoding.get_field_by_channel(channel).ok_or_else(|| {
            ChartonError::Data(format!("No field mapped to channel {:?}", channel))
        })?;

        let primary_series = self.data.column(field_name)?;
        let semantic_type = interpret_semantic_type(primary_series.dtype());

        match semantic_type {
            SemanticType::Discrete => {
                let labels = primary_series
                    .unique_stable()?
                    .cast(&DataType::String)?
                    .str()?
                    .into_no_null_iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();
                Ok(ScaleDomain::Discrete(labels))
            }

            SemanticType::Temporal => {
                let min_ns = primary_series.min::<i64>()?.unwrap_or(0);
                let max_ns = primary_series.max::<i64>()?.unwrap_or(0);
                let start_dt = time::OffsetDateTime::from_unix_timestamp_nanos(min_ns as i128)
                    .map_err(|_| ChartonError::Data("Invalid start timestamp".into()))?;
                let end_dt = time::OffsetDateTime::from_unix_timestamp_nanos(max_ns as i128)
                    .map_err(|_| ChartonError::Data("Invalid end timestamp".into()))?;
                Ok(ScaleDomain::Temporal(start_dt, end_dt))
            }

            SemanticType::Continuous => {
                let mut global_min = f64::INFINITY;
                let mut global_max = f64::NEG_INFINITY;
                let mut found_data = false;

                // --- STACKED BAR LOGIC ---
                // For stacked charts, boundaries are determined by the sum of values
                // in each group rather than individual rows.
                let is_y_stacked = channel == Channel::Y
                    && self.encoding.y.as_ref().is_some_and(|e| e.stack)
                    && self.encoding.color.is_some();

                if is_y_stacked {
                    let x_field = &self.encoding.x.as_ref().unwrap().field;
                    let y_field = &self.encoding.y.as_ref().unwrap().field;

                    // Aggregate sums per X-axis category to find the true visual peak
                    let grouped_sums = self
                        .data
                        .df
                        .clone()
                        .lazy()
                        .group_by([col(x_field)])
                        .agg([col(y_field).sum().alias("stack_sum")])
                        .collect()?;

                    let sum_series = grouped_sums.column("stack_sum")?.as_materialized_series();
                    global_min = sum_series.min::<f64>()?.unwrap_or(0.0);
                    global_max = sum_series.max::<f64>()?.unwrap_or(0.0);
                    found_data = true;
                } else {
                    let mut columns_to_scan = vec![field_name.to_string()];
                    if channel == Channel::Y {
                        if let Some(y2_enc) = &self.encoding.y2 {
                            columns_to_scan.push(y2_enc.field.clone());
                        }
                        columns_to_scan.push(format!("{}_{}_min", TEMP_SUFFIX, field_name));
                        columns_to_scan.push(format!("{}_{}_max", TEMP_SUFFIX, field_name));
                    }

                    for col_name in &columns_to_scan {
                        if let Ok(series) = self.data.column(col_name) {
                            if let Ok(Some(m)) = series.min::<f64>() {
                                global_min = global_min.min(m);
                                found_data = true;
                            }
                            if let Ok(Some(m)) = series.max::<f64>() {
                                global_max = global_max.max(m);
                                found_data = true;
                            }
                        }
                    }
                }

                // Fallback if no data was found during scan
                if !found_data {
                    global_min = primary_series.min::<f64>()?.unwrap_or(0.0);
                    global_max = primary_series.max::<f64>()?.unwrap_or(1.0);
                }

                // --- HALF-BIN COMPENSATION LOGIC ---
                // For binned data (e.g., Rect/Heatmap), the current min/max represent bin centers.
                // We expand the domain by half a bin width on both sides so the scale covers
                // the full visual extent of the rectangles.
                let bins = match channel {
                    Channel::X => self.encoding.x.as_ref().and_then(|e| e.bins),
                    Channel::Y => self.encoding.y.as_ref().and_then(|e| e.bins),
                    _ => None,
                };

                if let Some(n_bins) = bins {
                    if n_bins > 1 && global_max > global_min {
                        // The distance between the first and last center covers (n-1) intervals.
                        let bin_width = (global_max - global_min) / (n_bins as f64 - 1.0);
                        global_min -= bin_width / 2.0;
                        global_max += bin_width / 2.0;
                    } else if n_bins == 1 {
                        // Single bin case: expand by an arbitrary unit to give the block volume
                        global_min -= 0.5;
                        global_max += 0.5;
                    }
                }

                // Ensure zero baseline if requested (e.g., for bar/hist charts)
                let force_zero = self.encoding.get_zero_by_channel(channel);
                if force_zero {
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
}
