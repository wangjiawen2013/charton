use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::AHashMap;
use kernel_density_estimation::prelude::*;

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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
    /// let transform = DensityTransform::new("data")
    ///     .with_kernel(KernelType::Epanechnikov); // Use Epanechnikov kernel instead of default Normal
    /// ```
    pub fn with_kernel(mut self, kernel: impl Into<KernelType>) -> Self {
        self.kernel = kernel.into();
        self
    }
}

impl<T: Mark> Chart<T> {
    /// Transform data by performing kernel density estimation (KDE).
    /// Uses ColumnVector::unique_values() to ensure deterministic group ordering.
    pub fn transform_density(mut self, params: DensityTransform) -> Result<Self, ChartonError> {
        let density_field = &params.density;
        let density_col = self.data.column(density_field)?;

        // --- STEP 1: Calculate Global Range ---
        let (min_val, max_val) = density_col.min_max();

        // Extend range by 30% to capture distribution tails.
        let mut extended_min = 1.3 * min_val - 0.3 * max_val;
        let mut extended_max = 1.3 * max_val - 0.3 * min_val;

        // Handle identical values case.
        if (extended_max - extended_min).abs() < 1e-12 {
            let offset = if extended_min == 0.0 {
                1.0
            } else {
                extended_min.abs() * 0.1
            };
            extended_min -= offset;
            extended_max += offset;
        }

        // 200 evaluation points for a smooth curve.
        let steps = 200;
        let step_size = (extended_max - extended_min) / (steps as f64);
        let eval_points: Vec<f32> = (0..steps)
            .map(|i| (extended_min + (i as f64) * step_size) as f32)
            .collect();

        let x_axis_values: Vec<f64> = eval_points.iter().map(|&v| v as f64).collect();

        // --- STEP 2: Establish Deterministic Order ---
        // unique_values() returns Vec<String> directly, preserving appearance order.
        let group_order: Vec<String> = if let Some(ref g_field) = params.groupby {
            self.data.column(g_field)?.unique_values()
        } else {
            vec!["__default__".to_string()]
        };

        // --- STEP 3: Aggregate Observations by Group ---
        let mut groups: AHashMap<String, Vec<f32>> = AHashMap::new();
        let row_count = self.data.height();

        for i in 0..row_count {
            let group_key = if let Some(ref g_field) = params.groupby {
                self.data.get_str_or(g_field, i, "null")
            } else {
                "__default__".to_string()
            };

            if let Some(val) = density_col.get_f64(i) {
                groups.entry(group_key).or_default().push(val as f32);
            }
        }

        // --- STEP 4: Compute KDE per Group (Iterate using stable order) ---
        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        let mut final_group = Vec::new();

        for group_name in group_order {
            // Get observations; skip if group is missing or empty.
            let observations = match groups.get(&group_name) {
                Some(obs) if !obs.is_empty() => obs,
                _ => continue,
            };

            // Trait dispatch for KDE calculation.
            let density_values: Vec<f64> = match (&params.bandwidth, &params.kernel) {
                (BandwidthType::Scott, KernelType::Normal) => {
                    let kde = KernelDensityEstimator::new(observations.clone(), Scott, Normal);
                    if params.cumulative {
                        kde.cdf(&eval_points)
                    } else {
                        kde.pdf(&eval_points)
                    }
                }
                (BandwidthType::Scott, KernelType::Epanechnikov) => {
                    let kde =
                        KernelDensityEstimator::new(observations.clone(), Scott, Epanechnikov);
                    if params.cumulative {
                        kde.cdf(&eval_points)
                    } else {
                        kde.pdf(&eval_points)
                    }
                }
                (BandwidthType::Scott, KernelType::Uniform) => {
                    let kde = KernelDensityEstimator::new(observations.clone(), Scott, Uniform);
                    if params.cumulative {
                        kde.cdf(&eval_points)
                    } else {
                        kde.pdf(&eval_points)
                    }
                }
                (BandwidthType::Silverman, KernelType::Normal) => {
                    let kde = KernelDensityEstimator::new(observations.clone(), Silverman, Normal);
                    if params.cumulative {
                        kde.cdf(&eval_points)
                    } else {
                        kde.pdf(&eval_points)
                    }
                }
                (BandwidthType::Silverman, KernelType::Epanechnikov) => {
                    let kde =
                        KernelDensityEstimator::new(observations.clone(), Silverman, Epanechnikov);
                    if params.cumulative {
                        kde.cdf(&eval_points)
                    } else {
                        kde.pdf(&eval_points)
                    }
                }
                (BandwidthType::Silverman, KernelType::Uniform) => {
                    let kde = KernelDensityEstimator::new(observations.clone(), Silverman, Uniform);
                    if params.cumulative {
                        kde.cdf(&eval_points)
                    } else {
                        kde.pdf(&eval_points)
                    }
                }
                (BandwidthType::Fixed(bw), KernelType::Normal) => {
                    let h = *bw as f32;
                    let kde = KernelDensityEstimator::new(
                        observations.clone(),
                        move |_: &[f32]| h,
                        Normal,
                    );
                    if params.cumulative {
                        kde.cdf(&eval_points)
                    } else {
                        kde.pdf(&eval_points)
                    }
                }
                (BandwidthType::Fixed(bw), KernelType::Epanechnikov) => {
                    let h = *bw as f32;
                    let kde = KernelDensityEstimator::new(
                        observations.clone(),
                        move |_: &[f32]| h,
                        Epanechnikov,
                    );
                    if params.cumulative {
                        kde.cdf(&eval_points)
                    } else {
                        kde.pdf(&eval_points)
                    }
                }
                (BandwidthType::Fixed(bw), KernelType::Uniform) => {
                    let h = *bw as f32;
                    let kde = KernelDensityEstimator::new(
                        observations.clone(),
                        move |_: &[f32]| h,
                        Uniform,
                    );
                    if params.cumulative {
                        kde.cdf(&eval_points)
                    } else {
                        kde.pdf(&eval_points)
                    }
                }
            }
            .into_iter()
            .map(|v| v as f64)
            .collect();

            let obs_count = observations.len() as f64;
            let processed_y = if params.counts {
                density_values.into_iter().map(|v| v * obs_count).collect()
            } else {
                density_values
            };

            final_y.extend(processed_y);
            final_x.extend(x_axis_values.clone());

            // If grouping is active, repeat the group name for all 200 points.
            if params.groupby.is_some() {
                for _ in 0..steps {
                    final_group.push(group_name.clone());
                }
            }
        }

        // --- STEP 5: Build Final Dataset ---
        let mut new_ds = Dataset::new();
        new_ds.add_column(&params.as_[0], ColumnVector::F64 { data: final_x })?;
        new_ds.add_column(&params.as_[1], ColumnVector::F64 { data: final_y })?;

        if let Some(ref g_field) = params.groupby {
            new_ds.add_column(
                g_field,
                ColumnVector::String {
                    data: final_group,
                    validity: None,
                },
            )?;
        }

        self.data = new_ds;
        Ok(self)
    }
}
