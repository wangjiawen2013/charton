use crate::coord::Scale;
use crate::coord::cartesian::Cartesian;
use crate::data::determine_scale_for_dtype;
use crate::error::ChartonError;
use crate::render::utils::normalize_linear;
use std::collections::HashMap;

pub(crate) struct ProcessedChartData {
    pub(crate) x_vals: Vec<f64>,
    pub(crate) _y_vals: Vec<f64>,
    pub(crate) x_transformed_vals: Vec<f64>,
    pub(crate) y_transformed_vals: Vec<f64>,
    pub(crate) shape_vals: Option<Vec<String>>,
    pub(crate) normalized_sizes: Option<Vec<f64>>,
    pub(crate) color_info: Option<(Scale, Vec<f64>)>,
}

impl ProcessedChartData {
    /// Constructs a new `ProcessedChartData` instance for handling chart data preprocessing
    /// based on coordinate system specifications.
    ///
    /// This function extracts and transforms data columns from the given chart configuration
    /// and coordinate system, including:
    /// - Extracting x and y axis data, mapping discrete values to indices when needed
    /// - Processing visual channel data for shape, size, color
    /// - Normalizing certain numeric visual channels to appropriate ranges
    /// - Automatically determining color scale type based on data type
    ///
    /// # Arguments
    /// - `chart`: Chart configuration containing data source and encoding information
    /// - `coord_system`: Reference to the coordinate system for determining axis scale types
    ///
    /// # Returns
    /// A `Result<Self>` where `Self` is the `ProcessedChartData` struct instance.
    /// Returns an error if any issues occur during data extraction or processing.
    ///
    /// # Errors
    /// May return errors related to:
    /// - Missing data columns
    /// - Data type conversion failures
    /// - Polars data processing errors (e.g., unique value extraction)
    pub(crate) fn new<T: crate::mark::Mark>(
        chart: &crate::chart::common::Chart<T>,
        coord_system: &Cartesian, // Accept shared coordinate system
    ) -> Result<Self, ChartonError> {
        // Get data columns
        let x_series = chart
            .data
            .column(chart.encoding.x.as_ref().unwrap().field.as_str())?;
        let y_series = chart
            .data
            .column(chart.encoding.y.as_ref().unwrap().field.as_str())?;

        // Handle discrete x-axis data
        let x_vals: Vec<f64> = if matches!(coord_system.x_axis.scale, Scale::Discrete) {
            // For discrete data, we need to map string values to their indices
            // Get unique values while preserving order
            let unique_values_series = x_series.unique_stable()?;
            let unique_values = unique_values_series
                .str()?
                .into_no_null_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            // Create mapping from values to indices
            let mut value_to_index = HashMap::new();
            for (index, value) in unique_values.iter().enumerate() {
                value_to_index.insert(value.clone(), index as f64);
            }

            // Map original values to indices
            x_series
                .str()?
                .into_no_null_iter()
                .map(|val| {
                    let val_string = val.to_string();
                    *value_to_index.get(&val_string).unwrap_or(&0.0)
                })
                .collect()
        } else {
            // Continuous data
            x_series.f64()?.into_no_null_iter().collect::<Vec<_>>()
        };

        // Handle discrete y-axis data
        let _y_vals: Vec<f64> = if matches!(coord_system.y_axis.scale, Scale::Discrete) {
            // For discrete data, we need to map string values to their indices
            // Get unique values while preserving order
            let unique_values_series = y_series.unique_stable()?;
            let unique_values = unique_values_series
                .str()?
                .into_no_null_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            // Create mapping from values to indices
            let mut value_to_index = HashMap::new();
            for (index, value) in unique_values.iter().enumerate() {
                value_to_index.insert(value.clone(), index as f64);
            }

            // Map original values to indices
            y_series
                .str()?
                .into_no_null_iter()
                .map(|val| {
                    let val_string = val.to_string();
                    *value_to_index.get(&val_string).unwrap_or(&0.0)
                })
                .collect()
        } else {
            // Continuous data
            y_series.f64()?.into_no_null_iter().collect::<Vec<_>>()
        };

        // Transform data values according to scale type
        let x_transformed_vals: Vec<f64> = match coord_system.x_axis.scale {
            Scale::Log => x_vals.iter().map(|&x| x.log10()).collect(),
            _ => x_vals.clone(),
        };

        let y_transformed_vals: Vec<f64> = match coord_system.y_axis.scale {
            Scale::Log => _y_vals.iter().map(|&y| y.log10()).collect(),
            _ => _y_vals.clone(),
        };

        // Get data for shape channel if it exists
        let shape_vals = if let Some(shape_enc) = &chart.encoding.shape {
            let shape_col = chart.data.column(&shape_enc.field)?;
            let str_series = shape_col.str()?;
            Some(
                str_series
                    .into_no_null_iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };

        // Get data for size channel if it exists
        let size_vals = if let Some(size_enc) = &chart.encoding.size {
            let size_col = chart.data.column(&size_enc.field)?;
            Some(size_col.f64()?.into_no_null_iter().collect::<Vec<_>>())
        } else {
            None
        };

        // Normalize size values (Altair default size range is approximately 10-1000 square pixels)
        let normalized_sizes = size_vals.as_ref().map(|sizes| {
            normalize_linear(sizes, 2.0, 10.0) // Map sizes to 2-10 pixel range
        });

        // Get color data and automatically determine scale type
        let color_info: Option<(Scale, Vec<f64>)> = if let Some(color_enc) = &chart.encoding.color {
            let color_series = chart.data.column(&color_enc.field)?;
            // Auto-detect the data type according to the scale type
            let scale_type = determine_scale_for_dtype(color_series.dtype());

            match scale_type {
                // Continuous scales - normalize data to 0-1 range
                Scale::Linear | Scale::Log => {
                    // Normalize to 0-1 range for colormap usage
                    let min_val = color_series.min::<f64>()?.unwrap();
                    let max_val = color_series.max::<f64>()?.unwrap();

                    let normalized = if max_val - min_val > 1e-10 {
                        color_series
                            .f64()?
                            .into_no_null_iter()
                            .map(|v| (v - min_val) / (max_val - min_val))
                            .collect()
                    } else {
                        // Create a vector of 0.5 with the same length as the color series
                        vec![0.5; color_series.len()]
                    };

                    Some((scale_type, normalized))
                }
                // Discrete scale - map unique values to indices
                Scale::Discrete => {
                    // Get unique values while preserving order
                    let unique_values_series = color_series.unique_stable()?;
                    let unique_values = unique_values_series
                        .str()?
                        .into_no_null_iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>();

                    // Map original values to indices
                    let indices: Vec<f64> = color_series
                        .str()?
                        .into_no_null_iter()
                        .map(|val| {
                            unique_values
                                .iter()
                                .position(|v| v == val)
                                .map(|i| i as f64)
                                .unwrap_or(0.0)
                        })
                        .collect();

                    Some((scale_type, indices))
                }
            }
        } else {
            None
        };

        Ok(ProcessedChartData {
            x_vals,
            _y_vals,
            x_transformed_vals,
            y_transformed_vals,
            shape_vals,
            normalized_sizes,
            color_info,
        })
    }
}
