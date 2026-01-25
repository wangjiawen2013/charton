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
//pub mod errorbar_chart;

use crate::core::data::*;
use crate::core::layer::{Layer, MarkRenderer};
use crate::encode::{Channel, Encoding, IntoEncoding};
use crate::scale::{Scale, ScaleDomain, ScaleTrait, Expansion};
use crate::error::ChartonError;
use crate::mark::Mark;
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

        // Automatically convert numeric types to f32
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

        // Validate that DataFrame only contains supported data types
        let schema = self.data.df.schema();
        for (col_name, dtype) in schema.iter() {
            use polars::datatypes::DataType::*;
            match dtype {
                // Supported numeric types
                UInt8 | UInt16 | UInt32 | UInt64 | Int8 | Int16 | Int32 | Int64 | Int128
                | Float32 | Float64 | String => {
                    // These types are supported, continue
                }
                // Unsupported types
                _ => {
                    return Err(ChartonError::Data(format!(
                        "Column '{}' has unsupported data type {:?}. Only numeric types and String are supported.",
                        col_name, dtype
                    )));
                }
            }
        }

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

        // Add type checking for size encoding - must be continuous (f32)
        if let Some(size_enc) = &self.encoding.size {
            // we've already converted all numeric types to f32 when building the chart
            expected_types.insert(size_enc.field.as_str(), vec![DataType::Float32]);
        }

        // Add type checking for errorbar charts - y and y2 encodings must be f32 (continuous)
        if mark.mark_type() == "errorbar" {
            // we've already converted all numeric types to f32 when building the chart
            expected_types.insert(
                self.encoding.y.as_ref().unwrap().field.as_str(),
                vec![DataType::Float32],
            );

            // If y2 encoding exists, it must also be f32
            if let Some(y2_encoding) = &self.encoding.y2 {
                expected_types.insert(y2_encoding.field.as_str(), vec![DataType::Float32]);
            }
        }

        // Add type checking for histogram charts - x encoding must be f32 (continuous)
        if mark.mark_type() == "hist" {
            active_fields
                .retain(|&field| field != self.encoding.y.as_ref().unwrap().field.as_str());
            // we've already converted all numeric types to f32 when building the chart
            expected_types.insert(
                self.encoding.x.as_ref().unwrap().field.as_str(),
                vec![DataType::Float32],
            );
        }

        // Add type checking for rect charts - color encoding must be f32 (continuous) for proper color mapping
        if mark.mark_type() == "rect" {
            // we've already converted all numeric types to f32 when building the chart
            expected_types.insert(
                self.encoding.color.as_ref().unwrap().field.as_str(),
                vec![DataType::Float32],
            );
        }

        // Add type checking for boxplot charts - y encoding must be f32 (continuous)
        if mark.mark_type() == "boxplot" {
            // we've already converted all numeric types to f32 when building the chart
            // Boxplot requires x to be discrete and y to be continuous (numeric)
            expected_types.insert(
                self.encoding.x.as_ref().unwrap().field.as_str(),
                vec![DataType::String],
            );
            expected_types.insert(
                self.encoding.y.as_ref().unwrap().field.as_str(),
                vec![DataType::Float32],
            );
        }

        // Add type checking for bar charts - y encoding must be f32 (continuous)
        if mark.mark_type() == "bar" {
            // we've already converted all numeric types to f32 when building the chart
            // Boxplot requires x to be discrete and y to be continuous (numeric)
            expected_types.insert(
                self.encoding.x.as_ref().unwrap().field.as_str(),
                vec![DataType::String],
            );
            expected_types.insert(
                self.encoding.y.as_ref().unwrap().field.as_str(),
                vec![DataType::Float32],
            );
        }

        // Add type checking for rule charts - y and y2 encodings must be f32 (continuous)
        if mark.mark_type() == "rule" {
            // we've already converted all numeric types to f32 when building the chart
            expected_types.insert(
                self.encoding.y.as_ref().unwrap().field.as_str(),
                vec![DataType::Float32],
            );

            // If y2 encoding exists, it must also be f32
            if let Some(y2_encoding) = &self.encoding.y2 {
                expected_types.insert(y2_encoding.field.as_str(), vec![DataType::Float32]);
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

        // Set default scale types based on data types
        if let Some(ref mut x_encoding) = self.encoding.x {
            if x_encoding.scale_type.is_none() {
                let dtype = self.data.df.schema().get(&x_encoding.field).unwrap();
                let semantic_type = interpret_semantic_type(dtype);
                x_encoding.scale_type = match semantic_type {
                    SemanticType::Continuous => Some(Scale::Linear),
                    SemanticType::Discrete => Some(Scale::Discrete),
                    SemanticType::Temporal => Some(Scale::Temporal),
                };
            }
        }

        if let Some(ref mut y_encoding) = self.encoding.y {
            if y_encoding.scale_type.is_none() {
                let dtype = self.data.df.schema().get(&y_encoding.field).unwrap();
                let semantic_type = interpret_semantic_type(dtype);
                y_encoding.scale_type = match semantic_type {
                    SemanticType::Continuous => Some(Scale::Linear),
                    SemanticType::Discrete => Some(Scale::Discrete),
                    SemanticType::Temporal => Some(Scale::Temporal),
                };
            }
        }

        if let Some(ref mut color_encoding) = self.encoding.color {
            if color_encoding.scale_type.is_none() {
                let dtype = self.data.df.schema().get(&color_encoding.field).unwrap();
                let semantic_type = interpret_semantic_type(dtype);
                color_encoding.scale_type = match semantic_type {
                    SemanticType::Continuous => Some(Scale::Linear),
                    SemanticType::Discrete => Some(Scale::Discrete),
                    SemanticType::Temporal => Some(Scale::Temporal),
                };
            }
        }

        Ok(self)
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

    /// Calculates the raw data boundaries for a specific visual channel.
    /// 
    /// This method performs the "Discovery" phase of the rendering pipeline. 
    /// It translates low-level Polars data types into high-level visual domains
    /// (Continuous or Discrete) by interpreting the column's [SemanticType].
    ///
    /// # Parameters
    /// * `channel` - The visual aesthetic (X, Y, Color, etc.) to calculate bounds for.
    ///
    /// # Returns
    /// * `ScaleDomain` - Either a (min, max) pair for Quantitative/Temporal data,
    ///                   or a list of unique strings for Nominal data.
    fn get_data_bounds(&self, channel: Channel) -> Result<ScaleDomain, ChartonError> {
        // 1. Identify which data field is mapped to this channel 
        let field_name = self.encoding.get_field_by_channel(channel).ok_or_else(|| {
            ChartonError::Data(format!("No field mapped to channel {:?}", channel))
        })?;

        // 2. Access the column from the internal Polars DataFrame
        let series = self.data.column(&field_name)?;
        
        // 3. Interpret the "Semantic Meaning" (Physical Type -> Visual Intent)
        let semantic_type = interpret_semantic_type(series.dtype());

        match semantic_type {
            // --- CONTINUOUS: Numeric ranges ---
            SemanticType::Continuous => {
                let min_val = series.min::<f32>()?.unwrap_or(0.0);
                let max_val = series.max::<f32>()?.unwrap_or(1.0);

                // Aesthetic Rule: Bar-like marks (bars, areas, histograms) 
                // typically require the axis to include zero to avoid visual bias.
                let is_bar_like = self.mark.as_ref().map_or(false, |m| {
                    matches!(m.mark_type(), "bar" | "area" | "hist")
                });
                
                // Retrieve the 'zero' preference from encoding
                let force_zero = self.encoding.get_zero_by_channel(channel);

                let (low, high) = if is_bar_like || force_zero {
                    (min_val.min(0.0), max_val.max(0.0))
                } else {
                    (min_val, max_val)
                };

                Ok(ScaleDomain::Continuous(low, high))
            }

            // --- DISCRETE: Categorical unique labels ---
            SemanticType::Discrete => {
                // Use unique_stable to preserve appearance order
                let labels = series
                    .unique_stable()?
                    .cast(&DataType::String)? 
                    .str()?
                    .into_no_null_iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();
                
                Ok(ScaleDomain::Discrete(labels))
            }

            // --- TEMPORAL: Date/Time converted to continuous numeric range ---
            SemanticType::Temporal => {
                let min_val = series.min::<f64>()?.unwrap_or(0.0) as f32;
                let max_val = series.max::<f64>()?.unwrap_or(1.0) as f32;
                
                Ok(ScaleDomain::Continuous(min_val, max_val))
            }
        }
    } 
}