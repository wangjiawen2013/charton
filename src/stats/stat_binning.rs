use crate::core::data::ColumnVector;
use crate::core::utils::Parallelizable;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Categorizes continuous numerical data into discrete bins. It maps each value to a label based on
/// defined bin edges.
///
/// # Arguments
/// * `values` - A slice of f64 data points to be binned.
/// * `validity` - An optional bitmask (from ColumnVector) to handle null values.
/// * `bins` - A sorted slice of bin edges (e.g., `[0.0, 10.0, 20.0]`).
/// * `labels` - String labels corresponding to each bin (length must be `bins.len() - 1`).
///
/// # Returns
/// A vector of `Option<String>`, where `None` represents a null or out-of-bounds value.
pub(crate) fn cut(
    values: &[f64],
    validity: &Option<Vec<u8>>,
    bins: &[f64],
    labels: &[String],
) -> Vec<Option<String>> {
    // Pre-allocate the result vector to match input size
    values
        .maybe_par_iter()
        .enumerate()
        .map(|(i, &val)| {
            // 1. Check for nulls in the bitmask or NaN values in the float data
            if !ColumnVector::is_valid_in_mask(validity, i) || val.is_nan() {
                return None;
            }

            // 2. Perform high-performance binary search to find the correct bin
            let bin_idx = find_bin(val, bins);

            // 3. Return the corresponding label
            Some(labels[bin_idx].clone())
        })
        .collect()
}

/// Efficiently finds the bin index for a given value using Binary Search.
///
/// This implementation follows the standard "left-closed, right-open" [a, b) interval
/// logic, except for the final bin which is fully closed [a, b] to include
/// the maximum edge value.
///
/// # Logic
/// - If `val` is exactly the last edge, it is included in the last bin.
/// - If `val` is below the first edge or above the last, it is clamped to the nearest bin.
fn find_bin(value: f64, bins: &[f64]) -> usize {
    let last_bin_idx = bins.len() - 2;
    let last_edge = bins[bins.len() - 1];

    // Special case: The value hits the exact upper bound of the last bin
    if value >= last_edge {
        return last_bin_idx;
    }

    // Binary search O(log N) is significantly faster than linear windows() O(N)
    match bins.binary_search_by(|probe| {
        probe
            .partial_cmp(&value)
            .expect("Failed to compare floating point values")
    }) {
        // Exact match found: value is the start of the interval [value, next_edge)
        Ok(idx) => {
            // If the exact match is the last edge, it belongs to the previous bin
            if idx > last_bin_idx {
                last_bin_idx
            } else {
                idx
            }
        }
        // No exact match: `err` is the index where the value would be inserted.
        // Therefore, the value falls into the bin starting at `err - 1`.
        Err(err) => {
            if err == 0 {
                0
            } else if err > last_bin_idx {
                last_bin_idx
            } else {
                err - 1
            }
        }
    }
}
