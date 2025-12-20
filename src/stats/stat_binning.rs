use polars::prelude::*;

/// Cut continuous data into discrete bins
///
/// # Arguments
/// * `series` - A Series containing continuous data (f64)
/// * `bins` - A vector of bin edges (must be sorted in ascending order)
/// * `labels` - Labels for the bins
///
/// # Returns
/// A String Series with bin labels
pub(crate) fn cut(series: &Series, bins: &[f64], labels: &[String]) -> Series {
    let values: Vec<f64> = series.f64().unwrap().into_no_null_iter().collect();
    let mut result_categories = Vec::with_capacity(values.len());

    for val in values {
        let index = find_bin(val, bins);
        result_categories.push(labels[index].clone());
    }

    Series::new(series.name().clone(), result_categories)
}

/// Find the bin for value using left-closed, right-open [a, b) interval.
///
/// This function determines which bin a value belongs to based on the provided bin edges.
/// It uses a left-closed, right-open interval for all bins except the last one, which
/// includes the rightmost edge to handle the maximum value.
///
/// # Arguments
/// * `value` - The value to find a bin for
/// * `bins` - A slice of bin edges, must be sorted in ascending order
///
/// # Returns
/// The index of the bin that contains the value
///
/// # Examples
/// ```
/// let bins = &[0.0, 1.0, 2.0, 3.0];
/// assert_eq!(find_bin(0.5, bins), 0);  // Falls in [0.0, 1.0)
/// assert_eq!(find_bin(1.0, bins), 1);  // Falls in [1.0, 2.0)
/// assert_eq!(find_bin(3.0, bins), 2);  // Equals right edge, falls in [2.0, 3.0]
/// ```

// `bins` must be sorted. If `value == bins.last()`, it goes to the last bin.
fn find_bin(value: f64, bins: &[f64]) -> usize {
    // Search using iterator windows of size 2: (bins[i], bins[i+1])
    if let Some(index) = bins.windows(2).position(|w| value >= w[0] && value < w[1]) {
        return index;
    }

    // If not found, value must equal the last right edge.
    bins.len() - 2
}
