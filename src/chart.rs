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
pub mod tick_chart;

use crate::TEMP_SUFFIX;
use crate::coordinate::CoordinateTrait;
use crate::core::aesthetics::GlobalAesthetics;
use crate::core::data::{Dataset, SemanticType, ToDataset};
use crate::core::layer::{Layer, MarkRenderer};
use crate::encode::{Channel, Encoding, IntoEncoding, y::StackMode};
use crate::error::ChartonError;
use crate::mark::{
    Mark, area::MarkArea, bar::MarkBar, boxplot::MarkBoxplot, errorbar::MarkErrorBar,
    histogram::MarkHist, line::MarkLine, no_mark::NoMark, point::MarkPoint, rect::MarkRect,
    rule::MarkRule, text::MarkText, tick::MarkTick,
};
use crate::scale::{Expansion, Scale, ScaleDomain};
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
///   enabling the "Base Chart" pattern similar to Altair.
///
/// # Fields
///
/// * `data` - The underlying data source.
/// * `encoding` - Mapping between data fields and visual channels (x, y, color, etc.).
/// * `mark` - The specific visual mark configuration. Is `None` when `T` is [NoMark].
#[derive(Clone)]
pub struct Chart<T: Mark = NoMark> {
    pub(crate) data: Dataset,
    pub(crate) encoding: Encoding,
    pub(crate) mark: Option<T>,
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
        })
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

    /// Transitions the base chart into a Tick chart.
    pub fn mark_tick(self) -> Result<Chart<MarkTick>, ChartonError> {
        let chart = Chart::<MarkTick> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkTick::default()),
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
            | "rule" | "tick" => {
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
            expected_semantics.insert(
                size_enc.field.as_str(),
                vec![SemanticType::Continuous, SemanticType::Temporal],
            );
        }

        match mark_type.as_str() {
            "bar" | "boxplot" => {
                expected_semantics.insert(
                    self.encoding.x.as_ref().unwrap().field.as_str(),
                    vec![SemanticType::Discrete],
                );
                expected_semantics.insert(
                    self.encoding.y.as_ref().unwrap().field.as_str(),
                    vec![SemanticType::Continuous, SemanticType::Temporal],
                );
            }
            "hist" => {
                expected_semantics.insert(
                    self.encoding.x.as_ref().unwrap().field.as_str(),
                    vec![SemanticType::Continuous, SemanticType::Temporal],
                );
            }
            "rect" => {
                expected_semantics.insert(
                    self.encoding.color.as_ref().unwrap().field.as_str(),
                    vec![SemanticType::Continuous, SemanticType::Temporal],
                );
            }
            "errorbar" | "rule" => {
                expected_semantics.insert(
                    self.encoding.y.as_ref().unwrap().field.as_str(),
                    vec![SemanticType::Continuous, SemanticType::Temporal],
                );
                if let Some(y2) = &self.encoding.y2 {
                    expected_semantics.insert(
                        y2.field.as_str(),
                        vec![SemanticType::Continuous, SemanticType::Temporal],
                    );
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

        let _resolved_semantics = self
            .data
            .check_schema(&active_fields, &expected_semantics)?;

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
            "area" => self = self.transform_area_data()?,
            _ => {}
        }

        // Apply defaults for scales/axes based on the transformed data
        self.apply_post_transform_defaults()?;

        Ok(self)
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

    /// Completes the chart's encoding configuration by inferring missing metadata.
    fn apply_post_transform_defaults(&mut self) -> Result<(), ChartonError> {
        // 1. PRE-RESOLUTION SETUP
        let mt = self.mark.as_ref().unwrap().mark_type();

        // Define a helper closure to infer scale type based on the column's semantic type.
        // This removes the need for repetitive match statements across all channels.
        let resolve_scale_type = |field: &str| -> Result<Scale, ChartonError> {
            let col = self.data.column(field)?;
            Ok(match col.semantic_type() {
                SemanticType::Continuous => Scale::Linear,
                SemanticType::Discrete => Scale::Discrete,
                SemanticType::Temporal => Scale::Temporal,
            })
        };

        // --- 2. RESOLVE SCALE TYPES FOR ALL CHANNELS ---
        // Resolve X and Y (Mandatory)
        if self.encoding.x.as_ref().unwrap().scale_type.is_none() {
            let x_enc = self.encoding.x.as_mut().unwrap();
            x_enc.scale_type = Some(resolve_scale_type(&x_enc.field)?);
        }
        if self.encoding.y.as_ref().unwrap().scale_type.is_none() {
            let y_enc = self.encoding.y.as_mut().unwrap();
            y_enc.scale_type = Some(resolve_scale_type(&y_enc.field)?);
        }

        // Resolve Optional Channels (Color, Shape, Size)
        if let Some(ref mut color) = self.encoding.color {
            if color.scale_type.is_none() {
                color.scale_type = Some(resolve_scale_type(&color.field)?);
            }
        }
        if let Some(ref mut shape) = self.encoding.shape {
            if shape.scale_type.is_none() {
                shape.scale_type = Some(resolve_scale_type(&shape.field)?);
            }
        }
        if let Some(ref mut size) = self.encoding.size {
            if size.scale_type.is_none() {
                size.scale_type = Some(resolve_scale_type(&size.field)?);
            }
        }

        // --- 3. RESOLVE SPECIAL PADDING & BASELINES ---
        let x_enc = self.encoding.x.as_mut().unwrap();
        let y_enc = self.encoding.y.as_mut().unwrap();

        // Statistical Integrity for Linear Y-Scales:
        // Marks representing magnitude (Bar, Area, Hist) should generally start at zero.
        if y_enc.scale_type == Some(Scale::Linear) && ["area", "bar", "hist"].contains(&mt) {
            y_enc.zero = Some(true);

            // PIE MODE DETECTION: An empty X field implies a radial projection of the Y axis.
            let is_pie_mode = x_enc.field.is_empty();

            if let Ok(y_col) = self.data.column(&y_enc.field) {
                let (y_min, y_max) = y_col.min_max();

                y_enc.expansion = Some(if is_pie_mode {
                    // Force zero expansion for Pie charts to prevent "cracks" in the circle.
                    Expansion {
                        mult: (0.0, 0.0),
                        add: (0.0, 0.0),
                    }
                } else if y_min >= 0.0 {
                    // Buffer at the top for positive distributions.
                    Expansion {
                        mult: (0.0, 0.05),
                        add: (0.0, 0.0),
                    }
                } else if y_max <= 0.0 {
                    // Buffer at the bottom for negative distributions.
                    Expansion {
                        mult: (0.05, 0.0),
                        add: (0.0, 0.0),
                    }
                } else {
                    // Default padding for data crossing zero.
                    Expansion::default()
                });
            }
        }

        // --- 4. HALF-STEP PADDING FOR DISCRETE AXES ---
        // Categorical marks with thickness (Bar, Boxplot, Rect) need 0.5 units of padding
        // to center the marks and prevent them from clipping against the axis lines.
        let needs_discrete_padding = ["bar", "boxplot", "rect"].contains(&mt);
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

        // --- 5. FLUSH CONTINUOUS RECTANGLES ---
        // Heatmaps (Rect) on continuous scales should touch the edges of the plotting area.
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
    /// Calculates the raw data boundaries for any visual channel.
    fn get_data_bounds(&self, channel: Channel) -> Result<ScaleDomain, ChartonError> {
        let field_name = self.encoding.get_field_by_channel(channel).ok_or_else(|| {
            ChartonError::Data(format!("No field mapped to channel {:?}", channel))
        })?;

        let primary_series = self.data.column(field_name)?;
        let semantic_type = primary_series.semantic_type();

        match semantic_type {
            // --- DISCRETE DOMAIN ---
            SemanticType::Discrete => {
                let labels = primary_series.unique_values();
                Ok(ScaleDomain::Discrete(labels))
            }

            // --- TEMPORAL DOMAIN ---
            SemanticType::Temporal => {
                let (min_ts, max_ts) = primary_series.min_max();
                Ok(ScaleDomain::Temporal(min_ts as i64, max_ts as i64))
            }

            // --- CONTINUOUS DOMAIN ---
            SemanticType::Continuous => {
                let mut global_min = f64::INFINITY;
                let mut global_max = f64::NEG_INFINITY;
                let mut found_data = false;

                // Determine if stacking is required (Y-axis + Stack Mode + Color grouping)
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

                    // --- MANUAL STACKING AGGREGATION ---
                    // We use a HashMap to group Y-sums by their X-category string representation.
                    let mut stacks: std::collections::HashMap<String, f64> =
                        std::collections::HashMap::new();

                    // We iterate through the data rows
                    // Note: This assumes all columns in your 'self.data' have the same length.
                    let row_count = x_series.len();
                    for i in 0..row_count {
                        // Extract X key and Y value for current row
                        if let (Some(x_val), Some(y_val)) =
                            (x_series.get_as_string(i), y_series.get_as_f64(i))
                        {
                            let entry = stacks.entry(x_val).or_insert(0.0);
                            *entry += y_val;
                        }
                    }

                    // Find min/max across all stack totals
                    for &sum in stacks.values() {
                        global_min = global_min.min(sum);
                        global_max = global_max.max(sum);
                        found_data = true;
                    }
                } else {
                    // --- STANDARD SCANNING LOGIC ---
                    let mut columns_to_scan = Vec::new();
                    let mark_type = self.mark.as_ref().map(|m| m.mark_type());

                    if !matches!(mark_type, Some("area")) {
                        columns_to_scan.push(field_name.to_string());
                    }

                    if channel == Channel::Y {
                        if let Some(y2_enc) = &self.encoding.y2 {
                            columns_to_scan.push(y2_enc.field.clone());
                        }
                        columns_to_scan.push(format!("{}_{}_min", TEMP_SUFFIX, field_name));
                        columns_to_scan.push(format!("{}_{}_max", TEMP_SUFFIX, field_name));
                    }

                    for col_name in &columns_to_scan {
                        if let Ok(series) = self.data.column(col_name) {
                            let (m_min, m_max) = series.min_max();
                            global_min = global_min.min(m_min);
                            global_max = global_max.max(m_max);
                            found_data = true;
                        }
                    }
                }

                // Final fallbacks and adjustments
                if !found_data {
                    let (p_min, p_max) = primary_series.min_max();
                    global_min = p_min;
                    global_max = p_max;
                }

                // Half-bin compensation for Binned data
                let bins = match channel {
                    Channel::X => self.encoding.x.as_ref().and_then(|e| e.bins),
                    Channel::Y => self.encoding.y.as_ref().and_then(|e| e.bins),
                    _ => None,
                };

                if let Some(n_bins) = bins {
                    if n_bins > 1 && global_max > global_min {
                        let bin_width = (global_max - global_min) / (n_bins as f64 - 1.0);
                        global_min -= bin_width / 2.0;
                        global_max += bin_width / 2.0;
                    }
                }

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
}
