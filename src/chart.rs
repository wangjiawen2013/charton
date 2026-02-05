pub mod point_chart;
pub mod line_chart;
pub mod bar_chart;
pub mod hist_chart;
pub mod box_chart;
pub mod pie_chart;
pub mod area_chart;
pub mod rect_chart;
pub mod rule_chart;
pub mod text_chart;
pub mod errorbar_chart;

use crate::core::data::*;
use crate::core::layer::{Layer, MarkRenderer};
use crate::encode::{Channel, Encoding, IntoEncoding};
use crate::scale::{Scale, ScaleDomain, Expansion};
use crate::coordinate::CoordinateTrait;
use crate::core::aesthetics::GlobalAesthetics;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::TEMP_SUFFIX;
use polars::prelude::*;
use std::sync::Arc;

/// Generic Chart structure - chart-specific properties only
///
/// This struct represents a single-layer chart with a specific mark type. It holds
/// all the necessary data and configuration for rendering a chart, including the
/// data source, encoding mappings, mark properties, and styling options.
///
/// The generic parameter `T` represents the mark type, which determines the
/// visualization type (e.g., bar, line, point, area, etc.).
///
/// # Type Parameters
///
/// * `T` - The mark type that implements the `Mark` trait, determining the chart type
///
/// # Fields
///
/// * `data` - The data source for the chart as a DataFrame
/// * `encoding` - Encoding mappings that define how data fields map to visual properties
/// * `mark` - Optional mark configuration specific to the chart type
pub struct Chart<T: Mark> {
    pub(crate) data: DataFrameSource,
    pub(crate) encoding: Encoding,
    pub(crate) mark: Option<T>,
}

impl<T: Mark> Chart<T> {
    /// Create a new chart instance with the provided data source
    ///
    /// This is the entry point for creating a new chart. It initializes a chart with the
    /// provided data source and sets up default values for all other chart properties.
    /// The chart is not yet fully configured and requires additional method calls to
    /// specify the mark type, encoding mappings, and other properties.
    ///
    /// The data source can be any type that implements `Into<DataFrameSource>`, which
    /// includes `&DataFrame`, `&LazyFrame`, and other compatible types.
    ///
    /// # Arguments
    ///
    /// * `source` - The data source for the chart, convertible to DataFrameSource
    ///
    /// # Returns
    ///
    /// Returns a Result containing the new Chart instance or a ChartonError if initialization fails
    ///
    /// # Example
    ///
    /// ```
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df = df![
    ///     "x" => [1, 2, 3, 4, 5],
    ///     "y" => [10, 20, 30, 40, 50]
    /// ]?;
    ///
    /// let chart = Chart::<MarkPoint>::build(&df)?;
    /// ```
    pub fn build<S>(source: S) -> Result<Self, ChartonError>
    where
        S: TryInto<DataFrameSource, Error = ChartonError>,
    {
        let source = source.try_into()?;

        let mut chart = Self {
            data: source,
            encoding: Encoding::new(),
            mark: None,
        };

        // Automatically convert numeric types to f64
        chart.data = convert_numeric_types(chart.data.clone())?;

        Ok(chart)
    }

    /// Apply encoding mappings to the chart
    ///
    /// This method sets up the visual encoding mappings that define how data fields map to
    /// visual properties of the chart marks. These encodings determine how your data is
    /// visually represented in the chart.
    ///
    /// The method performs several important validations:
    /// 1. Checks that all data columns have supported data types
    /// 2. Ensures the required mark type has been set
    /// 3. Validates that mandatory encodings are provided for the specific chart type
    /// 4. Verifies data types match encoding requirements
    /// 5. Filters out rows with null values in encoded columns
    /// 6. Applies chart-specific data transformations when needed
    ///
    /// Different chart types have different encoding requirements:
    /// - Most charts require both x and y encodings
    /// - Rect charts require x, y, and color encodings
    /// - Arc charts require theta and color encodings
    ///
    /// # Arguments
    ///
    /// * `enc` - An encoding specification that implements IntoEncoding trait
    ///
    /// # Returns
    ///
    /// Returns a Result containing the updated Chart instance or a ChartonError if validation fails
    ///
    /// # Example
    ///
    /// ```
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df = df![
    ///     "x" => [1, 2, 3, 4, 5],
    ///     "y" => [10, 20, 30, 40, 50],
    ///     "category" => ["A", "B", "A", "B", "A"]
    /// ]?;
    ///
    /// let chart = Chart::<MarkPoint>::build(&df)?
    ///     .mark_point()
    ///     .encode(
    ///         x("x").scale(Scale::Linear),
    ///         y("y").scale(Scale::Linear),
    ///         color("category")
    ///     )?;
    /// ```
    pub fn encode<U>(mut self, enc: U) -> Result<Self, ChartonError>
    where
        U: IntoEncoding,
    {
        // 1. Apply the user-provided encoding specifications
        enc.apply(&mut self.encoding);

        // 2. Mark identification: A mark is required to determine the chart's structural rules.
        // We convert the mark type to a String to release the borrow on `self`.
        let mark_type = self
            .mark
            .as_ref()
            .map(|m| m.mark_type().to_string())
            .ok_or_else(|| ChartonError::Mark("A mark is required to create a chart".into()))?;

        // --- Step 3: Mandatory Encoding Validation ---
        // Ensure the minimum required visual channels are present for the chosen mark type.
        match mark_type.as_str() {
            "errorbar" | "bar" | "hist" | "line" | "point" | "area" | "boxplot" | "text" | "rule" => {
                if self.encoding.x.is_none() || self.encoding.y.is_none() {
                    return Err(ChartonError::Encoding(format!(
                        "{} chart requires both x and y encodings", mark_type
                    )));
                }
            }
            "rect" => {
                if self.encoding.x.is_none() || self.encoding.y.is_none() || self.encoding.color.is_none() {
                    return Err(ChartonError::Encoding("Rect chart requires x, y, and color encodings".into()));
                }
            }
            "arc" => {
                if self.encoding.theta.is_none() || self.encoding.color.is_none() {
                    return Err(ChartonError::Encoding("Arc chart requires both theta and color encodings".into()));
                }
            }
            _ => {
                return Err(ChartonError::Mark(format!("Unknown mark type: {}", mark_type)));
            }
        }

        // --- Step 4: Semantic Type Validation ---
        // Instead of checking for raw Polars DataTypes (like Float64), we check for 
        // high-level SemanticCategories. This is more robust as we've already 
        // normalized numeric types to f64.
        let mut active_fields = self.encoding.active_fields();
        let mut expected_semantics = std::collections::HashMap::new();

        // Histogram Virtual Column Handling.
        // Histograms are a special case where the Y-axis (frequency/count) is a 
        // "virtual" column produced by the statistical transformation.
        // Because this column is not present in the raw source DataFrame, we 
        // remove it from the validation set to avoid "field not found" errors.
        if mark_type.as_str() == "hist" {
            // Safety: Both X and Y are guaranteed to exist by Step 3.
            let y_field = self.encoding.y.as_ref().unwrap().field.as_str();
            
            // Retain only fields that actually exist in the raw input data.
            active_fields.retain(|&field| field != y_field);
        }

        // A. Universal Aesthetic Channels
        // Channels like Size and Opacity are mathematically mapped to continuous scales.
        if let Some(shape_enc) = &self.encoding.shape {
            expected_semantics.insert(shape_enc.field.as_str(), vec![SemanticType::Discrete]);
        }
        if let Some(size_enc) = &self.encoding.size {
            expected_semantics.insert(size_enc.field.as_str(), vec![SemanticType::Continuous]);
        }

        // B. Mark-Specific Semantic Requirements
        match mark_type.as_str() {
            "bar" | "boxplot" => {
                // Standard categorical plots require a discrete axis (X) and a numeric axis (Y).
                expected_semantics.insert(self.encoding.x.as_ref().unwrap().field.as_str(), vec![SemanticType::Discrete]);
                expected_semantics.insert(self.encoding.y.as_ref().unwrap().field.as_str(), vec![SemanticType::Continuous]);
            }
            "hist" => {
                // Histograms bin continuous data on the X axis.
                expected_semantics.insert(self.encoding.x.as_ref().unwrap().field.as_str(), vec![SemanticType::Continuous]);
            }
            "rect" => {
                // Heatmaps map continuous values to a color gradient.
                expected_semantics.insert(self.encoding.color.as_ref().unwrap().field.as_str(), vec![SemanticType::Continuous]);
            }
            "errorbar" | "rule" => {
                // Ranges require continuous values for both start (y) and end (y2) points.
                expected_semantics.insert(self.encoding.y.as_ref().unwrap().field.as_str(), vec![SemanticType::Continuous]);
                if let Some(y2) = &self.encoding.y2 {
                    expected_semantics.insert(y2.field.as_str(), vec![SemanticType::Continuous]);
                }
            }
            "text" => {
                if let Some(text_enc) = &self.encoding.text {
                    expected_semantics.insert(text_enc.field.as_str(), vec![SemanticType::Discrete]);
                }
            }
            _ => {} // Other marks like 'point' or 'line' are flexible with their axis types.
        }

        // 5. Schema Integrity Check: Validate columns exist and match semantic expectations.
        check_schema(&mut self.data.df, &active_fields, &expected_semantics).map_err(|e| {
            eprintln!("Error validating encoding fields for {}: {}", mark_type, e);
            e
        })?;

        // Filter out null values
        let filtered_df = self
            .data
            .df
            .drop_nulls(Some(
                &active_fields
                    .iter()
                    .map(|&s| s.to_string()) // Convert &str to String
                    .collect::<Vec<_>>(),
            ))
            .map_err(|e| {
                eprintln!(
                    "Error filtering null values from columns {:?}: {}",
                    active_fields, e
                );
                e
            })?;

        // Check if the filtered DataFrame is empty
        if filtered_df.height() == 0 {
            eprintln!(
                "Warning: No valid data remaining after filtering null values from columns: {:?}",
                active_fields
            );
            self.data = DataFrameSource { df: filtered_df };
            return Ok(self); // Return early to avoid unnecessary processing
        } else {
            self.data = DataFrameSource { df: filtered_df };
        }

        // Set default encodings
        self.resolve_pre_transform_encodings()?;

        // Perform chart-specific data transformations based on mark type
        match mark_type.as_str() {
            "boxplot" => {
                // Apply boxplot-specific data transformations
                self = self.transform_boxplot_data()?;
            }
            "errorbar" => {
                // Apply errorbar-specific data transformations only when y2 encoding is not present
                if self.encoding.y2.is_none() {
                    self = self.transform_errorbar_data()?;
                }
            }
            "rect" => {
                // Apply rect-specific data transformations
                self = self.transform_rect_data()?;
            }
            "bar" => {
                // Apply bar-specific data transformations
                self = self.transform_bar_data()?;
            }
            "hist" => {
                // Apply histogram-specific data transformations
                self = self.transform_histogram_data()?;
            }
            _ => {
                // Nothing to do for other marks
            }
        }

        // Set default encodings
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
                        x_enc.bins = Some(if unique_count <= 1 { 1 } else { 
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
                        y_enc.bins = Some(if unique_count <= 1 { 1 } else { 
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
                SemanticType::Discrete   => Scale::Discrete,
                SemanticType::Temporal   => Scale::Temporal,
            });
        }

        if y_enc.scale_type.is_none() {
            let y_dtype = self.data.df.schema().get(&y_enc.field).unwrap();
            y_enc.scale_type = Some(match interpret_semantic_type(y_dtype) {
                SemanticType::Continuous => Scale::Linear,
                SemanticType::Discrete   => Scale::Discrete,
                SemanticType::Temporal   => Scale::Temporal,
            });
        }

        // --- 2. RESOLVE SPECIAL PADDING & BASELINES ---
        // Apply chart-specific visual rules (e.g., bar charts should start at zero)
        if y_enc.scale_type == Some(Scale::Linear) {
            if ["area", "bar", "hist"].contains(&mt) {
                // Force zero baseline for statistical accuracy in magnitude-based charts
                y_enc.zero = Some(true);

                if let Ok(y_series) = self.data.column(&y_enc.field) {
                    let y_min = y_series.min::<f64>()?.unwrap_or(0.0);
                    let y_max = y_series.max::<f64>()?.unwrap_or(0.0);

                    // Asymmetric expansion: only add padding away from the zero baseline
                    y_enc.expansion = Some(if y_min >= 0.0 {
                        Expansion { mult: (0.0, 0.05), add: (0.0, 0.0) }
                    } else if y_max <= 0.0 {
                        Expansion { mult: (0.05, 0.0), add: (0.0, 0.0) }
                    } else {
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
                x_enc.expansion = Some(Expansion { mult: (0.0, 0.0), add: (0.5, 0.5) });
            }
            // Apply to Y axis (Specific to Discrete Heatmaps)
            if y_enc.scale_type == Some(Scale::Discrete) && y_enc.expansion.is_none() {
                y_enc.expansion = Some(Expansion { mult: (0.0, 0.0), add: (0.5, 0.5) });
            }
        }

        // --- 4. FLUSH EXPANSION FOR CONTINUOUS RECT ---
        // Rect charts (Heatmaps) on continuous axes should flush to the edges.
        // Since we already apply "Half-bin Compensation" in get_data_bounds, 
        // we set Expansion to zero here to avoid double-padding.
        if mt == "rect" {
            if x_enc.scale_type != Some(Scale::Discrete) && x_enc.expansion.is_none() {
                x_enc.expansion = Some(Expansion { mult: (0.0, 0.0), add: (0.0, 0.0) });
            }
            if y_enc.scale_type != Some(Scale::Discrete) && y_enc.expansion.is_none() {
                y_enc.expansion = Some(Expansion { mult: (0.0, 0.0), add: (0.0, 0.0) });
            }
        }

        // --- 5. RESOLVE OPTIONAL COLOR CHANNEL ---
        if let Some(ref mut color_enc) = self.encoding.color {
            // Only infer the color scale type if it isn't predefined
            if color_enc.scale_type.is_none() {
                let c_dtype = self.data.df.schema().get(&color_enc.field).unwrap();
                color_enc.scale_type = Some(match interpret_semantic_type(c_dtype) {
                    SemanticType::Continuous => Scale::Linear,
                    SemanticType::Discrete   => Scale::Discrete,
                    SemanticType::Temporal   => Scale::Temporal,
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
                    SemanticType::Discrete   => Scale::Discrete,
                    SemanticType::Temporal   => Scale::Temporal,
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
                    SemanticType::Discrete   => Scale::Discrete,
                    SemanticType::Temporal   => Scale::Temporal,
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
        if self.mark.as_ref().map(|m| m.mark_type()) == Some("arc") {
            false
        } else {
            true
        }
    }

    /// Retrieves the field name for a specific channel.
    /// Redirects to the central Encoding container.
    fn get_field(&self, channel: Channel) -> Option<String> {
        self.encoding.get_field_by_channel(channel).map(|s| s.to_string())
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
    fn inject_resolved_scales(&self, coord: Arc<dyn CoordinateTrait>, aesthetics: &GlobalAesthetics) {
        
        // 1. Inject Position Scales (X & Y)
        // We only inject if the channel was actually configured by the user.
        if let Some(ref x_enc) = self.encoding.x {
            if let Ok(mut guard) = x_enc.resolved_scale.write() {
                *guard = Some(coord.get_x_arc());
            }
        }
        
        if let Some(ref y_enc) = self.encoding.y {
            if let Ok(mut guard) = y_enc.resolved_scale.write() {
                *guard = Some(coord.get_y_arc());
            }
        }

        // 2. Inject Aesthetic Scales (Color, Shape, Size)
        // We perform a "Field Match" check to ensure the global scale matches this layer's intent.

        // --- Color Channel ---
        // Use .as_ref() to match against a reference instead of moving the value
        if let (Some(enc), Some(map)) = (self.encoding.color.as_ref(), aesthetics.color.as_ref()) {
            if enc.field == map.field {
                if let Ok(mut guard) = enc.resolved_scale.write() {
                    *guard = Some(map.scale_impl.clone());
                }
            }
        }

        // --- Shape Channel ---
        if let (Some(enc), Some(map)) = (self.encoding.shape.as_ref(), aesthetics.shape.as_ref()) {
            if enc.field == map.field {
                if let Ok(mut guard) = enc.resolved_scale.write() {
                    *guard = Some(map.scale_impl.clone());
                }
            }
        }

        // --- Size Channel ---
        if let (Some(enc), Some(map)) = (self.encoding.size.as_ref(), aesthetics.size.as_ref()) {
            if enc.field == map.field {
                if let Ok(mut guard) = enc.resolved_scale.write() {
                    *guard = Some(map.scale_impl.clone());
                }
            }
        }
    }
}