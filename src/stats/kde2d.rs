//! Bivariate kernel density estimation on a regular grid.
//!
//! This is the 2D counterpart of [`crate::stats::kde`]: given scattered `(x, y)`
//! points, it estimates how crowded the plane is around every grid node. The
//! result is the scalar field that a heatmap shows as colour or that
//! [`crate::transform::contour_transform`] turns into density contours.
//!
//! Dependencies are kept minimal: a product of two Gaussian kernels, one per
//! axis. That is enough for a chart, and it matches what the 1D estimator uses.

use crate::stats::kde::BandwidthType;

/// Sample standard deviation (`n - 1` denominator), zero for a single value.
fn std_dev(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    variance.sqrt()
}

/// Interquartile range, used by Silverman's rule.
fn iqr(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let pick = |q: f64| -> f64 {
        let pos = q * (sorted.len() - 1) as f64;
        let base = pos.floor() as usize;
        let frac = pos - base as f64;
        if base + 1 < sorted.len() {
            sorted[base] + frac * (sorted[base + 1] - sorted[base])
        } else {
            sorted[base]
        }
    };
    pick(0.75) - pick(0.25)
}

/// Bandwidth for each axis, shared by the whole grid.
///
/// The rule-of-thumb widths can collapse to zero (constant data); falling back
/// to a small positive value keeps the arithmetic finite and the estimate
/// smooth.
fn bandwidths(xs: &[f64], ys: &[f64], bandwidth: BandwidthType) -> (f64, f64) {
    let n = xs.len() as f64;
    // Scott's exponent for two dimensions is n^(-1/6).
    let scale = n.powf(-1.0 / 6.0);

    let axis = |values: &[f64]| -> f64 {
        let sd = std_dev(values);
        match bandwidth {
            BandwidthType::Fixed(h) => h,
            BandwidthType::Scott => sd * scale,
            BandwidthType::Silverman => {
                let robust = iqr(values) / 1.349;
                let spread = if sd > 0.0 && robust > 0.0 {
                    sd.min(robust)
                } else if sd > 0.0 {
                    sd
                } else {
                    robust
                };
                0.9 * spread * scale
            }
        }
    };

    let guard = |h: f64, values: &[f64]| -> f64 {
        if h.is_finite() && h > 0.0 {
            h
        } else {
            let magnitude = values.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
            if magnitude > 0.0 {
                magnitude * 1e-6
            } else {
                1.0
            }
        }
    };

    (guard(axis(xs), xs), guard(axis(ys), ys))
}

/// Estimates the density of `(xs, ys)` at every grid node.
///
/// `grid_x` and `grid_y` are the node coordinates, ascending. The result is
/// row-major: `z[j * grid_x.len() + i]` is the density at
/// `(grid_x[i], grid_y[j])`, ready for [`crate::stats::contour::contours`].
pub(crate) fn density_2d(
    xs: &[f64],
    ys: &[f64],
    grid_x: &[f64],
    grid_y: &[f64],
    bandwidth: BandwidthType,
) -> Vec<f64> {
    let nx = grid_x.len();
    let ny = grid_y.len();
    let n = xs.len();
    if n == 0 || nx == 0 || ny == 0 {
        return vec![0.0; nx * ny];
    }

    let (hx, hy) = bandwidths(xs, ys, bandwidth);
    let prefactor = 1.0 / (n as f64 * hx * hy * 2.0 * std::f64::consts::PI);

    let mut out = vec![0.0; nx * ny];
    for j in 0..ny {
        let gy = grid_y[j];
        for i in 0..nx {
            let gx = grid_x[i];
            let mut sum = 0.0;
            for k in 0..n {
                let dx = (gx - xs[k]) / hx;
                let dy = (gy - ys[k]) / hy;
                sum += (-0.5 * (dx * dx + dy * dy)).exp();
            }
            out[j * nx + i] = prefactor * sum;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(min: f64, max: f64, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| min + (max - min) * i as f64 / (n - 1) as f64)
            .collect()
    }

    #[test]
    fn density_peaks_where_points_cluster() {
        // All points near the origin; the centre node must be the tallest.
        let xs = vec![0.0, 0.1, -0.1, 0.05, -0.05];
        let ys = vec![0.0, 0.05, -0.05, 0.1, -0.1];
        let g = grid(-2.0, 2.0, 21);
        let z = density_2d(&xs, &ys, &g, &g, BandwidthType::Scott);

        let idx = |v: f64| g.iter().position(|&x| (x - v).abs() < 1e-9).unwrap();
        let centre = z[idx(0.0) * 21 + idx(0.0)];
        let corner = z[0];
        assert!(centre > corner, "centre {centre} vs corner {corner}");
    }

    #[test]
    fn constant_input_stays_finite() {
        let xs = vec![2.0; 5];
        let ys = vec![3.0; 5];
        let g = grid(1.0, 3.0, 11);
        let z = density_2d(&xs, &ys, &g, &g, BandwidthType::Scott);
        assert!(z.iter().all(|v| v.is_finite()));
    }
}
