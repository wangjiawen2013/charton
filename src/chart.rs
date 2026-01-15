pub mod point_chart;

use crate::scale::{Expansion, Scale, ScaleDomain};
use crate::core::layer::{MarkRenderer, Layer};
use crate::core::utils::estimate_text_width;
use crate::core::data::*;
use crate::core::utils::render_constants::*;
use crate::encode::{Encoding, IntoEncoding};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::theme::Theme;
use crate::visual::color::{ColorMap, ColorPalette};
use polars::prelude::*;

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
/// * `mark_cmap` - Color map used for continuous color encoding
/// * `mark_palette` - Color palette used for discrete color encoding
pub struct Chart<T: Mark> {
    pub(crate) data: DataFrameSource,
    pub(crate) encoding: Encoding,
    pub(crate) mark: Option<T>,
    pub(crate) mark_cmap: ColorMap,
    pub(crate) mark_palette: ColorPalette,
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
            mark_cmap: ColorMap::Viridis,
            mark_palette: ColorPalette::Tab10,
        };

        // Automatically convert numeric types to f64
        chart.data = convert_numeric_types(chart.data.clone())?;

        Ok(chart)
    }

    /// Set the color map for the chart
    ///
    /// Defines the color mapping function used for continuous color encodings. The color map
    /// translates data values to colors on a continuous spectrum. Common options include
    /// Viridis, Plasma, Inferno, and other perceptually uniform colormaps.
    ///
    /// # Arguments
    ///
    /// * `cmap` - The ColorMap to use for continuous color encoding
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_color_map(mut self, cmap: ColorMap) -> Self {
        self.mark_cmap = cmap;
        self
    }

    /// Set the color palette for the chart
    ///
    /// Defines the color palette used for discrete color encodings. The palette provides
    /// a set of distinct colors for categorical data. Common options include Tab10, Set1,
    /// and other colorblind-friendly palettes.
    ///
    /// # Arguments
    ///
    /// * `palette` - The ColorPalette to use for discrete color encoding
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_color_palette(mut self, palette: ColorPalette) -> Self {
        self.mark_palette = palette;
        self
    }

    /// Set both color map and palette at the same time
    ///
    /// Convenience method to set both the continuous color map and discrete color palette
    /// in a single call. This is useful when you want to configure both color encoding
    /// schemes simultaneously.
    ///
    /// # Arguments
    ///
    /// * `cmap` - The ColorMap to use for continuous color encoding
    /// * `palette` - The ColorPalette to use for discrete color encoding
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_colors(mut self, cmap: ColorMap, palette: ColorPalette) -> Self {
        self.mark_cmap = cmap;
        self.mark_palette = palette;
        self
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
            if x_encoding.scale.is_none() {
                let dtype = self.data.df.schema().get(&x_encoding.field).unwrap();
                let data_type_category = determine_data_type_category(dtype);
                x_encoding.scale = match data_type_category {
                    DataTypeCategory::Continuous => Some(Scale::Linear),
                    DataTypeCategory::Discrete => Some(Scale::Discrete),
                };
            }
        }

        if let Some(ref mut y_encoding) = self.encoding.y {
            if y_encoding.scale.is_none() {
                let dtype = self.data.df.schema().get(&y_encoding.field).unwrap();
                let data_type_category = determine_data_type_category(dtype);
                y_encoding.scale = match data_type_category {
                    DataTypeCategory::Continuous => Some(Scale::Linear),
                    DataTypeCategory::Discrete => Some(Scale::Discrete),
                };
            }
        }

        if let Some(ref mut color_encoding) = self.encoding.color {
            if color_encoding.scale.is_none() {
                let dtype = self.data.df.schema().get(&color_encoding.field).unwrap();
                let data_type_category = determine_data_type_category(dtype);
                color_encoding.scale = match data_type_category {
                    DataTypeCategory::Continuous => Some(Scale::Linear),
                    DataTypeCategory::Discrete => Some(Scale::Discrete),
                };
            }
        }

        Ok(self)
    }

    pub(crate) fn get_x_scale_type(&self) -> Option<Scale> {
        // For charts that don't have x encoding (like pie charts), return None
        if self.encoding.x.is_none() {
            return None;
        }

        // If x encoding exists, return the scale from the encoding
        let x_encoding = self.encoding.x.as_ref().unwrap();
        x_encoding.scale.clone()
    }

    pub(crate) fn get_y_scale_type(&self) -> Option<Scale> {
        // For charts that don't have y encoding (like pie charts), return None
        if self.encoding.y.is_none() {
            return None;
        }

        // If y encoding exists, return the scale from the encoding
        let y_encoding = self.encoding.y.as_ref().unwrap();
        y_encoding.scale.clone()
    }

    pub(crate) fn get_color_scale_type(&self) -> Option<Scale> {
        // For charts that don't have encoding, return None
        if self.encoding.color.is_none() {
            return None;
        }

        // If color encoding exists, return the scale from the encoding
        let color_encoding = self.encoding.color.as_ref().unwrap();
        color_encoding.scale.clone()
    }

    pub(crate) fn get_shape_scale_type(&self) -> Option<Scale> {
        // For charts that don't have shape encoding, return None
        if self.encoding.shape.is_none() {
            None
        } else {
            Some(Scale::Discrete)
        }
    }

    pub(crate) fn get_size_scale_type(&self) -> Option<Scale> {
        // For charts that don't have size encoding, return None
        if self.encoding.size.is_none() {
            return None;
        }

        // If size encoding exists, return the scale from the encoding
        let size_encoding = self.encoding.size.as_ref().unwrap();
        Some(size_encoding.scale.clone())
    }
}

// Implementation of Layer trait for Chart<T> allowing any chart to be used as a layer
impl<T> Layer for Chart<T>
where
    T: crate::mark::Mark,
    Chart<T>: MarkRenderer,
{
    fn requires_axes(&self) -> bool {
        // For pie charts (which use MarkArc), don't show axes
        if self.mark.as_ref().map(|m| m.mark_type()) == Some("arc") {
            false
        } else {
            // For all other chart types, show axes by default
            true
        }
    }

    fn preferred_x_axis_expanding(&self) -> Expansion {
        match self.mark.as_ref().map(|m| m.mark_type()) {
            Some("rect") => {
                let x_scale_type = self.get_x_scale_type();
                let x_is_discrete = matches!(x_scale_type.as_ref(), Some(Scale::Discrete));

                if !x_is_discrete {
                    Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) }
                } else {
                    Expansion { mult: (0.0, 0.0), add: (0.5, 0.5) }
                }
            }
            Some("boxplot") | Some("bar") => Expansion { mult: (0.0, 0.0), add: (0.6, 0.6) },
            Some("hist") => Expansion { mult: (0.0, 0.0), add: (0.6, 0.6) },
            _ => Expansion::default(),
        }
    }

    fn preferred_y_axis_expanding(&self) -> Expansion {
        match self.mark.as_ref().map(|m| m.mark_type()) {
            Some("rect") => {
                let y_scale_type = self.get_y_scale_type();
                let y_is_discrete = matches!(y_scale_type.as_ref(), Some(Scale::Discrete));

                if !y_is_discrete {
                    Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) }
                } else {
                    Expansion { mult: (0.0, 0.0), add: (0.5, 0.5) }
                }
            }
            Some("bar") | Some("area") => {
                // For bar and area charts, if minimum value >= 0, padding should be different
                let y_encoding = self.encoding.y.as_ref().unwrap();
                let y_series = self.data.column(&y_encoding.field).unwrap();
                let min_val = y_series.min::<f64>().unwrap().unwrap();
                if min_val >= 0.0 {
                    Expansion { mult: (0.0, 0.05), add: (0.0, 0.0) } // No lower padding, default upper
                } else {
                    Expansion::default() // Default 5% padding on both sides
                }
            }
            Some("boxplot") | Some("hist") => Expansion { mult: (0.0, 0.0), add: (0.6, 0.6) },
            _ => Expansion::default(),
        }
    }

    /// Get the X continuous bounds.
    fn get_x_continuous_bounds(&self) -> Result<(f64, f64), ChartonError> {
        if self.encoding.x.is_none() {
            return Ok((0.0, 1.0));
        }

        let x_encoding = self.encoding.x.as_ref().expect("X encoding should exist");
        let x_series = self.data.column(&x_encoding.field)?;

        let min_val = x_series.min::<f64>()?.ok_or_else(|| {
            ChartonError::Data(format!("Failed to calculate min for x-axis: {}", x_encoding.field))
        })?;
        let max_val = x_series.max::<f64>()?.ok_or_else(|| {
            ChartonError::Data(format!("Failed to calculate max for x-axis: {}", x_encoding.field))
        })?;

        // Same logic as Y: Force include zero for bar/area/hist
        let is_bar_like = self.mark.as_ref().map_or(false, |m| {
            matches!(m.mark_type(), "bar" | "area" | "hist")
        });

        let (final_min, final_max) = if is_bar_like || x_encoding.zero == Some(true) {
            (min_val.min(0.0), max_val.max(0.0))
        } else {
            (min_val, max_val)
        };

        Ok((final_min, final_max))
    }

    fn get_x_discrete_tick_labels(&self) -> Result<Option<Vec<String>>, ChartonError> {
        // For charts that don't have x encoding (like pie charts), return None
        if self.encoding.x.is_none() {
            return Ok(None);
        }

        let x_encoding = self.encoding.x.as_ref().expect("X encoding should exist");
        let unique_labels = self
            .data
            .column(&x_encoding.field)?
            .unique_stable()?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        Ok(Some(unique_labels))
    }

    fn get_x_encoding_field(&self) -> Option<String> {
        self.encoding.x.as_ref().map(|x| x.field.clone())
    }

    fn get_x_scale_type_from_layer(&self) -> Option<Scale> {
        self.get_x_scale_type()
    }

    /// Get the Y continuous bounds, following the color-axis implementation pattern.
    fn get_y_continuous_bounds(&self) -> Result<(f64, f64), ChartonError> {
        if self.encoding.y.is_none() {
            return Ok((0.0, 1.0));
        }

        let y_encoding = self.encoding.y.as_ref().expect("Y encoding should exist");
        let y_series = self.data.column(&y_encoding.field)?;

        let min_val = y_series.min::<f64>()?.ok_or_else(|| {
            ChartonError::Data(format!("Failed to calculate min for y-axis: {}", y_encoding.field))
        })?;
        let max_val = y_series.max::<f64>()?.ok_or_else(|| {
            ChartonError::Data(format!("Failed to calculate max for y-axis: {}", y_encoding.field))
        })?;

        // Simplified zero-inclusion: 
        // Force true for bar/area/hist, otherwise follow user encoding.
        let is_bar_like = self.mark.as_ref().map_or(false, |m| {
            matches!(m.mark_type(), "bar" | "area" | "hist")
        });

        let (final_min, final_max) = if is_bar_like || y_encoding.zero == Some(true) {
            (min_val.min(0.0), max_val.max(0.0))
        } else {
            (min_val, max_val)
        };

        Ok((final_min, final_max))
    }

    fn get_y_discrete_tick_labels(&self) -> Result<Option<Vec<String>>, ChartonError> {
        // For charts that don't have y encoding (like pie charts), return None
        if self.encoding.y.is_none() {
            return Ok(None);
        }

        let y_encoding = self.encoding.y.as_ref().expect("Y encoding should exist");
        let unique_labels = self
            .data
            .column(&y_encoding.field)?
            .unique_stable()?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        Ok(Some(unique_labels))
    }

    fn get_y_encoding_field(&self) -> Option<String> {
        self.encoding.y.as_ref().map(|y| y.field.clone())
    }

    fn get_y_scale_type_from_layer(&self) -> Option<Scale> {
        self.get_y_scale_type()
    }

/// Get the color continuous bounds, following the X-axis implementation pattern.
    fn get_color_continuous_bounds(&self) -> Result<Option<(f64, f64)>, ChartonError> {
        // For charts that don't have color encoding, return None 
        // (Note: Unlike X, we return None here so LayeredChart can skip it)
        if self.encoding.color.is_none() {
            return Ok(None);
        }

        let color_encoding = self.encoding.color.as_ref().expect("Color encoding should exist");
        let color_series = self.data.column(&color_encoding.field)?;
        
        let min_val = color_series.min::<f64>()?.ok_or_else(|| {
            ChartonError::Data(format!("Failed to calculate minimum value for color field: {}", color_encoding.field))
        })?;
        let max_val = color_series.max::<f64>()?.ok_or_else(|| {
            ChartonError::Data(format!("Failed to calculate maximum value for color field: {}", color_encoding.field))
        })?;

        Ok(Some((min_val, max_val)))
    }

    /// Get discrete color labels, following the X-axis implementation pattern.
    fn get_color_discrete_labels(&self) -> Result<Option<Vec<String>>, ChartonError> {
        // For charts that don't have color encoding, return None
        if self.encoding.color.is_none() {
            return Ok(None);
        }

        let color_encoding = self.encoding.color.as_ref().expect("Color encoding should exist");
        
        // Use unique_stable to preserve data order for the legend
        let unique_labels = self
            .data
            .column(&color_encoding.field)?
            .unique_stable()?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        if unique_labels.is_empty() {
            Ok(None)
        } else {
            Ok(Some(unique_labels))
        }
    }

    /// Simply return the scale type from encoding
    fn get_color_scale_type_from_layer(&self) -> Option<Scale> {
        self.get_color_scale_type()
    }

    // --- Get shape labels (Discrete)---
    fn get_shape_discrete_labels(&self) -> Result<Option<Vec<String>>, ChartonError> {
        if self.encoding.shape.is_none() {
            return Ok(None);
        }

        let shape_encoding = self.encoding.shape.as_ref().expect("Shape encoding should exist");
        let unique_labels = self
            .data
            .column(&shape_encoding.field)?
            .unique_stable()?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        Ok(Some(unique_labels))
    }

    fn get_shape_scale_type_from_layer(&self) -> Option<Scale> {
        self.get_shape_scale_type()
    }

    // --- Get size continuous bounds (Continuous) ---
    fn get_size_continuous_bounds(&self) -> Result<Option<(f64, f64)>, ChartonError> {
        if self.encoding.size.is_none() {
            return Ok(None);
        }

        let size_encoding = self.encoding.size.as_ref().expect("Size encoding should exist");
        let size_series = self.data.column(&size_encoding.field)?;
        
        let min_val = size_series.min::<f64>()?.ok_or_else(|| {
            ChartonError::Data(format!("Failed to calculate min for size field: {}", size_encoding.field))
        })?;
        let max_val = size_series.max::<f64>()?.ok_or_else(|| {
            ChartonError::Data(format!("Failed to calculate max for size field: {}", size_encoding.field))
        })?;

        Ok(Some((min_val, max_val)))
    }

    fn get_size_scale_type_from_layer(&self) -> Option<Scale> {
        self.get_size_scale_type()
    }

    /// Sets the resolved Scale type for a specific visual channel.
    /// This updates the internal encoding configuration to align with the global chart scale.
    fn set_scale_type(&mut self, channel: &str, scale: Scale) {
        match channel {
            "color" => {
                if let Some(ref mut color_encoding) = self.encoding.color {
                    color_encoding.scale = Some(scale);
                }
            }
            "size" => {
                if let Some(ref mut size_encoding) = self.encoding.size {
                    // Size usually uses a continuous scale (e.g., Linear)
                    size_encoding.scale = scale;
                }
            }
            // Note: Shape is typically fixed to Scale::Discrete in Grammar of Graphics,
            // but we can add handling here if your implementation requires dynamic shape scales.
            _ => {}
        }
    }

    /// Sets the resolved Data Domain for a specific visual channel.
    /// This "back-fills" the global domain calculated by LayeredChart into this specific layer.
    fn set_domain(&mut self, channel: &str, domain: ScaleDomain) {
        match channel {
            "color" => {
                if let Some(ref mut color_encoding) = self.encoding.color {
                    color_encoding.domain = Some(domain);
                }
            }
            "shape" => {
                if let Some(ref mut shape_encoding) = self.encoding.shape {
                    shape_encoding.domain = Some(domain);
                }
            }
            "size" => {
                if let Some(ref mut size_encoding) = self.encoding.size {
                    size_encoding.domain = Some(domain);
                }
            }
            // For x and y, the domains are usually managed by the Coordinate System,
            // but if your layers store them, add matching logic for "x" and "y" here.
            _ => {}
        }
    }

    /// Simply returns the internal encoding struct.
    fn get_encoding(&self) -> &Encoding {
        &self.encoding
    }

    /// Inspects the specific channel in the encoding and returns its scale.
    fn get_scale_type(&self, channel: &str) -> Option<Scale> {
        match channel {
            "color" => self.encoding.color.as_ref()?.scale.clone(),
            "shape" => Some(self.encoding.shape.as_ref()?.scale.clone()),
            "size" => Some(self.encoding.size.as_ref()?.scale.clone()),
            _ => None,
        }
    }

    /// Inspects the specific channel in the encoding and returns its domain.
    /// Note: This assumes the domain has been resolved (calculated from data) 
    /// during the chart's initialization or pre-render phase.
    fn get_domain(&self, channel: &str) -> Option<ScaleDomain> {
        match channel {
            "color" => self.encoding.color.as_ref()?.domain.clone(),
            "shape" => self.encoding.shape.as_ref()?.domain.clone(),
            "size" => self.encoding.size.as_ref()?.domain.clone(),
            _ => None,
        }
    }
}