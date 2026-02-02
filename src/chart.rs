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
        enc.apply(&mut self.encoding);

        // A mark is required to determine chart type - cannot proceed without it
        let mark = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("A mark is required to create a chart".into()))?;

        // Validate mandatory encodings - these are the minimum required fields for each chart type and cannot be omitted
        match mark.mark_type() {
            // Marks that require both x and y encodings
            "errorbar" | "bar" | "hist" | "line" | "point" | "area" | "boxplot" | "text"
            | "rule" => {
                if self.encoding.x.is_none() || self.encoding.y.is_none() {
                    return Err(ChartonError::Encoding(format!(
                        "{} chart requires both x and y encodings",
                        mark.mark_type()
                    )));
                }
            }
            // Rect charts require x, y, and color encodings
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
            // Marks with specialized requirements - arc charts require theta and color encodings
            "arc" => {
                // For arc charts, we need theta encoding (for the pie slice sizes) and color encoding (for segments)
                if self.encoding.theta.is_none() || self.encoding.color.is_none() {
                    return Err(ChartonError::Encoding(
                        "Arc chart requires both theta and color encodings".into(),
                    ));
                }
            }
            // This match is exhaustive - all possible mark types are covered above
            // If we reach here, it indicates a programming error where an unknown mark type was created
            _ => {
                return Err(ChartonError::Mark(format!(
                    "Unknown mark type: {}. This is a programming error - all mark types should be handled explicitly.",
                    mark.mark_type()
                )));
            }
        }

        // Build required columns and expected types
        let mut active_fields = self.encoding.active_fields();
        let mut expected_types = std::collections::HashMap::new();

        // Add type checking for shape encoding - must be discrete (String)
        if let Some(shape_enc) = &self.encoding.shape {
            expected_types.insert(shape_enc.field.as_str(), vec![DataType::String]);
        }

        // Add type checking for size encoding - must be continuous (f64)
        if let Some(size_enc) = &self.encoding.size {
            // we've already converted all numeric types to f64 when building the chart
            expected_types.insert(size_enc.field.as_str(), vec![DataType::Float64]);
        }

        // Add type checking for errorbar charts - y and y2 encodings must be f64 (continuous)
        if mark.mark_type() == "errorbar" {
            // we've already converted all numeric types to f64 when building the chart
            expected_types.insert(
                self.encoding.y.as_ref().unwrap().field.as_str(),
                vec![DataType::Float64],
            );

            // If y2 encoding exists, it must also be f64
            if let Some(y2_encoding) = &self.encoding.y2 {
                expected_types.insert(y2_encoding.field.as_str(), vec![DataType::Float64]);
            }
        }

        // Add type checking for histogram charts - x encoding must be f64 (continuous)
        if mark.mark_type() == "hist" {
            active_fields
                .retain(|&field| field != self.encoding.y.as_ref().unwrap().field.as_str());
            // we've already converted all numeric types to f64 when building the chart
            expected_types.insert(
                self.encoding.x.as_ref().unwrap().field.as_str(),
                vec![DataType::Float64],
            );
        }

        // Add type checking for rect charts - color encoding must be f64 (continuous) for proper color mapping
        if mark.mark_type() == "rect" {
            // we've already converted all numeric types to f64 when building the chart
            expected_types.insert(
                self.encoding.color.as_ref().unwrap().field.as_str(),
                vec![DataType::Float64],
            );
        }

        // Add type checking for boxplot charts - y encoding must be f64 (continuous)
        if mark.mark_type() == "boxplot" {
            // we've already converted all numeric types to f64 when building the chart
            // Boxplot requires x to be discrete and y to be continuous (numeric)
            expected_types.insert(
                self.encoding.x.as_ref().unwrap().field.as_str(),
                vec![DataType::String],
            );
            expected_types.insert(
                self.encoding.y.as_ref().unwrap().field.as_str(),
                vec![DataType::Float64],
            );
        }

        // Add type checking for bar charts - y encoding must be f64 (continuous)
        if mark.mark_type() == "bar" {
            // we've already converted all numeric types to f64 when building the chart
            // Boxplot requires x to be discrete and y to be continuous (numeric)
            expected_types.insert(
                self.encoding.x.as_ref().unwrap().field.as_str(),
                vec![DataType::String],
            );
            expected_types.insert(
                self.encoding.y.as_ref().unwrap().field.as_str(),
                vec![DataType::Float64],
            );
        }

        // Add type checking for rule charts - y and y2 encodings must be f64 (continuous)
        if mark.mark_type() == "rule" {
            // we've already converted all numeric types to f64 when building the chart
            expected_types.insert(
                self.encoding.y.as_ref().unwrap().field.as_str(),
                vec![DataType::Float64],
            );

            // If y2 encoding exists, it must also be f64
            if let Some(y2_encoding) = &self.encoding.y2 {
                expected_types.insert(y2_encoding.field.as_str(), vec![DataType::Float64]);
            }
        }

        // Add type checking for text charts - text encoding must be String
        if mark.mark_type() == "text"
            && let Some(text_enc) = &self.encoding.text
        {
            expected_types.insert(text_enc.field.as_str(), vec![DataType::String]);
        }

        // Use check_schema to validate columns exist in the dataframe and have correct types
        check_schema(&mut self.data.df, &active_fields, &expected_types).map_err(|e| {
            eprintln!("Error validating encoding fields: {}", e);
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

        // Perform chart-specific data transformations based on mark type
        match mark.mark_type() {
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
        self.apply_default_encodings();

        Ok(self)
    }

    /// Completes the chart's encoding configuration by inferring missing metadata.
    ///
    /// This method is the "Intelligence Layer" that populates resolved states:
    /// 1. **Scale Mapping**: Automatically maps DataFrame column types to high-level Scale 
    ///    types (Linear, Discrete, or Temporal).
    /// 2. **Visual Integrity**: Enforces the "Zero-Baseline" rule for quantitative 
    ///    marks (Area, Bar, Hist) to ensure the visual area represents the data magnitude accurately.
    ///
    /// # Safety/Preconditions
    /// This method uses `unwrap()` on X/Y encodings and Schema lookups because it 
    /// assumes `check_schema` and mandatory field validation have already passed 
    /// in the `encode` pipeline.
    fn apply_default_encodings(&mut self) {
        // --- 1. RESOLVE X CHANNEL ---
        // Mandatory: x_enc and its field are guaranteed to exist by earlier validation
        let x_enc = self.encoding.x.as_mut().unwrap();
        let x_dtype = self.data.df.schema().get(&x_enc.field).unwrap();
        
        x_enc.scale_type = Some(match interpret_semantic_type(x_dtype) {
            SemanticType::Continuous => Scale::Linear,
            SemanticType::Discrete   => Scale::Discrete,
            SemanticType::Temporal   => Scale::Temporal,
        });

        // --- 2. RESOLVE Y CHANNEL ---
        // Mandatory: y_enc and its field are guaranteed to exist
        let y_enc = self.encoding.y.as_mut().unwrap();
        let y_dtype = self.data.df.schema().get(&y_enc.field).unwrap();
        
        y_enc.scale_type = Some(match interpret_semantic_type(y_dtype) {
            SemanticType::Continuous => Scale::Linear,
            SemanticType::Discrete   => Scale::Discrete,
            SemanticType::Temporal   => Scale::Temporal,
        });

        // --- 3. RESOLVE Y-ZERO BASELINE ---
        // Apply the Grammar of Graphics rule: Quantitative bars and areas should start at zero.
        if y_enc.scale_type == Some(Scale::Linear) {
            let mt = self.mark.as_ref().unwrap().mark_type();
            if ["area", "bar", "hist"].contains(&mt) {
                y_enc.zero = Some(true);
            }
        }

        // --- 4. RESOLVE OPTIONAL COLOR CHANNEL ---
        if let Some(ref mut color_enc) = self.encoding.color {
            // Field existence for color was also verified by check_schema
            let c_dtype = self.data.df.schema().get(&color_enc.field).unwrap();
            
            color_enc.scale_type = Some(match interpret_semantic_type(c_dtype) {
                SemanticType::Continuous => Scale::Linear,
                SemanticType::Discrete   => Scale::Discrete,
                SemanticType::Temporal   => Scale::Temporal,
            });
        }
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

        // 1. Identify the primary series to determine the semantic type
        let primary_series = self.data.column(field_name)?;
        let semantic_type = interpret_semantic_type(primary_series.dtype());

        match semantic_type {
            // --- DISCRETE: Categorical labels (X-axis of ErrorBars, etc.) ---
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

            // --- TEMPORAL: Time-based ranges ---
            SemanticType::Temporal => {
                let min_ns = primary_series.min::<i64>()?.unwrap_or(0);
                let max_ns = primary_series.max::<i64>()?.unwrap_or(0);
                let start_dt = time::OffsetDateTime::from_unix_timestamp_nanos(min_ns as i128)
                    .map_err(|_| ChartonError::Data("Invalid start timestamp".into()))?;
                let end_dt = time::OffsetDateTime::from_unix_timestamp_nanos(max_ns as i128)
                    .map_err(|_| ChartonError::Data("Invalid end timestamp".into()))?;
                Ok(ScaleDomain::Temporal(start_dt, end_dt))
            }

            // --- CONTINUOUS: Quantitative ranges (The heart of ErrorBar scaling) ---
            SemanticType::Continuous => {
                let mut global_min = f64::INFINITY;
                let mut global_max = f64::NEG_INFINITY;
                let mut found_data = false;

                // Build a list of candidate columns that contribute to this channel's domain
                let mut columns_to_scan = vec![field_name.to_string()];

                // Add potential auxiliary columns for Y-axis
                if channel == Channel::Y {
                    // Standard secondary field
                    if let Some(y2_enc) = &self.encoding.y2 {
                        columns_to_scan.push(y2_enc.field.clone());
                    }

                    // Specific to errobar chart
                    columns_to_scan.push(format!("{}_{}_min", TEMP_SUFFIX, field_name));
                    columns_to_scan.push(format!("{}_{}_max", TEMP_SUFFIX, field_name));
                }

                // Scan all candidate columns and calculate the union of their extents
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

                // Fallback if no valid data was found (e.g., temp columns not yet generated)
                if !found_data {
                    global_min = primary_series.min::<f64>()?.unwrap_or(0.0);
                    global_max = primary_series.max::<f64>()?.unwrap_or(1.0);
                }

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