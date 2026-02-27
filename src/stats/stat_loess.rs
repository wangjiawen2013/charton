/// This module provides an implementation of LOESS (Locally Estimated Scatterplot Smoothing).
/// It is designed for high-performance rendering pipelines by minimizing allocations
/// and using efficient partial-sorting algorithms.

/// Apply LOESS smoothing to a set of data points.
///
/// # Arguments
/// * `x` - The horizontal coordinates (independent variable).
/// * `y` - The vertical coordinates (dependent variable).
/// * `bandwidth` - A value between 0.0 and 1.0 representing the fraction of
///                 points used for local regression.
///
/// # Returns
/// A tuple containing (original_x, smoothed_y).
pub(crate) fn loess(x: &[f64], y: &[f64], bandwidth: f64) -> (Vec<f64>, Vec<f64>) {
    let n = x.len();
    // LOESS requires at least 2 points for a linear fit.
    if n < 2 {
        return (x.to_vec(), y.to_vec());
    }

    // k is the number of neighbors included in the local window.
    let k = (n as f64 * bandwidth).max(2.0).min(n as f64) as usize;

    let mut smoothed_y = Vec::with_capacity(n);

    // PERF: Reuse a single distance buffer to avoid O(N) allocations.
    let mut distances: Vec<(usize, f64)> = Vec::with_capacity(n);

    for i in 0..n {
        let target_x = x[i];

        // 1. Calculate absolute distances from target_x to all other points.
        distances.clear();
        for (j, &val) in x.iter().enumerate() {
            distances.push((j, (val - target_x).abs()));
        }

        // 2. PERF(Performance): Partial sort using Quickselect (select_nth_unstable_by).
        // This finds the k-nearest neighbors in O(N) average time,
        // significantly faster than a full O(N log N) sort.
        // We use partial_cmp().unwrap_or because distances are non-NaN f64s.
        let (neighbors, _, _) = distances.select_nth_unstable_by(k - 1, |a, b| {
            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        // 3. Determine the maximum distance in the neighborhood for weighting.
        let max_dist = neighbors.iter().map(|d| d.1).fold(0.0, f64::max);

        // If all neighbors are at the same X, local regression is vertical (invalid).
        if max_dist == 0.0 {
            smoothed_y.push(y[i]);
            continue;
        }

        // 4. Perform weighted linear regression on the neighborhood.
        if let Some(pred) =
            weighted_linear_regression_optimized(x, y, neighbors, max_dist, target_x)
        {
            smoothed_y.push(pred);
        } else {
            // Fallback to the original value if the regression fails (e.g., singular matrix).
            smoothed_y.push(y[i]);
        }
    }

    (x.to_vec(), smoothed_y)
}

/// A memory-efficient weighted linear regression implementation.
///
/// Instead of creating new vectors for the local subset, this indices into the
/// original data using the indices found during neighbor selection.
fn weighted_linear_regression_optimized(
    original_x: &[f64],
    original_y: &[f64],
    neighbors: &[(usize, f64)],
    max_dist: f64,
    target_x: f64,
) -> Option<f64> {
    let mut sum_w = 0.0;
    let mut sum_wx = 0.0;
    let mut sum_wy = 0.0;
    let mut sum_wxx = 0.0;
    let mut sum_wxy = 0.0;

    // Use the Tricube Weighting Function: W(u) = (1 - |u|^3)^3 for |u| < 1
    for &(idx, dist) in neighbors {
        let u = dist / max_dist;
        let w = (1.0 - u.powi(3)).powi(3).max(0.0);

        let xi = original_x[idx];
        let yi = original_y[idx];

        sum_w += w;
        sum_wx += w * xi;
        sum_wy += w * yi;
        sum_wxx += w * xi * xi;
        sum_wxy += w * xi * yi;
    }

    if sum_w == 0.0 {
        return None;
    }

    // Solve for the linear fit: y = intercept + slope * x
    // Using the Cramer's rule/determinant method for 2x2 matrix.
    let denom = sum_w * sum_wxx - sum_wx * sum_wx;

    if denom.abs() < 1e-12 {
        // Points are vertically aligned; return weighted average y instead.
        return Some(sum_wy / sum_w);
    }

    let slope = (sum_w * sum_wxy - sum_wx * sum_wy) / denom;
    let intercept = (sum_wy - slope * sum_wx) / sum_w;

    // The smoothed value is the prediction at the specific target_x.
    Some(intercept + slope * target_x)
}
