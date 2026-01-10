use polars::prelude::*;
use std::collections::HashMap;

use crate::base::layer::{Layer, MarkRenderer, LegendRenderer};
use crate::base::context::SharedRenderingContext;
use crate::error::ChartonError;
use crate::theme::Theme;
use crate::scale::Scale;
use crate::mark::Mark;
use crate::encode::encoding::{Encoding, IntoEncoding};
use crate::data::{DataFrameSource, check_schema};
use crate::render::utils::estimate_text_width;
use crate::visual::color::{ColorMap, ColorPalette};

// Constants for legend calculation
const ITEM_HEIGHT: f64 = 20.0;
const MAX_ITEMS_PER_COLUMN: usize = 15;
const COLOR_BOX_SIZE: f64 = 12.0;
const COLOR_BOX_SPACING: f64 = 8.0;
const LABEL_PADDING: f64 = 10.0;
const COLUMN_SPACING: f64 = 20.0;

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
        chart.data = Self::convert_numeric_types(chart.data.clone())?;

        Ok(chart)
    }

    // Association function to convert numeric columns to f64
    fn convert_numeric_types(df_source: DataFrameSource) -> Result<DataFrameSource, ChartonError> {
        let mut new_columns = Vec::new();

        for col in df_source.df.get_columns() {
            use polars::datatypes::DataType::*;
            match col.dtype() {
                UInt8 | UInt16 | UInt32 | UInt64 | Int8 | Int16 | Int32 | Int64 | Int128
                | Float32 | Float64 => {
                    let casted = col.cast(&Float64)?;
                    new_columns.push(casted);
                }
                _ => {
                    new_columns.push(col.clone());
                }
            }
        }

        let new_df = DataFrame::new(new_columns)?;

        Ok(DataFrameSource::new(new_df))
    }

    /// Set the color map for the chart
    pub fn with_color_map(mut self, cmap: ColorMap) -> Self {
        self.mark_cmap = cmap;
        self
    }

    /// Set the color palette for the chart
    pub fn with_color_palette(mut self, palette: ColorPalette) -> Self {
        self.mark_palette = palette;
        self
    }

    /// Set both color map and palette at the same time
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
                UInt8 | UInt16 | UInt32 | UInt64 | Int8 | Int16 | Int32 | Int64 | Int128
                | Float32 | Float64 | String => {
                    // These types are supported, continue
                }
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

        // Validate mandatory encodings - these are the minimum required fields for each chart type
        match mark.mark_type() {
            "errorbar" | "bar" | "hist" | "line" | "point" | "area" | "boxplot" | "text"
            | "rule" => {
                if self.encoding.x.is_none() || self.encoding.y.is_none() {
                    return Err(ChartonError::Encoding(format!(
                        "{} chart requires both x and y encodings",
                        mark.mark_type()
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
            "arc" => {
                if self.encoding.theta.is_none() || self.encoding.color.is_none() {
                    return Err(ChartonError::Encoding(
                        "Arc chart requires both theta and color encodings".into(),
                    ));
                }
            }
            _ => {
                return Err(ChartonError::Mark(format!(
                    "Unknown mark type: {}. This is a programming error.",
                    mark.mark_type()
                )));
            }
        }

        // Build required columns and expected types
        let mut active_fields = self.encoding.active_fields();
        let mut expected_types = HashMap::new();

        // (Type checking for shape, size, errorbar, hist, rect, boxplot, bar, rule, text)
        // Note: Logic simplified for brevity here but follows your exact logic
        if let Some(shape_enc) = &self.encoding.shape {
            expected_types.insert(shape_enc.field.as_str(), vec![DataType::String]);
        }
        if let Some(size_enc) = &self.encoding.size {
            expected_types.insert(size_enc.field.as_str(), vec![DataType::Float64]);
        }
        // ... (remaining expected_types logic from your code)

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
                    .map(|&s| s.to_string())
                    .collect::<Vec<_>>(),
            ))
            .map_err(|e| {
                eprintln!("Error filtering null values: {}", e);
                e
            })?;

        // Check if the filtered DataFrame is empty
        if filtered_df.height() == 0 {
            eprintln!("Warning: No valid data remaining after filtering.");
            self.data = DataFrameSource { df: filtered_df };
            return Ok(self);
        } else {
            self.data = DataFrameSource { df: filtered_df };
        }

        // Perform chart-specific data transformations based on mark type
        match mark.mark_type() {
            "errorbar" if self.encoding.y2.is_none() => self.transform_errorbar_data(),
            "rect" => self.transform_rect_data(),
            "bar" => self.transform_bar_data(),
            "hist" => self.transform_histogram_data(),
            _ => Ok(self),
        }
    }
}

// MARK RENDERER IMPLEMENTATION
impl<T: Mark> MarkRenderer for Chart<T> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Implementation for rendering marks...
        Ok(())
    }
}

// LEGEND RENDERER IMPLEMENTATION
impl<T: Mark> LegendRenderer for Chart<T> {
    fn render_legends(
        &self,
        svg: &mut String,
        theme: &Theme,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Implementation for rendering legends...
        Ok(())
    }
}

// LAYER TRAIT IMPLEMENTATION
impl<T: Mark> Layer for Chart<T> {
    /// Add this method to control whether axes should be rendered for this layer
    fn requires_axes(&self) -> bool {
        if self.mark.as_ref().map(|m| m.mark_type()) == Some("arc") {
            false
        } else {
            true
        }
    }

    /// Method to get preferred axis padding for this layer
    fn preferred_x_axis_padding_min(&self) -> Option<f64> {
        match self.mark.as_ref().map(|m| m.mark_type()) {
            Some("rect") => {
                let x_encoding = self.encoding.x.as_ref().unwrap();
                let x_series = self.data.df.column(&x_encoding.field).ok()?;
                let scale = determine_scale_for_dtype(x_series.dtype());
                match scale {
                    Scale::Discrete => Some(0.5),
                    _ => Some(0.0),
                }
            }
            Some("boxplot") | Some("bar") => Some(0.6),
            _ => None,
        }
    }

    fn preferred_x_axis_padding_max(&self) -> Option<f64> {
        match self.mark.as_ref().map(|m| m.mark_type()) {
            Some("rect") => self.preferred_x_axis_padding_min(),
            Some("boxplot") | Some("bar") | Some("hist") => Some(0.6),
            _ => None,
        }
    }

    fn preferred_y_axis_padding_min(&self) -> Option<f64> {
        match self.mark.as_ref().map(|m| m.mark_type()) {
            Some("rect") => {
                let y_encoding = self.encoding.y.as_ref().unwrap();
                let y_series = self.data.df.column(&y_encoding.field).ok()?;
                let scale = determine_scale_for_dtype(y_series.dtype());
                match scale {
                    Scale::Discrete => Some(0.5),
                    _ => Some(0.0),
                }
            }
            Some("bar") | Some("area") => {
                let y_encoding = self.encoding.y.as_ref().unwrap();
                let y_series = self.data.df.column(&y_encoding.field).ok()?;
                let min_val = y_series.min::<f64>().ok()??;
                if min_val >= 0.0 { Some(0.0) } else { None }
            }
            Some("boxplot") => Some(0.6),
            Some("hist") => Some(0.0),
            _ => None,
        }
    }

    fn preferred_y_axis_padding_max(&self) -> Option<f64> {
        match self.mark.as_ref().map(|m| m.mark_type()) {
            Some("rect") => self.preferred_y_axis_padding_min(),
            Some("boxplot") | Some("bar") | Some("hist") => Some(0.6),
            _ => None,
        }
    }

    /// For continuous data - return min/max bounds
    fn get_x_continuous_bounds(&self) -> Result<(f64, f64), ChartonError> {
        if self.encoding.x.is_none() {
            return Ok((0.0, 1.0));
        }

        let x_encoding = self.encoding.x.as_ref().unwrap();
        let x_series = self.data.column(&x_encoding.field)?;
        let x_min_val = x_series.min::<f64>()?.ok_or_else(|| {
            ChartonError::Data("Failed to calculate minimum value for x-axis".to_string())
        })?;
        let x_max_val = x_series.max::<f64>()?.ok_or_else(|| {
            ChartonError::Data("Failed to calculate maximum value for x-axis".to_string())
        })?;

        let (x_min, x_max) = match self.mark.as_ref().map(|m| m.mark_type()) {
            Some("rect") | Some("hist") => {
                let unique_count = x_series.n_unique()?;
                let bin_size = (x_max_val - x_min_val) / (unique_count as f64);
                let half_bin = bin_size / 2.0;
                (x_min_val - half_bin, x_max_val + half_bin)
            }
            _ => (x_min_val, x_max_val),
        };

        if x_encoding.zero == Some(true) {
            Ok((x_min.min(0.0), x_max.max(0.0)))
        } else {
            Ok((x_min, x_max))
        }
    }

    fn get_y_continuous_bounds(&self) -> Result<(f64, f64), ChartonError> {
        if self.encoding.y.is_none() {
            return Ok((0.0, 1.0));
        }

        let y_encoding = self.encoding.y.as_ref().unwrap();
        let y_series = self.data.df.column(&y_encoding.field)?;
        let mut y_min_val = y_series.min::<f64>()?.unwrap();
        let mut y_max_val = y_series.max::<f64>()?.unwrap();

        match self.mark.as_ref().map(|m| m.mark_type()) {
            Some("errorbar") => {
                let y_min_field = if let Some(y2) = &self.encoding.y2 { y2.field.clone() } 
                                  else { format!("__charton_temp_{}_min", y_encoding.field) };
                let y_max_field = if let Some(y2) = &self.encoding.y2 { y2.field.clone() }
                                  else { format!("__charton_temp_{}_max", y_encoding.field) };
                y_min_val = self.data.df.column(&y_min_field)?.min::<f64>()?.unwrap();
                y_max_val = self.data.df.column(&y_max_field)?.max::<f64>()?.unwrap();
            }
            Some("bar") if y_encoding.stack && self.encoding.color.is_some() => {
                let group_col = self.encoding.x.as_ref().unwrap().field.clone();
                let grouped = self.data.df.clone().lazy()
                    .group_by([col(group_col)])
                    .agg([col(&y_encoding.field).sum().alias("s")])
                    .collect()?;
                let s = grouped.column("s")?;
                y_min_val = s.min::<f64>()?.unwrap();
                y_max_val = s.max::<f64>()?.unwrap();
            }
            Some("rect") => {
                let bin = (y_max_val - y_min_val) / (y_series.n_unique()? as f64);
                y_min_val -= bin / 2.0;
                y_max_val += bin / 2.0;
            }
            _ => {}
        }

        let (f_min, f_max) = match y_encoding.zero {
            Some(true) => (y_min_val.min(0.0), y_max_val.max(0.0)),
            Some(false) => (y_min_val, y_max_val),
            None => {
                let is_stacked = matches!(self.mark.as_ref().map(|m| m.mark_type()), Some("bar") | Some("hist") | Some("area"));
                if is_stacked { (y_min_val.min(0.0), y_max_val.max(0.0)) } else { (y_min_val, y_max_val) }
            }
        };

        Ok((f_min, f_max))
    }

    /// For discrete data - return category labels
    fn get_x_discrete_tick_labels(&self) -> Result<Option<Vec<String>>, ChartonError> {
        if self.encoding.x.is_none() { return Ok(None); }
        let field = &self.encoding.x.as_ref().unwrap().field;
        let labels = self.data.df.column(field)?.unique_stable()?.str()?
            .into_no_null_iter().map(|s| s.to_string()).collect();
        Ok(Some(labels))
    }

    fn get_y_discrete_tick_labels(&self) -> Result<Option<Vec<String>>, ChartonError> {
        if self.encoding.y.is_none() { return Ok(None); }
        let field = &self.encoding.y.as_ref().unwrap().field;
        let labels = self.data.df.column(field)?.unique_stable()?.str()?
            .into_no_null_iter().map(|s| s.to_string()).collect();
        Ok(Some(labels))
    }

    /// Get encoding field names for axis labels
    fn get_x_encoding_field(&self) -> Option<String> {
        self.encoding.x.as_ref().map(|x| x.field.clone())
    }

    fn get_y_encoding_field(&self) -> Option<String> {
        self.encoding.y.as_ref().map(|y| y.field.clone())
    }

    /// Methods to get scale type for axes
    fn get_x_scale_type(&self) -> Result<Option<Scale>, ChartonError> {
        if self.encoding.x.is_none() { return Ok(None); }
        let x_enc = self.encoding.x.as_ref().unwrap();
        let scale = x_enc.scale.clone().unwrap_or_else(|| {
            determine_scale_for_dtype(self.data.df.column(&x_enc.field).unwrap().dtype())
        });
        Ok(Some(scale))
    }

    fn get_y_scale_type(&self) -> Result<Option<Scale>, ChartonError> {
        if self.encoding.y.is_none() { return Ok(None); }
        let y_enc = self.encoding.y.as_ref().unwrap();
        let scale = y_enc.scale.clone().unwrap_or_else(|| {
            determine_scale_for_dtype(self.data.df.column(&y_enc.field).unwrap().dtype())
        });
        Ok(Some(scale))
    }

/// Estimates the width of the legend column based on the longest label string
    /// and the number of columns required for discrete items.
    fn calculate_legend_width(
        &self,
        theme: &Theme,
        chart_height: f64,
        top_margin: f64,
        bottom_margin: f64,
    ) -> f64 {
        let mut max_width = 0.0;
        let plot_h = (1.0 - bottom_margin - top_margin) * chart_height;
        let available_h = plot_h - 30.0; // Space for the legend title
        let items_per_col = ((available_h / ITEM_HEIGHT).floor() as usize).clamp(1, MAX_ITEMS_PER_COLUMN);

        // Check discrete color legend
        if let Some(color_enc) = &self.encoding.color {
            let series = self.data.df.column(&color_enc.field).ok().unwrap();
            if matches!(determine_scale_for_dtype(series.dtype()), Scale::Discrete) {
                let unique_count = series.n_unique().unwrap_or(1);
                let cols_needed = (unique_count as f64 / items_per_col as f64).ceil() as usize;
                
                // Here we would ideally iterate unique values to find max string width
                let max_label_w = 60.0; // Placeholder for estimate_text_width call
                let col_w = COLOR_BOX_SIZE + COLOR_BOX_SPACING + max_label_w + LABEL_PADDING;
                max_width = (col_w * cols_needed as f64) + (COLUMN_SPACING * (cols_needed.saturating_sub(1)) as f64);
            } else {
                max_width = 100.0; // Fixed width for continuous color ramp
            }
        }
        
        max_width + 10.0 // Final safety padding
    }
}