//! One-dimensional kernel density estimation.
//!
//! `transform_density` turns a numeric column into a smooth density curve: a
//! new table with a row for every evaluation point and the estimated density
//! there. It is the statistical half of a density plot, a violin or a ridgeline.
//!
//! The transform only writes numbers; it never draws. Turning the curve into a
//! shape is the geometry's job: `mark_area` with a mirror stack for a smooth
//! density plot, or `transform_band` for a violin.
//!
//! # Missing values
//!
//! A row whose density value is missing is skipped. A missing value in a
//! `groupby` column is recorded as the reserved [`MISSING_CATEGORY`] level
//! rather than dropped: whether it survives is then decided by how that column
//! is used downstream — a positional `center` drops it, a non-positional
//! `group` or `color` keeps it. See `MISSING_CATEGORY` for the full policy.
//!
//! [`MISSING_CATEGORY`]: crate::core::data::MISSING_CATEGORY

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
    // When true, evaluate the density only over each group's observed range,
    // so a violin ends at its own min/max. Off by default, which keeps the
    // smooth tails of a density plot (ggplot2 `geom_density`).
    pub(crate) trim: bool,
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
            trim: false,
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

    /// Trims the curve to the observed range of each group.
    ///
    /// `false` (the default) evaluates every group on one shared range extended
    /// by 30% on each side, so the smooth tails stay visible — the density-plot
    /// convention (`geom_density`). `true` matches ggplot2's violin
    /// (`trim = TRUE`) and Altair's violin: each group ends at its own smallest
    /// and largest observation, so groups with different spread get different
    /// heights and no near-zero tails. The violin examples set this to `true`.
    pub const fn with_trim(mut self, trim: bool) -> Self {
        self.trim = trim;
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
    ///
    /// By default every group shares one range extended by 30%, so the smooth
    /// tails of a density plot stay visible. Set
    /// [`with_trim(true)`](DensityTransform::with_trim) for violins, where each
    /// group is evaluated over its own observed range instead.
    pub fn transform_density(mut self, params: DensityTransform) -> Result<Self, ChartonError> {
        let density_field = &params.density;
        let density_col = self.data.column(density_field)?;

        // The estimate's output columns must not clash with the group columns
        // that are copied back into the output table.
        let mut output_names: Vec<&str> = vec![params.as_[0].as_str(), params.as_[1].as_str()];
        output_names.extend(params.groupby.iter().map(String::as_str));
        crate::transform::ensure_distinct_columns(&output_names)?;

        // 200 points keeps a curve smooth without being expensive.
        const STEPS: usize = 200;

        // --- STEP 0: Untrimmed (shared) evaluation grid ---
        // The default density plot evaluates every group on one range covering
        // all rows, extended by 30% so the tails fade out rather than being cut
        // at the last observation. This is the behaviour `geom_density` has.
        let untrimmed_grid: Option<Vec<f64>> = if params.trim {
            None
        } else {
            let (min_val, max_val) = density_col.min_max();
            let mut lo = 1.3 * min_val - 0.3 * max_val;
            let mut hi = 1.3 * max_val - 0.3 * min_val;
            // A constant column has no spread; give it a small window so the
            // estimator produces a finite spike rather than NaNs.
            if (hi - lo).abs() < 1e-12 {
                let offset = if lo == 0.0 { 1.0 } else { lo.abs() * 0.1 };
                lo -= offset;
                hi += offset;
            }
            let step = (hi - lo) / (STEPS as f64);
            Some((0..STEPS).map(|i| lo + (i as f64) * step).collect())
        };

        // --- STEP 1: Establish Deterministic Order ---
        // The keys are the tuples of the groupby fields, in first-appearance
        // order, so curves line up with the legend. With no groupby there is a
        // single, empty key.
        let group_columns: Vec<&ColumnVector> = params
            .groupby
            .iter()
            .map(|field| self.data.column(field))
            .collect::<Result<Vec<_>, _>>()?;

        // A missing value in a groupby column becomes the reserved "NA" level.
        // Whether it survives depends on how the field is used downstream:
        // `lane_layout` drops it for a positional `category` but keeps it for a
        // non-positional `group`, and colour keeps it (grey).
        let group_key = |i: usize| -> Vec<String> {
            group_columns
                .iter()
                .map(|col| col.label_with_missing(i))
                .collect()
        };

        let group_order: Vec<Vec<String>> = if params.groupby.is_empty() {
            vec![Vec::new()]
        } else {
            let mut seen = AHashSet::new();
            let mut order = Vec::new();
            for i in 0..self.data.height() {
                let key = group_key(i);
                if seen.insert(key.clone()) {
                    order.push(key);
                }
            }
            order
        };

        // --- STEP 2: Aggregate Observations by Group ---
        let mut groups: AHashMap<Vec<String>, Vec<f64>> = AHashMap::new();
        let row_count = self.data.height();

        if !params.groupby.is_empty() {
            for i in 0..row_count {
                let Some(val) = density_col.get(i).to_f64() else {
                    continue;
                };
                let key = group_key(i);
                groups.entry(key).or_default().push(val);
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

        // --- STEP 3: Compute KDE per Group ---
        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        // One output column per groupby field.
        let mut final_groups: Vec<Vec<String>> = vec![Vec::new(); params.groupby.len()];

        for key in group_order {
            let observations = match groups.get(&key) {
                Some(obs) if !obs.is_empty() => obs,
                _ => continue,
            };

            // Trimmed mode evaluates each group over its own observed range, so
            // two groups never share a grid and a violin ends at the data
            // extremes instead of drawing a thin near-zero tail. Untrimmed mode
            // reuses the one shared grid computed in STEP 0.
            let eval_points: Vec<f64> = match &untrimmed_grid {
                Some(grid) => grid.clone(),
                None => {
                    let (min_val, max_val) = observations
                        .iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
                            (lo.min(v), hi.max(v))
                        });
                    let (mut lo, mut hi) = (min_val, max_val);
                    // A constant group has no spread; give it a small symmetric
                    // window so the estimator produces a finite spike.
                    if (hi - lo).abs() < 1e-12 {
                        let offset = if lo == 0.0 { 1.0 } else { lo.abs() * 0.1 };
                        lo -= offset;
                        hi += offset;
                    }
                    (0..STEPS)
                        .map(|i| lo + (hi - lo) * (i as f64) / ((STEPS - 1) as f64))
                        .collect()
                }
            };

            // Build one estimate for this group and read it at its own points.
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
            final_x.extend(eval_points);

            for (field_index, label) in key.iter().enumerate() {
                final_groups[field_index].extend(std::iter::repeat_n(label.clone(), STEPS));
            }
        }

        // --- STEP 4: Build Final Dataset ---
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::Chart;
    use crate::core::data::{ColumnVector, Dataset};
    use crate::mark::no_mark::NoMark;

    fn chart(values: Vec<f64>, groups: Vec<&str>) -> Chart<NoMark> {
        let mut ds = Dataset::new();
        ds.add_column(
            "value",
            ColumnVector::Float64 {
                data: values,
                validity: None,
            },
        )
        .unwrap();
        ds.add_column(
            "grp",
            ColumnVector::String {
                data: groups.into_iter().map(str::to_string).collect(),
                validity: None,
            },
        )
        .unwrap();
        Chart::<NoMark>::build(ds).unwrap()
    }

    fn range_of(chart: &Chart<NoMark>, group: &str) -> (f64, f64) {
        let x = chart.data.column("value").unwrap().to_f64_vec();
        let groups = chart.data.column("grp").unwrap();
        x.iter()
            .enumerate()
            .filter(|(i, _)| groups.get(*i).to_string().as_deref() == Some(group))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), (_, v)| {
                (lo.min(*v), hi.max(*v))
            })
    }

    /// With `trim`, each group owns its evaluation grid, so two groups of
    /// different spread produce curves of different height.
    #[test]
    fn trimmed_groups_get_their_own_range() {
        let chart = chart(vec![0.0, 10.0, 100.0, 110.0], vec!["A", "A", "B", "B"])
            .transform_density(
                DensityTransform::new("value")
                    .with_groupbys(["grp"])
                    .with_trim(true),
            )
            .unwrap();

        let a = range_of(&chart, "A");
        let b = range_of(&chart, "B");
        assert!(
            (a.0 - 0.0).abs() < 1e-9 && (a.1 - 10.0).abs() < 1e-9,
            "A={a:?}"
        );
        assert!(
            (b.0 - 100.0).abs() < 1e-9 && (b.1 - 110.0).abs() < 1e-9,
            "B={b:?}"
        );
    }

    /// The default (untrimmed) keeps one shared, extended grid for every group,
    /// which is what a density plot expects.
    #[test]
    fn untrimmed_groups_share_one_extended_range() {
        let chart = chart(vec![0.0, 10.0, 100.0, 110.0], vec!["A", "A", "B", "B"])
            .transform_density(DensityTransform::new("value").with_groupbys(["grp"]))
            .unwrap();

        let a = range_of(&chart, "A");
        let b = range_of(&chart, "B");
        assert!(
            (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9,
            "A={a:?} B={b:?}"
        );
        assert!(a.0 < 0.0 && a.1 > 110.0, "range=({}, {})", a.0, a.1);
    }

    /// A null groupby value becomes the reserved `"NA"` level instead of being
    /// dropped, so a valid observation is not lost.
    #[test]
    fn null_groups_become_the_missing_level() {
        let mut ds = Dataset::new();
        ds.add_column(
            "value",
            ColumnVector::Float64 {
                data: vec![1.0, 2.0, 3.0, 4.0],
                validity: None,
            },
        )
        .unwrap();
        // grp: ["a", null, "a", "b"]
        ds.add_column(
            "grp",
            ColumnVector::String {
                data: vec!["a".into(), "ignored".into(), "a".into(), "b".into()],
                validity: Some(vec![0b1101]),
            },
        )
        .unwrap();

        let chart = Chart::<NoMark>::build(ds)
            .unwrap()
            .transform_density(
                DensityTransform::new("value")
                    .with_groupbys(["grp"])
                    .with_trim(true),
            )
            .unwrap();

        // `labels_with_missing` is what the colour domain uses: NA is present
        // and placed last, the two real groups keep their order.
        let groups = chart.data.column("grp").unwrap().labels_with_missing();
        assert_eq!(
            groups,
            vec!["a".to_string(), "b".to_string(), "NA".to_string()]
        );
    }
}
