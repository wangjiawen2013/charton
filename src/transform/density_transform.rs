use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::stats::kde::Kde;
use ahash::AHashMap;

// The density options and the estimator live in the statistics module so that
// the density transform and the point layouts share one implementation. They
// are imported here to sit next to the transform that uses them.
pub use crate::stats::kde::{BandwidthType, KernelType};

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
    pub const fn with_bandwidth(mut self, bandwidth: BandwidthType) -> Self {
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
    pub const fn with_counts(mut self, counts: bool) -> Self {
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
    pub const fn with_cumulative(mut self, cumulative: bool) -> Self {
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
    /// Uses ColumnVector::unique_values() to ensure deterministic group ordering
    /// and consistent null-filtering behavior.
    pub fn transform_density(mut self, params: DensityTransform) -> Result<Self, ChartonError> {
        let density_field = &params.density;
        let density_col = self.data.column(density_field)?;

        // --- STEP 1: Calculate Global Range (Ignoring Nulls) ---
        let (min_val, max_val) = density_col.min_max();

        // Extend range by 30% to capture distribution tails (standard visualization practice).
        let mut extended_min = 1.3 * min_val - 0.3 * max_val;
        let mut extended_max = 1.3 * max_val - 0.3 * min_val;

        // Handle edge case where all values are identical or constant.
        if (extended_max - extended_min).abs() < 1e-12 {
            let offset = if extended_min == 0.0 {
                1.0
            } else {
                extended_min.abs() * 0.1
            };
            extended_min -= offset;
            extended_max += offset;
        }

        // Generate 200 evaluation points for a smooth curve.
        let steps = 200;
        let step_size = (extended_max - extended_min) / (steps as f64);
        let eval_points: Vec<f64> = (0..steps)
            .map(|i| extended_min + (i as f64) * step_size)
            .collect();

        let x_axis_values = eval_points.clone();

        // --- STEP 2: Establish Deterministic Order ---
        // We use unique_values() to ensure the order of density curves matches
        // the legend and other transforms (First Appearance).
        let group_order: Vec<Option<String>> = if let Some(ref g_field) = params.groupby {
            self.data
                .column(g_field)?
                .unique_values()
                .into_iter()
                .map(Some)
                .collect()
        } else {
            // Use None as a placeholder for the global (no-groupby) case.
            vec![None]
        };

        // --- STEP 3: Aggregate Observations by Group ---
        let mut groups: AHashMap<Option<String>, Vec<f64>> = AHashMap::new();
        let row_count = self.data.height();

        if let Some(ref g_field) = params.groupby {
            let group_col = self.data.column(g_field)?;
            for i in 0..row_count {
                if let Some(val) = density_col.get(i).to_f64() {
                    let key = group_col.get(i).to_string();
                    groups.entry(key).or_default().push(val);
                }
            }
        } else {
            // Optimized path for global density calculation.
            let mut all_obs = Vec::with_capacity(row_count);
            for i in 0..row_count {
                if let Some(val) = density_col.get(i).to_f64() {
                    all_obs.push(val);
                }
            }
            groups.insert(None, all_obs);
        }

        // --- STEP 4: Compute KDE per Group ---
        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        let mut final_group = Vec::new();

        for key in group_order {
            let observations = match groups.get(&key) {
                Some(obs) if !obs.is_empty() => obs,
                _ => continue,
            };

            let group_label = key.as_deref().unwrap_or("all").to_string();

            // Build one estimate for this group and read it at the fixed
            // evaluation points that make up the density curve.
            let kde = Kde::new(observations.clone(), params.bandwidth, params.kernel);
            let density_values: Vec<f64> = if params.cumulative {
                kde.cdf(&eval_points)
            } else {
                kde.pdf(&eval_points)
            };

            let obs_count = observations.len() as f64;
            let processed_y = if params.counts {
                density_values.into_iter().map(|v| v * obs_count).collect()
            } else {
                density_values
            };

            final_y.extend(processed_y);
            final_x.extend(x_axis_values.clone());

            if params.groupby.is_some() {
                for _ in 0..steps {
                    final_group.push(group_label.clone());
                }
            }
        }

        // --- STEP 5: Build Final Dataset ---
        let mut new_ds = Dataset::new();

        let x_prototype = density_col.type_prototype(); // density_col is X axis
        let restored_x = match x_prototype {
            ColumnVector::Datetime { timezone, .. } => ColumnVector::Datetime {
                data: final_x.into_iter().map(|v| v.round() as i64).collect(),
                validity: None,
                timezone,
            },
            ColumnVector::Date { .. } => ColumnVector::Date {
                data: final_x.into_iter().map(|v| v.round() as i32).collect(),
                validity: None,
            },
            ColumnVector::Duration { .. } => ColumnVector::Duration {
                data: final_x.into_iter().map(|v| v.round() as i64).collect(),
                validity: None,
            },
            ColumnVector::Time { .. } => ColumnVector::Time {
                data: final_x.into_iter().map(|v| v.round() as i64).collect(),
                validity: None,
            },
            _ => ColumnVector::Float64 {
                data: final_x,
                validity: None,
            },
        };

        new_ds.add_column(&params.as_[0], restored_x)?;

        // Use Float64 variant with validity: None (KDE output points are always valid)
        new_ds.add_column(
            &params.as_[1],
            ColumnVector::Float64 {
                data: final_y,
                validity: None,
            },
        )?;

        if let Some(ref g_field) = params.groupby {
            new_ds.add_column(
                g_field,
                ColumnVector::String {
                    data: final_group,
                    validity: None,
                },
            )?;
        }

        // Replace chart data with the newly generated density dataset.
        self.data = new_ds;
        Ok(self)
    }
}
