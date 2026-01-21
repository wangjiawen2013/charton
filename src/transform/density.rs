use crate::chart::Chart;
use crate::core::data::*;
use crate::error::ChartonError;
use crate::mark::Mark;
use kernel_density_estimation::prelude::*;
use polars::prelude::*;

/// Kernel functions used in kernel density estimation
///
/// The kernel function determines the shape of the distribution used to estimate
/// the probability density at each point. Different kernels can produce different
/// smoothness characteristics in the resulting density curve.
///
/// Variants:
/// - `Normal`: Gaussian kernel, produces smooth curves
/// - `Epanechnikov`: Quartic kernel, optimal in mean square error sense
/// - `Uniform`: Rectangular kernel, equivalent to a moving average
#[derive(Debug, Clone)]
pub enum KernelType {
    Normal,
    Epanechnikov,
    Uniform,
}

impl KernelType {
    fn as_str(&self) -> &'static str {
        match self {
            KernelType::Normal => "Normal",
            KernelType::Epanechnikov => "Epanechnikov",
            KernelType::Uniform => "Uniform",
        }
    }
}

impl std::fmt::Display for KernelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Bandwidth selection methods for kernel density estimation
///
/// The bandwidth parameter controls the smoothness of the density estimation.
/// A larger bandwidth results in a smoother density curve, while a smaller
/// bandwidth results in a more detailed curve that may capture more features
/// but also more noise.
///
/// Variants:
/// - `Scott`: Uses Scott's rule of thumb for automatic bandwidth selection
/// - `Silverman`: Uses Silverman's rule of thumb for automatic bandwidth selection
/// - `Fixed(f64)`: Uses a fixed bandwidth value specified by the contained f64
#[derive(Debug, Clone)]
pub enum BandwidthType {
    Scott,
    Silverman,
    Fixed(f64),
}

impl BandwidthType {
    fn as_str(&self) -> &'static str {
        match self {
            BandwidthType::Scott => "Scott",
            BandwidthType::Silverman => "Silverman",
            BandwidthType::Fixed(_) => "Fixed",
        }
    }
}

impl std::fmt::Display for BandwidthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BandwidthType::Fixed(value) => write!(f, "Fixed({})", value),
            _ => write!(f, "{}", self.as_str()),
        }
    }
}

/// Configuration parameters for kernel density estimation transformation
///
/// This struct encapsulates all the settings needed to perform a kernel density
/// estimation on data, including the input field, output field names, bandwidth
/// selection method, kernel function, and various options for output formatting.
#[derive(Debug, Clone)]
pub struct DensityTransform {
    // The name of the input column containing the data to perform density estimation on
    pub(crate) density: String,
    // The names of the two output columns: [x_values_column_name, density_values_column_name]
    pub(crate) as_: [String; 2],
    // The bandwidth selection method for the kernel density estimation.
    pub(crate) bandwidth: BandwidthType,
    // A boolean flag indicating if the output values should be probability estimates (false) or smoothed counts (true)
    pub(crate) counts: bool,
    // A boolean flag indicating whether to produce density estimates (false) or cumulative density estimates (true)
    pub(crate) cumulative: bool,
    // The data fields to group by
    pub(crate) groupby: Option<String>,
    // The kernel function to use for density estimation
    pub(crate) kernel: KernelType,
}

impl DensityTransform {
    /// Creates a new `DensityTransform` instance with default parameters
    ///
    /// # Parameters
    /// * `density_field` - The name of the column containing the data to perform density estimation on
    ///
    /// # Returns
    /// A new `DensityTransform` instance with the following defaults:
    /// - Output field names: ["value", "density"]
    /// - Bandwidth selection: Scott's rule
    /// - Counts: false (outputs probability densities)
    /// - Cumulative: false (outputs density estimates)
    /// - No grouping
    /// - Kernel function: Normal (Gaussian)
    pub fn new(density_field: impl Into<String>) -> Self {
        Self {
            density: density_field.into(),
            as_: ["value".to_string(), "density".to_string()],
            bandwidth: BandwidthType::Scott, // Default to use Scott's rule
            counts: false,
            cumulative: false,
            groupby: None,
            kernel: KernelType::Normal,
        }
    }

    /// Sets the output column names for the density transformation
    ///
    /// # Parameters
    /// * `value_field` - The name for the column that will contain the x-axis values (evaluation points)
    /// * `density_field` - The name for the column that will contain the computed density values
    ///
    /// # Returns
    /// The modified `DensityTransform` instance with updated output column names
    ///
    /// # Example
    /// ```
    /// let transform = DensityTransform::new("data")
    ///     .with_as("x_values", "y_density");
    /// ```
    pub fn with_as(
        mut self,
        value_field: impl Into<String>,
        density_field: impl Into<String>,
    ) -> Self {
        self.as_ = [value_field.into(), density_field.into()];
        self
    }

    /// Sets the bandwidth selection method for the kernel density estimation
    ///
    /// # Parameters
    /// * `bandwidth` - The bandwidth selection method to use, which controls the smoothness of the density curve
    ///
    /// # Returns
    /// The modified `DensityTransform` instance with the updated bandwidth setting
    ///
    /// # Example
    /// ```
    /// let transform = DensityTransform::new("data")
    ///     .with_bandwidth(BandwidthType::Silverman);
    /// ```
    pub fn with_bandwidth(mut self, bandwidth: BandwidthType) -> Self {
        self.bandwidth = bandwidth;
        self
    }

    /// Sets whether the output values should be probability estimates or smoothed counts
    ///
    /// # Parameters
    /// * `counts` - If true, outputs smoothed counts; if false, outputs probability density estimates
    ///
    /// # Returns
    /// The modified `DensityTransform` instance with the updated counts setting
    ///
    /// # Example
    /// ```
    /// let transform = DensityTransform::new("data")
    ///     .with_counts(true); // Output smoothed counts instead of probabilities
    /// ```
    pub fn with_counts(mut self, counts: bool) -> Self {
        self.counts = counts;
        self
    }

    /// Sets whether to produce density estimates or cumulative density estimates
    ///
    /// # Parameters
    /// * `cumulative` - If true, produces cumulative density estimates; if false, produces regular density estimates
    ///
    /// # Returns
    /// The modified `DensityTransform` instance with the updated cumulative setting
    ///
    /// # Example
    /// ```
    /// let transform = DensityTransform::new("data")
    ///     .with_cumulative(true); // Output cumulative density instead of regular density
    /// ```
    pub fn with_cumulative(mut self, cumulative: bool) -> Self {
        self.cumulative = cumulative;
        self
    }

    /// Sets the field to group by for separate density estimations
    ///
    /// # Parameters
    /// * `groupby` - The name of the column to group by, with separate density curves computed for each group
    ///
    /// # Returns
    /// The modified `DensityTransform` instance with the updated groupby setting
    ///
    /// # Example
    /// ```
    /// let transform = DensityTransform::new("data")
    ///     .with_groupby("category"); // Compute separate density curves for each category
    /// ```
    pub fn with_groupby(mut self, groupby: &str) -> Self {
        self.groupby = Some(groupby.into());
        self
    }

    /// Sets the kernel function to use for density estimation
    ///
    /// # Parameters
    /// * `kernel` - The kernel function to use, which determines the shape of the distribution used for estimating density
    ///
    /// # Returns
    /// The modified `DensityTransform` instance with the updated kernel setting
    ///
    /// # Example
    /// ```
    /// let transform = DensityTransform::new("data")
    ///     .with_kernel(KernelType::Epanechnikov); // Use Epanechnikov kernel instead of default Normal
    /// ```
    pub fn with_kernel(mut self, kernel: impl Into<KernelType>) -> Self {
        self.kernel = kernel.into();
        self
    }
}

impl<T: Mark> Chart<T> {
    /// Transform data by performing kernel density estimation
    ///
    /// This method computes kernel density estimates for the specified data field,
    /// optionally grouped by another field. The resulting density estimates can
    /// be configured to be cumulative, counts-based, or regular probability densities.
    ///
    /// # Parameters
    /// * `params` - A `DensityTransform` configuration object specifying the details of the density estimation
    ///
    /// # Returns
    /// * `Result<Self, ChartonError>` - The chart with transformed density data or an error if the transformation fails
    ///
    /// # Example
    /// ```
    /// let density_params = DensityTransform::new("values")
    ///     .with_bandwidth(BandwidthType::Silverman)
    ///     .with_kernel(KernelType::Epanechnikov)
    ///     .with_groupby("category".to_string());
    ///
    /// chart.transform_density(density_params)?;
    /// ```
    pub fn transform_density(mut self, params: DensityTransform) -> Result<Self, ChartonError> {
        // Get all density values to compute global min/max
        let density_field = &params.density;
        let density_series = self.data.column(density_field)?;

        // Determine global min and max values for evaluation
        let min_val = density_series.min::<f64>()?.unwrap();
        let max_val = density_series.max::<f64>()?.unwrap();

        // Extend the range by 30% on both sides to better visualize the density tails
        let extended_min = 1.3 * min_val - 0.3 * max_val;
        let extended_max = 1.3 * max_val - 0.3 * min_val;

        // Handle edge case where all values are the same
        let (min_val, max_val) = if (extended_max - extended_min).abs() < 1e-12 {
            let offset = if extended_min == 0.0 {
                1.0
            } else {
                extended_min.abs() * 0.1
            };
            (extended_min - offset, extended_max + offset)
        } else {
            (extended_min, extended_max)
        };

        // Create evaluation points (default 200 steps)
        let steps = 200;
        let step_size = (max_val - min_val) / (steps as f64);
        let eval_points: Vec<f32> = (0..steps)
            .map(|i| (min_val + (i as f64) * step_size) as f32)
            .collect();

        // Create value column (x-axis)
        let value_column: Vec<f64> = eval_points.iter().map(|&v| v as f64).collect();

        // Determine the group field name once to avoid duplication
        let group_field_name = params
            .groupby
            .clone()
            .unwrap_or_else(|| format!("__charton_temp_group_{}", crate::TEMP_SUFFIX));

        // Create a working DataFrame with grouping column
        let working_df = if let Some(ref group_field) = params.groupby {
            // Use existing group field
            self.data.df.select([density_field, group_field])?
        } else {
            // Create a fake grouping column with a single group
            let fake_group_series = Series::new(
                (&group_field_name).into(),
                vec!["fake"; self.data.df.height()],
            );
            self.data
                .df
                .select([density_field])?
                .with_column(fake_group_series)?
                .clone() // Ensure we're working with an owned DataFrame
        };

        // Group by the group field and collect density values for each group
        let grouped_df = working_df
            .lazy()
            .group_by_stable([col(&group_field_name)])
            .agg([col(density_field).implode().alias(density_field)])
            .collect()?;

        let mut all_groups = Vec::new();
        let mut all_x_values = Vec::new();
        let mut all_y_values = Vec::new();

        // Process each group
        for i in 0..grouped_df.height() {
            // Get group name
            let group_value = match grouped_df
                .column(&group_field_name)
                .map_err(ChartonError::Polars)?
                .get(i)?
            {
                AnyValue::String(s) => s.to_string(),
                AnyValue::Int32(v) => v.to_string(),
                AnyValue::Int64(v) => v.to_string(),
                AnyValue::Float64(v) => v.to_string(),
                _ => "unknown".to_string(),
            };

            // Get density values for this group
            let list_series = grouped_df.column(density_field)?.get(i)?;

            let group_vals: Vec<f64> = match list_series {
                AnyValue::List(inner) => inner.f64()?.into_no_null_iter().collect(),
                _ => continue,
            };

            // Convert to f32 for kernel density estimation crate
            let observations: Vec<f32> = group_vals.iter().map(|&v| v as f32).collect();

            // Create KDE based on specific combinations of bandwidth and kernel and calculate density values
            let density_values: Vec<f64> = match (&params.bandwidth, &params.kernel) {
                (BandwidthType::Scott, KernelType::Normal) => {
                    let kde = KernelDensityEstimator::new(observations.clone(), Scott, Normal);

                    // Calculate density values
                    if params.cumulative {
                        // Calculate cumulative density
                        kde.cdf(&eval_points).iter().map(|&v| v as f64).collect()
                    } else {
                        // Calculate probability density
                        kde.pdf(&eval_points).iter().map(|&v| v as f64).collect()
                    }
                }

                (BandwidthType::Scott, KernelType::Epanechnikov) => {
                    let kde =
                        KernelDensityEstimator::new(observations.clone(), Scott, Epanechnikov);

                    if params.cumulative {
                        kde.cdf(&eval_points).iter().map(|&v| v as f64).collect()
                    } else {
                        kde.pdf(&eval_points).iter().map(|&v| v as f64).collect()
                    }
                }
                (BandwidthType::Scott, KernelType::Uniform) => {
                    let kde = KernelDensityEstimator::new(observations.clone(), Scott, Uniform);

                    if params.cumulative {
                        kde.cdf(&eval_points).iter().map(|&v| v as f64).collect()
                    } else {
                        kde.pdf(&eval_points).iter().map(|&v| v as f64).collect()
                    }
                }
                (BandwidthType::Silverman, KernelType::Normal) => {
                    let kde = KernelDensityEstimator::new(observations.clone(), Silverman, Normal);

                    if params.cumulative {
                        kde.cdf(&eval_points).iter().map(|&v| v as f64).collect()
                    } else {
                        kde.pdf(&eval_points).iter().map(|&v| v as f64).collect()
                    }
                }
                (BandwidthType::Silverman, KernelType::Epanechnikov) => {
                    let kde =
                        KernelDensityEstimator::new(observations.clone(), Silverman, Epanechnikov);

                    if params.cumulative {
                        kde.cdf(&eval_points).iter().map(|&v| v as f64).collect()
                    } else {
                        kde.pdf(&eval_points).iter().map(|&v| v as f64).collect()
                    }
                }
                (BandwidthType::Silverman, KernelType::Uniform) => {
                    let kde = KernelDensityEstimator::new(observations.clone(), Silverman, Uniform);

                    if params.cumulative {
                        kde.cdf(&eval_points).iter().map(|&v| v as f64).collect()
                    } else {
                        kde.pdf(&eval_points).iter().map(|&v| v as f64).collect()
                    }
                }
                (BandwidthType::Fixed(value), KernelType::Normal) => {
                    let bandwidth = Box::new(|_: &[f32]| *value as f32);
                    let kde = KernelDensityEstimator::new(observations.clone(), bandwidth, Normal);

                    if params.cumulative {
                        kde.cdf(&eval_points).iter().map(|&v| v as f64).collect()
                    } else {
                        kde.pdf(&eval_points).iter().map(|&v| v as f64).collect()
                    }
                }
                (BandwidthType::Fixed(value), KernelType::Epanechnikov) => {
                    let bandwidth = Box::new(|_: &[f32]| *value as f32);
                    let kde =
                        KernelDensityEstimator::new(observations.clone(), bandwidth, Epanechnikov);

                    if params.cumulative {
                        kde.cdf(&eval_points).iter().map(|&v| v as f64).collect()
                    } else {
                        kde.pdf(&eval_points).iter().map(|&v| v as f64).collect()
                    }
                }
                (BandwidthType::Fixed(value), KernelType::Uniform) => {
                    let bandwidth = Box::new(|_: &[f32]| *value as f32);
                    let kde = KernelDensityEstimator::new(observations.clone(), bandwidth, Uniform);

                    if params.cumulative {
                        kde.cdf(&eval_points).iter().map(|&v| v as f64).collect()
                    } else {
                        kde.pdf(&eval_points).iter().map(|&v| v as f64).collect()
                    }
                }
            };

            let density_values = if params.counts {
                // Scale by number of observations to get counts
                density_values
                    .into_iter()
                    .map(|v| v * group_vals.len() as f64)
                    .collect()
            } else {
                // Probability density
                density_values
            };

            // Add results directly to the combined vectors
            for _ in 0..value_column.len() {
                all_groups.push(group_value.clone());
            }
            all_x_values.extend(value_column.clone());
            all_y_values.extend(density_values);
        }

        // Create the result DataFrame
        let result_df = if params.groupby.is_some() {
            // Include group column if we have grouping
            df![
                &params.as_[0] => all_x_values,
                &params.as_[1] => all_y_values,
                &group_field_name => all_groups
            ]
        } else {
            // Just the value and density columns
            df![
                &params.as_[0] => all_x_values,
                &params.as_[1] => all_y_values
            ]
        };

        self.data = DataFrameSource::new(result_df?);

        Ok(self)
    }
}
