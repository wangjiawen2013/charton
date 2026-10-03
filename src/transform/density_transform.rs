//! One-dimensional kernel density estimation.
//!
//! `transform_density` turns a numeric column into a smooth density curve: a
//! new table with a row for every evaluation point and the estimated density
//! there. It is the statistical half of a density plot, a violin or a ridgeline.
//!
//! The transform only writes numbers; it never draws. Turning the curve into a
//! shape is the geometry's job: `mark_area` with a mirror stack for a smooth
//! density plot, or `transform_band` for a violin.

use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::stats::kde::Kde;
use ahash::{AHashMap, AHashSet};

// The density options and the estimator live in the statistics module so that
// the density transform and the point layouts share one implementation. They
// are imported here to sit next to the transform that uses them.
pub use crate::stats::kde::{BandwidthType, KernelType};

/// Settings for [`Chart::transform_density`].
///
/// The defaults describe a single density curve of the whole column. Chain
/// [`with_groupbys`](Self::with_groupbys) for one curve per group, and
/// [`with_as`](Self::with_as) to rename the two output columns.
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
    // The data fields to group by. Empty means "one global density curve".
    pub(crate) groupby: Vec<String>,
    // The kernel function to use for density estimation
    pub(crate) kernel: KernelType,
}

impl DensityTransform {
    /// Estimates the density of the `density_field` column.
    ///
    /// Defaults: output columns `["value", "density"]`, Scott's rule for the
    /// bandwidth, a Gaussian kernel, probability densities (not counts) and no
    /// grouping.
    pub fn new(density_field: impl Into<String>) -> Self {
        Self {
            density: density_field.into(),
            as_: ["value".to_string(), "density".to_string()],
            bandwidth: BandwidthType::Scott, // Default to use Scott's rule
            counts: false,
            cumulative: false,
            groupby: Vec::new(),
            kernel: KernelType::Normal,
        }
    }

    /// Renames the two output columns: the evaluation points and the density.
    ///
    /// Defaults to `["value", "density"]`.
    pub fn with_as(
        mut self,
        value_field: impl Into<String>,
        density_field: impl Into<String>,
    ) -> Self {
        self.as_ = [value_field.into(), density_field.into()];
        self
    }

    /// Chooses how the smoothing width is picked — Scott's rule, Silverman's
    /// rule, or a fixed value. A wider bandwidth gives a smoother curve.
    pub const fn with_bandwidth(mut self, bandwidth: BandwidthType) -> Self {
        self.bandwidth = bandwidth;
        self
    }

    /// Emits smoothed *counts* instead of a probability density, by scaling
    /// each curve by the number of observations in its group.
    pub const fn with_counts(mut self, counts: bool) -> Self {
        self.counts = counts;
        self
    }

    /// Emits the cumulative density (an ECDF-like rising curve) instead of the
    /// density itself.
    pub const fn with_cumulative(mut self, cumulative: bool) -> Self {
        self.cumulative = cumulative;
        self
    }

    /// Groups the estimate by one or more fields, for example `["species"]` or
    /// `["Sex", "Species"]`.
    ///
    /// Each distinct combination of the fields becomes its own density curve.
    /// Grouping by a single field is the ordinary "one curve per category"
    /// case; grouping by two fields at once is what a dodged or split violin
    /// needs, where the first is the position on the discrete axis and the
    /// second is the lane inside it.
    pub fn with_groupbys<I, S>(mut self, groupbys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.groupby = groupbys.into_iter().map(Into::into).collect();
        self
    }

    /// Chooses the smoothing kernel (the bump shape). Gaussian by default.
    pub fn with_kernel(mut self, kernel: impl Into<KernelType>) -> Self {
        self.kernel = kernel.into();
        self
    }
}

impl<T: Mark> Chart<T> {
    /// Estimates a density curve from a numeric column.
    ///
    /// The dataset is replaced by a table of evaluation points and densities.
    /// Group it with [`DensityTransform::with_groupbys`] to get one curve per
    /// group, then draw it with `mark_area` (a mirror stack) or
    /// `transform_band`.
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
        // The keys are the tuples of the groupby fields, in first-appearance
        // order, so curves line up with the legend. With no groupby there is a
        // single, empty key.
        let group_columns: Vec<&ColumnVector> = params
            .groupby
            .iter()
            .map(|field| self.data.column(field))
            .collect::<Result<Vec<_>, _>>()?;

        let group_order: Vec<Vec<String>> = if params.groupby.is_empty() {
            vec![Vec::new()]
        } else {
            let mut seen = AHashSet::new();
            let mut order = Vec::new();
            for i in 0..self.data.height() {
                let key: Vec<String> = group_columns
                    .iter()
                    .map(|col| col.get(i).to_string().unwrap_or_else(|| "null".to_string()))
                    .collect();
                if seen.insert(key.clone()) {
                    order.push(key);
                }
            }
            order
        };

        // --- STEP 3: Aggregate Observations by Group ---
        let mut groups: AHashMap<Vec<String>, Vec<f64>> = AHashMap::new();
        let row_count = self.data.height();

        if !params.groupby.is_empty() {
            for i in 0..row_count {
                if let Some(val) = density_col.get(i).to_f64() {
                    let key: Vec<String> = group_columns
                        .iter()
                        .map(|col| col.get(i).to_string().unwrap_or_else(|| "null".to_string()))
                        .collect();
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
            groups.insert(Vec::new(), all_obs);
        }

        // --- STEP 4: Compute KDE per Group ---
        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        // One output column per groupby field.
        let mut final_groups: Vec<Vec<String>> = vec![Vec::new(); params.groupby.len()];

        for key in group_order {
            let observations = match groups.get(&key) {
                Some(obs) if !obs.is_empty() => obs,
                _ => continue,
            };

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

            for (field_index, label) in key.iter().enumerate() {
                final_groups[field_index].extend(std::iter::repeat_n(label.clone(), steps));
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

        for (field_index, field) in params.groupby.iter().enumerate() {
            new_ds.add_column(
                field,
                ColumnVector::String {
                    data: std::mem::take(&mut final_groups[field_index]),
                    validity: None,
                },
            )?;
        }

        // Replace chart data with the newly generated density dataset.
        self.data = new_ds;
        Ok(self)
    }
}
