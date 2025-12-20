/// Apply LOESS (Locally Estimated Scatterplot Smoothing) to data points
///
/// # Arguments
/// * `x` - x coordinates of data points
/// * `y` - y coordinates of data points  
/// * `bandwidth` - bandwidth parameter (0.0 to 1.0), controls the degree of smoothing
///
/// # Returns
/// A tuple of (smoothed_x, smoothed_y) vectors
pub(crate) fn loess(x: &[f64], y: &[f64], bandwidth: f64) -> (Vec<f64>, Vec<f64>) {
    let n = x.len();
    // Number of points to consider for each local regression
    let bandwidth_size = (n as f64 * bandwidth).max(1.0).min(n as f64) as usize;

    // Compute LOESS smoothed values
    let mut smoothed_x = Vec::with_capacity(n);
    let mut smoothed_y = Vec::with_capacity(n);

    for i in 0..n {
        let target_x = x[i];

        // Find nearest neighbors
        let mut distances: Vec<(usize, f64)> = x
            .iter()
            .enumerate()
            .map(|(j, &x_val)| (j, (x_val - target_x).abs()))
            .collect();

        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        distances.truncate(bandwidth_size);

        // Calculate tricube weights
        let max_dist = distances.last().map(|d| d.1).unwrap_or(1.0);
        if max_dist == 0.0 {
            smoothed_x.push(target_x);
            smoothed_y.push(y[i]);
            continue;
        }

        let weights: Vec<f64> = distances
            .iter()
            .map(|(_, d)| {
                let rel_dist = d / max_dist;
                (1.0 - rel_dist.powi(3)).powi(3).max(0.0)
            })
            .collect();

        // Weighted linear regression
        let weighted_x: Vec<f64> = distances.iter().map(|(j, _)| x[*j]).collect();
        let weighted_y: Vec<f64> = distances.iter().map(|(j, _)| y[*j]).collect();

        if let Some(pred) = weighted_linear_regression(&weighted_x, &weighted_y, &weights) {
            smoothed_x.push(target_x);
            smoothed_y.push(pred);
        } else {
            smoothed_x.push(target_x);
            smoothed_y.push(y[i]); // Fallback to original value
        }
    }

    (smoothed_x, smoothed_y)
}

/// Perform weighted linear regression
///
/// # Arguments
/// * `x` - x coordinates
/// * `y` - y coordinates
/// * `weights` - weights for each point
///
/// # Returns
/// Predicted y value at the first x point, or None if regression fails
fn weighted_linear_regression(x: &[f64], y: &[f64], weights: &[f64]) -> Option<f64> {
    if x.is_empty() || x.len() != y.len() || x.len() != weights.len() {
        return None;
    }

    let sum_w: f64 = weights.iter().sum();
    if sum_w == 0.0 {
        return None;
    }

    let sum_wx: f64 = weights.iter().zip(x).map(|(w, &x_val)| w * x_val).sum();
    let sum_wy: f64 = weights.iter().zip(y).map(|(w, &y_val)| w * y_val).sum();
    let sum_wxx: f64 = weights
        .iter()
        .zip(x)
        .map(|(w, &x_val)| w * x_val * x_val)
        .sum();
    let sum_wxy: f64 = weights
        .iter()
        .zip(x.iter().zip(y))
        .map(|(w, (&x_val, &y_val))| w * x_val * y_val)
        .sum();

    let denom = sum_w * sum_wxx - sum_wx * sum_wx;
    if denom.abs() < 1e-10 {
        // Return weighted mean if we can't do regression
        return Some(sum_wy / sum_w);
    }

    // Evaluate at the target point (first x value)
    let target_x = x[0];
    let slope = (sum_w * sum_wxy - sum_wx * sum_wy) / denom;
    let intercept = (sum_wy - slope * sum_wx) / sum_w;

    Some(intercept + slope * target_x)
}
