use crate::scale::{Expansion, Scale};
use crate::core::layer::{MarkRenderer, LegendRenderer, Layer};
use crate::coordinate::cartesian::Cartesian2D;
use crate::data::*;
use crate::encode::encoding::{Encoding, IntoEncoding};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::render::axis_renderer::render_axes;
use crate::render::constants::render_constants::*;
use crate::render::utils::estimate_text_width;
use crate::theme::Theme;
use crate::visual::color::{ColorMap, ColorPalette};
use indexmap::IndexSet;
use polars::prelude::*;
use resvg;
use std::fmt::Write;

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

        Ok(self)
    }
}

// Implementation of Layer trait for Chart<T> allowing any chart to be used as a layer
impl<T> Layer for Chart<T>
where
    T: crate::mark::Mark,
    Chart<T>: MarkRenderer + LegendRenderer,
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
                let x_encoding = self.encoding.x.as_ref().unwrap();
                let x_series = self.data.column(&x_encoding.field).unwrap();
                let is_continuous = matches!(
                    crate::data::determine_data_type_category(x_series.dtype()),
                    crate::data::DataTypeCategory::Continuous
                );
                if is_continuous {
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
                let y_encoding = self.encoding.y.as_ref().unwrap();
                let y_series = self.data.column(&y_encoding.field).unwrap();
                let is_continuous = matches!(
                    crate::data::determine_data_type_category(y_series.dtype()),
                    crate::data::DataTypeCategory::Continuous
                );
                if is_continuous {
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

    fn get_x_continuous_bounds(&self) -> Result<(f64, f64), ChartonError> {
        // For charts that don't have x encoding (like pie charts), return default bounds
        if self.encoding.x.is_none() {
            return Ok((0.0, 1.0));
        }

        let x_encoding = self.encoding.x.as_ref().expect("X encoding should exist");
        let x_series = self.data.column(&x_encoding.field)?;
        let x_min_val = x_series.min::<f64>()?.ok_or_else(|| {
            ChartonError::Data("Failed to calculate minimum value for x-axis".to_string())
        })?;
        let x_max_val = x_series.max::<f64>()?.ok_or_else(|| {
            ChartonError::Data("Failed to calculate maximum value for x-axis".to_string())
        })?;

        // Handle chart-type-specific logic
        let (x_min_val, x_max_val) = match self.mark.as_ref().map(|m| m.mark_type()) {
            // Handle rect charts with bin size adjustment
            Some("rect") | Some("hist") => {
                // For rect charts with continuous data, adjust bounds by half bin size
                let unique_count = x_series.n_unique()?;
                let bin_size = (x_max_val - x_min_val) / (unique_count as f64);
                let half_bin_size = bin_size / 2.0;

                // Expand bounds by half bin size
                (x_min_val - half_bin_size, x_max_val + half_bin_size)
            }
            // Default case for all other chart types
            _ => (x_min_val, x_max_val),
        };

        // Handle zero-crossing logic
        let (final_min, final_max) = match x_encoding.zero {
            Some(true) => {
                // Force include zero
                (x_min_val.min(0.0), x_max_val.max(0.0))
            }
            _ => (x_min_val, x_max_val),
        };

        Ok((final_min, final_max))
    }

    fn get_y_continuous_bounds(&self) -> Result<(f64, f64), ChartonError> {
        // For charts that don't have y encoding (like pie charts), return default bounds
        if self.encoding.y.is_none() {
            return Ok((0.0, 1.0));
        }

        let y_encoding = self.encoding.y.as_ref().expect("Y encoding should exist");
        let y_series = self.data.column(&y_encoding.field)?;
        let y_min_val = y_series.min::<f64>()?.ok_or_else(|| {
            ChartonError::Data("Failed to calculate minimum value for y-axis".to_string())
        })?;
        let y_max_val = y_series.max::<f64>()?.ok_or_else(|| {
            ChartonError::Data("Failed to calculate maximum value for y-axis".to_string())
        })?;

        // Handle chart-type-specific logic with match
        let (y_min_val, y_max_val) = match self.mark.as_ref().map(|m| m.mark_type()) {
            // Handle errorbar charts specially - check for min/max columns
            Some("errorbar") => {
                // For errorbar charts, we need to consider the min/max values from the calculated columns
                let (y_min_field, y_max_field) = if let Some(y2_encoding) = &self.encoding.y2 {
                    // If y2 encoding exists, use y field for min and y2 field for max
                    (
                        self.encoding.y.as_ref().unwrap().field.clone(),
                        y2_encoding.field.clone(),
                    )
                } else {
                    // If no y2 encoding, fall back to auto-generated field names
                    (
                        self.encoding
                            .y
                            .as_ref()
                            .map(|y| format!("__charton_temp_{}_min", y.field))
                            .expect("Y encoding and column ymin required"),
                        self.encoding
                            .y
                            .as_ref()
                            .map(|y| format!("__charton_temp_{}_max", y.field))
                            .expect("Y encoding and column ymax required"),
                    )
                };

                // Get min values
                let y_min_series = self.data.column(&y_min_field)?;
                let y_min_val = y_min_series.min::<f64>()?.unwrap();

                // Get max values
                let y_max_series = self.data.column(&y_max_field)?;
                let y_max_val = y_max_series.max::<f64>()?.unwrap();

                (y_min_val, y_max_val)
            }
            // Handle rule charts specially - check for y2 column
            Some("rule") => {
                // For rule charts, we need to consider both y and y2 values
                // Add y2 ranges if y2 column exists
                if let Some(y2_encoding) = &self.encoding.y2 {
                    let y2_series = self.data.column(&y2_encoding.field)?;
                    let y2_min_val = y2_series.min::<f64>()?.unwrap();
                    let y2_max_val = y2_series.max::<f64>()?.unwrap();
                    let y_min_val = y_min_val.min(y2_min_val);
                    let y_max_val = y_max_val.max(y2_max_val);

                    (y_min_val, y_max_val)
                } else {
                    (y_min_val, y_max_val)
                }
            }
            // Handle stacked bars specially
            Some("bar") if y_encoding.stack && self.encoding.color.is_some() => {
                // For stacked bars, we need to calculate the sum of values for each group
                // and find the min/max of those sums
                let group_col = self.encoding.x.as_ref().unwrap().field.clone();

                // Create a DataFrame to work with
                let df = self.data.df.clone();

                // Group by the group column and sum the values
                let grouped_sums = df
                    .lazy()
                    .group_by_stable([col(group_col)])
                    .agg([col(&y_encoding.field).sum().alias("sum_values")])
                    .collect()?;

                if let Ok(sum_series) = grouped_sums.column("sum_values") {
                    let sum_series = sum_series.as_materialized_series();
                    let group_min = sum_series.min::<f64>()?.unwrap();
                    let group_max = sum_series.max::<f64>()?.unwrap();

                    (group_min, group_max)
                } else {
                    (y_min_val, y_max_val)
                }
            }
            // Handle rect charts with bin size adjustment
            Some("rect") => {
                // For rect charts with continuous data, adjust bounds by half bin size
                let unique_count = y_series.n_unique()?;
                let bin_size = (y_max_val - y_min_val) / (unique_count as f64);
                let half_bin_size = bin_size / 2.0;

                // Expand bounds by half bin size
                (y_min_val - half_bin_size, y_max_val + half_bin_size)
            }
            // Default case for all other chart types
            _ => (y_min_val, y_max_val),
        };

        // Handle zero-crossing logic
        let (final_min, final_max) = match y_encoding.zero {
            Some(true) => {
                // Force include zero
                (y_min_val.min(0.0), y_max_val.max(0.0))
            }
            Some(false) => {
                // Use data range as-is
                (y_min_val, y_max_val)
            }
            None => {
                let is_supported_mark = self.mark.as_ref().map(|m| {
                    m.mark_type() == "bar" || m.mark_type() == "hist" || m.mark_type() == "area"
                });
                if is_supported_mark.unwrap_or(false) {
                    (y_min_val.min(0.0), y_max_val.max(0.0))
                } else {
                    (y_min_val, y_max_val)
                }
            }
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

    fn get_x_encoding_field(&self) -> Option<String> {
        self.encoding.x.as_ref().map(|x| x.field.clone())
    }

    fn get_y_encoding_field(&self) -> Option<String> {
        self.encoding.y.as_ref().map(|y| y.field.clone())
    }

    /// Returns the data type of the field mapped to the X axis.
    /// Returns `None` if the X encoding is not defined.
    /// In Charton, the field must exists in the DataFrame if the encoding is present.
    fn get_x_data_type(&self) -> Option<polars::datatypes::DataType> {
        let x_encoding = self.encoding.x.as_ref()?;
        self.data.column(x_encoding.field.as_str()).ok().map(|series| series.dtype().clone())
    }

    /// Returns the data type of the field mapped to the Y axis.
    /// Returns `None` if the Y encoding is not defined.
    /// In Charton, the field must exists in the DataFrame if the encoding is present.
    fn get_y_data_type(&self) -> Option<polars::datatypes::DataType> {
        let y_encoding = self.encoding.y.as_ref()?;
        self.data.column(y_encoding.field.as_str()).ok().map(|series| series.dtype().clone())
    }

    fn calculate_legend_width(
        &self,
        theme: &Theme,
        chart_height: f64,
        top_margin: f64,
        bottom_margin: f64,
    ) -> f64 {
        let mut max_legend_width = 0.0;

        // Check color legend width
        if let Some(color_enc) = &self.encoding.color {
            let color_series = self
                .data
                .column(&color_enc.field)
                .expect("Color column should exist");

            // Determine if the color encoding should use a continuous scale (like a color ramp)
            // or a discrete scale (like a color palette) by checking the data type category.
            let is_continuous = matches!(
                crate::data::determine_data_type_category(color_series.dtype()),
                crate::data::DataTypeCategory::Continuous
            );

            if !is_continuous {
                // For discrete color legend, calculate actual width needed
                let unique = color_series
                    .unique_stable()
                    .expect("Failed to calculate legend width")
                    .str()
                    .expect("Failed to calculate legend width")
                    .into_no_null_iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();

                // Multi-column legend setup
                let plot_h = (1.0 - bottom_margin - top_margin) * chart_height;
                let available_vertical_space = plot_h - 30.0; // Subtract space for title
                let max_items_per_column =
                    (available_vertical_space / ITEM_HEIGHT).floor() as usize;

                // Ensure we have at least one item per column and respect the maximum items per column
                let max_items_per_column = max_items_per_column.clamp(1, MAX_ITEMS_PER_COLUMN);

                let total_items = unique.len();
                let columns_needed =
                    ((total_items as f64) / (max_items_per_column as f64)).ceil() as usize;

                // Estimate max label width (need to define estimate_text_width or import it)
                let max_label_width = unique
                    .iter()
                    .map(|label| estimate_text_width(label, theme.tick_label_font_size as f64))
                    .fold(0.0, f64::max);

                // Calculate total width needed (using constants)
                let column_width =
                    COLOR_BOX_SIZE + COLOR_BOX_SPACING + max_label_width + LABEL_PADDING;
                max_legend_width = (column_width * columns_needed as f64) +
                                (COLUMN_SPACING * (columns_needed - 1) as f64) + // COLUMN_SPACING
                                10.0; // Additional padding
            } else {
                max_legend_width = 100.0; // Colorbar width + padding
            }
        }

        // Check size legend width
        if self.encoding.size.is_some() {
            let size_legend_width = 100.0; // Approximate size legend width
            max_legend_width = max_legend_width.max(size_legend_width);
        }

        // Check shape legend width
        if let Some(shape_enc) = &self.encoding.shape {
            let shape_series = self
                .data
                .column(&shape_enc.field)
                .expect("Shape column should exist");
            let unique_shapes_series = shape_series
                .unique_stable()
                .expect("Failed to calculate unique shape values");

            let unique_shapes: Vec<String> = unique_shapes_series
                .str()
                .expect("Shape values must be strings")
                .into_no_null_iter()
                .map(|s| s.to_string())
                .collect();

            // Multi-column legend setup
            let plot_h = (1.0 - bottom_margin - top_margin) * chart_height;
            let available_vertical_space = plot_h - 30.0; // Subtract space for title
            let max_items_per_column = (available_vertical_space / ITEM_HEIGHT).floor() as usize;

            // Ensure we have at least one item per column and respect the maximum items per column
            let max_items_per_column = max_items_per_column.clamp(1, MAX_ITEMS_PER_COLUMN);

            let total_items = unique_shapes.len();
            let columns_needed =
                ((total_items as f64) / (max_items_per_column as f64)).ceil() as usize;

            // Estimate max label width
            let max_label_width = unique_shapes
                .iter()
                .map(|label| estimate_text_width(label, theme.tick_label_font_size as f64))
                .fold(0.0, f64::max);

            // Calculate total width needed
            let column_width = COLOR_BOX_SIZE + COLOR_BOX_SPACING + max_label_width + LABEL_PADDING;
            let shape_legend_width = (column_width * columns_needed as f64) +
                                    (COLUMN_SPACING * (columns_needed - 1) as f64) + // COLUMN_SPACING
                                    10.0; // Additional padding
            max_legend_width = max_legend_width.max(shape_legend_width);
        }

        // Calculate legend title widths and take maximum
        let mut title_widths = Vec::new();

        // Determine the font size to use for legend titles
        let legend_title_font_size = theme.legend_font_size.unwrap_or(theme.label_font_size) as f64;

        // Color legend title (field name)
        if let Some(color_enc) = &self.encoding.color {
            let title_width = estimate_text_width(&color_enc.field, legend_title_font_size);
            title_widths.push(title_width + 20.0); // Add padding
        }

        // Size legend title (field name)
        if let Some(size_enc) = &self.encoding.size {
            let title_width = estimate_text_width(&size_enc.field, legend_title_font_size);
            title_widths.push(title_width + 20.0); // Add padding
        }

        // Shape legend title (field name)
        if let Some(shape_enc) = &self.encoding.shape {
            let title_width = estimate_text_width(&shape_enc.field, legend_title_font_size);
            title_widths.push(title_width + 20.0); // Add padding
        }

        // Find maximum title width
        if !title_widths.is_empty() {
            let max_title_width = title_widths.into_iter().fold(0.0, f64::max);
            max_legend_width = max_legend_width.max(max_title_width);
        }

        max_legend_width
    }
}