//! Contour transform: turn a scalar field into iso-lines.
//!
//! This is the statistical half of a contour plot. It reads a value `z` sampled
//! on a regular `x`/`y` grid — either the customer's own data or the output of
//! [`crate::transform::density_2d_transform::Density2DTransform`] — and, for each
//! requested level, extracts the polylines where `z` equals that level. The
//! output is a plain `(x, y, path_group, level)` table that
//! [`crate::chart::Chart::mark_path`] draws directly.
//!
//! This is an **iso-line** (marching squares) statistic, not an iso-band one:
//! it emits the boundary lines, not the filled regions between two levels.
//! Filling the regions would need a separate iso-band/polygon-clipping step.
//!
//! Like every other statistic here, it is not contour-specific: any regular grid
//! of `x`, `y` and `z` can be contoured.

use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::stats::contour::contours;
use ahash::AHashMap;

/// How many iso-levels to draw, or which ones.
#[derive(Debug, Clone)]
pub enum ContourLevels {
    /// This many evenly spaced levels strictly inside the data range.
    Count(usize),
    /// Exactly these level values.
    Values(Vec<f64>),
}

impl From<usize> for ContourLevels {
    fn from(n: usize) -> Self {
        Self::Count(n)
    }
}

impl From<Vec<f64>> for ContourLevels {
    fn from(values: Vec<f64>) -> Self {
        Self::Values(values)
    }
}

/// Configuration for [`Chart::transform_contour`].
#[derive(Debug, Clone)]
pub struct ContourTransform {
    pub(crate) x: String,
    pub(crate) y: String,
    pub(crate) z: String,
    pub(crate) levels: ContourLevels,
    /// Output column names: `[x, y, path_group]`.
    pub(crate) as_: [String; 3],
    /// Output column holding each line's level.
    pub(crate) level_as: String,
}

impl ContourTransform {
    /// Contours the field `z` sampled on the regular grid of `x` and `y`.
    pub fn new(x: impl Into<String>, y: impl Into<String>, z: impl Into<String>) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            z: z.into(),
            levels: ContourLevels::Count(8),
            as_: ["x".to_string(), "y".to_string(), "path_group".to_string()],
            level_as: "level".to_string(),
        }
    }

    /// Draws `n` evenly spaced levels strictly inside the data range.
    pub fn with_levels(mut self, n: usize) -> Self {
        self.levels = ContourLevels::Count(n.max(1));
        self
    }

    /// Draws exactly these level values.
    pub fn with_levels_values(mut self, values: impl Into<Vec<f64>>) -> Self {
        self.levels = ContourLevels::Values(values.into());
        self
    }

    /// Renames the produced `[x, y, path_group]` columns.
    pub fn with_as(
        mut self,
        x: impl Into<String>,
        y: impl Into<String>,
        path_group: impl Into<String>,
    ) -> Self {
        self.as_ = [x.into(), y.into(), path_group.into()];
        self
    }

    /// Renames the produced level column.
    pub fn with_level_as(mut self, name: impl Into<String>) -> Self {
        self.level_as = name.into();
        self
    }
}

impl<T: Mark> Chart<T> {
    /// Rewrites the dataset into a contour-line table.
    ///
    /// ```rust,ignore
    /// chart!(ds)?
    ///     .transform_density_2d(Density2DTransform::new("x", "y"))?
    ///     .transform_contour(ContourTransform::new("x", "y", "density").with_levels(8))?
    ///     .mark_path()?
    ///     .encode((alt::x("x"), alt::y("y"),
    ///              alt::path_group("path_group"), alt::color("level")))?
    ///     .save("contour.svg")?;
    /// ```
    pub fn transform_contour(mut self, params: ContourTransform) -> Result<Self, ChartonError> {
        // --- Step 1: pull the samples out of the source columns ---
        let row_count = self.data.height();
        let x_col = self.data.column(&params.x)?;
        let y_col = self.data.column(&params.y)?;
        let z_col = self.data.column(&params.z)?;

        let mut samples: Vec<(f64, f64, f64)> = Vec::with_capacity(row_count);
        for i in 0..row_count {
            if let (Some(x), Some(y), Some(z)) = (
                x_col.get(i).to_f64(),
                y_col.get(i).to_f64(),
                z_col.get(i).to_f64(),
            ) && x.is_finite()
                && y.is_finite()
                && z.is_finite()
            {
                samples.push((x, y, z));
            }
        }

        // --- Step 2: recover the regular grid (unique, ascending x and y) ---
        let mut xs: Vec<f64> = samples.iter().map(|s| s.0).collect();
        let mut ys: Vec<f64> = samples.iter().map(|s| s.1).collect();
        xs.sort_by(f64::total_cmp);
        ys.sort_by(f64::total_cmp);
        xs.dedup();
        ys.dedup();

        let nx = xs.len();
        let ny = ys.len();
        if nx < 2 || ny < 2 {
            return Err(ChartonError::Data(
                "Contour needs a grid of at least 2 x 2 points".into(),
            ));
        }

        let x_index: AHashMap<u64, usize> = xs
            .iter()
            .enumerate()
            .map(|(i, v)| (v.to_bits(), i))
            .collect();
        let y_index: AHashMap<u64, usize> = ys
            .iter()
            .enumerate()
            .map(|(j, v)| (v.to_bits(), j))
            .collect();

        let mut grid = vec![f64::NAN; nx * ny];
        for (x, y, z) in &samples {
            if let (Some(&i), Some(&j)) = (x_index.get(&x.to_bits()), y_index.get(&y.to_bits())) {
                grid[j * nx + i] = *z;
            }
        }

        // --- Step 3: resolve the levels ---
        let (min, max) = grid
            .iter()
            .filter(|v| v.is_finite())
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
                (lo.min(v), hi.max(v))
            });
        if !min.is_finite() || !max.is_finite() || (max - min).abs() < f64::EPSILON {
            // A flat (or empty) field has no contour to draw.
            self.data = Dataset::new();
            return Ok(self);
        }

        let levels: Vec<f64> = match &params.levels {
            ContourLevels::Values(values) => values.clone(),
            ContourLevels::Count(n) => {
                let n = *n;
                (1..=n)
                    .map(|k| min + (max - min) * k as f64 / (n + 1) as f64)
                    .collect()
            }
        };

        // --- Step 4: extract and flatten the polylines ---
        let lines = contours(&xs, &ys, &grid, &levels);

        let mut out_x = Vec::new();
        let mut out_y = Vec::new();
        let mut out_id = Vec::new();
        let mut out_level = Vec::new();

        for (index, line) in lines.iter().enumerate() {
            let id = format!("contour-{index}");
            for (x, y) in &line.points {
                out_x.push(*x);
                out_y.push(*y);
                out_id.push(id.clone());
                out_level.push(line.level);
            }
        }

        // --- Step 5: assemble the new dataset ---
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
            ColumnVector::String {
                data: out_id,
                validity: None,
            },
        )?;
        new_data.add_column(
            &params.level_as,
            ColumnVector::Float64 {
                data: out_level,
                validity: None,
            },
        )?;

        self.data = new_data;
        Ok(self)
    }
}
