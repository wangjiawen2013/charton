//! 2D density transform: turn scattered points into a smooth density grid.
//!
//! This is the statistical half of a density heatmap or a density contour. It
//! reads two numeric columns, estimates a bivariate kernel density on a regular
//! grid, and writes `(x, y, density)` back as a table of grid nodes. The result
//! can be drawn directly as a heatmap, or passed to
//! [`crate::transform::contour_transform::ContourTransform`] to become
//! iso-density lines.

use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::stats::kde::BandwidthType;
use crate::stats::kde2d::density_2d;

/// Configuration for [`Chart::transform_density_2d`].
#[derive(Debug, Clone)]
pub struct Density2DTransform {
    pub(crate) x: String,
    pub(crate) y: String,
    /// Output column names: `[x, y, density]`.
    pub(crate) as_: [String; 3],
    pub(crate) bandwidth: BandwidthType,
    /// Number of nodes per axis (the grid is square).
    pub(crate) grid: usize,
    /// Fraction by which the data range is widened, so the tails are visible.
    pub(crate) padding: f64,
}

impl Density2DTransform {
    /// Estimates the joint density of the `x` and `y` columns.
    pub fn new(x: impl Into<String>, y: impl Into<String>) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            as_: ["x".to_string(), "y".to_string(), "density".to_string()],
            bandwidth: BandwidthType::Scott,
            grid: 50,
            padding: 0.1,
        }
    }

    /// Renames the produced `[x, y, density]` columns.
    pub fn with_as(
        mut self,
        x: impl Into<String>,
        y: impl Into<String>,
        density: impl Into<String>,
    ) -> Self {
        self.as_ = [x.into(), y.into(), density.into()];
        self
    }

    /// Chooses the smoothing rule.
    pub const fn with_bandwidth(mut self, bandwidth: BandwidthType) -> Self {
        self.bandwidth = bandwidth;
        self
    }

    /// Sets the number of grid nodes per axis.
    pub fn with_grid_size(mut self, grid: usize) -> Self {
        self.grid = grid.max(2);
        self
    }

    /// Sets how far the grid is widened past the data, as a fraction.
    pub fn with_padding(mut self, padding: f64) -> Self {
        self.padding = padding.max(0.0);
        self
    }
}

impl<T: Mark> Chart<T> {
    /// Rewrites the dataset into a density grid.
    ///
    /// ```rust,ignore
    /// chart!(ds)?
    ///     .transform_density_2d(Density2DTransform::new("sepal_length", "petal_length"))?
    ///     .transform_contour(ContourTransform::new("x", "y", "density"))?
    ///     .mark_path()?
    ///     .encode((alt::x("x"), alt::y("y"),
    ///              alt::path_group("path_group"), alt::color("level")))?
    ///     .save("density_contour.svg")?;
    /// ```
    pub fn transform_density_2d(
        mut self,
        params: Density2DTransform,
    ) -> Result<Self, ChartonError> {
        // --- Step 1: collect the samples ---
        let row_count = self.data.height();
        let x_col = self.data.column(&params.x)?;
        let y_col = self.data.column(&params.y)?;

        let mut xs = Vec::with_capacity(row_count);
        let mut ys = Vec::with_capacity(row_count);
        for i in 0..row_count {
            if let (Some(x), Some(y)) = (x_col.get(i).to_f64(), y_col.get(i).to_f64())
                && x.is_finite()
                && y.is_finite()
            {
                xs.push(x);
                ys.push(y);
            }
        }

        if xs.is_empty() {
            self.data = Dataset::new();
            return Ok(self);
        }

        // --- Step 2: build the grid, widened so the tails are not clipped ---
        let span = |values: &[f64]| -> (f64, f64) {
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for &v in values {
                lo = lo.min(v);
                hi = hi.max(v);
            }
            if (hi - lo).abs() < f64::EPSILON {
                let pad = if lo == 0.0 { 1.0 } else { lo.abs() * 0.1 };
                (lo - pad, hi + pad)
            } else {
                let pad = (hi - lo) * params.padding;
                (lo - pad, hi + pad)
            }
        };

        let (x_lo, x_hi) = span(&xs);
        let (y_lo, y_hi) = span(&ys);
        let n = params.grid;
        let axis = |lo: f64, hi: f64| -> Vec<f64> {
            (0..n)
                .map(|i| lo + (hi - lo) * i as f64 / (n - 1) as f64)
                .collect()
        };
        let grid_x = axis(x_lo, x_hi);
        let grid_y = axis(y_lo, y_hi);

        // --- Step 3: estimate and flatten ---
        let z = density_2d(&xs, &ys, &grid_x, &grid_y, params.bandwidth);

        let mut out_x = Vec::with_capacity(n * n);
        let mut out_y = Vec::with_capacity(n * n);
        for j in 0..n {
            for i in 0..n {
                out_x.push(grid_x[i]);
                out_y.push(grid_y[j]);
            }
        }

        // --- Step 4: assemble the new dataset ---
        let mut new_data = Dataset::new();
        new_data.add_column(
            &params.as_[0],
            ColumnVector::Float64 {
                data: out_x,
                validity: None,
            },
        )?;
        new_data.add_column(
            &params.as_[1],
            ColumnVector::Float64 {
                data: out_y,
                validity: None,
            },
        )?;
        new_data.add_column(
            &params.as_[2],
            ColumnVector::Float64 {
                data: z,
                validity: None,
            },
        )?;

        self.data = new_data;
        Ok(self)
    }
}
