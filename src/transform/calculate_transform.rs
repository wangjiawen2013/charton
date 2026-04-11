use crate::chart::Chart;
use crate::core::data::{ColumnVector, RowAccessor};
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::Mark;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

impl<T: Mark> Chart<T> {
    /// Transforms the chart's data by calculating a new column using a native Rust closure.
    ///
    /// This follows the declarative pattern of libraries like Altair, allowing users
    /// to derive new fields directly within the chart's fluent API.
    ///
    /// # Performance
    /// It utilizes `RowAccessor` for $O(1)$ column access per row and leverages
    /// `rayon` for parallel execution across all CPU cores when the "parallel"
    /// feature is enabled.
    ///
    /// # Example
    /// ```rust
    /// chart.transform_calculate("bmi", |row| {
    ///     let w = row.val("weight")?;
    ///     let h = row.val("height")?;
    ///     Some(w / (h * h))
    /// })?;
    /// ```
    pub fn transform_calculate<F>(mut self, as_name: &str, f: F) -> Result<Self, ChartonError>
    where
        F: Fn(RowAccessor) -> Option<f64> + Sync + Send,
    {
        let row_count = self.data.height();

        // Return early if there is no data to process
        if row_count == 0 {
            return Ok(self);
        }

        // Shared reference to the dataset to be captured by the parallel closure.
        // Dataset methods (get_f64, get_str) are read-only and thread-safe.
        let ds_ref = &self.data;

        // --- PARALLEL EXECUTION ---
        // We calculate the new values and track validity (nulls) simultaneously.
        // Using unzip() is efficient for creating two separate contiguous vectors.
        let (new_data, _validity): (Vec<f64>, Vec<bool>) = (0..row_count)
            .maybe_into_par_iter()
            .map(|i| {
                let accessor = RowAccessor::new(ds_ref, i);
                match f(accessor) {
                    Some(val) => (val, true),
                    None => (0.0, false), // Default to 0.0 for Nulls in the data vector
                }
            })
            .unzip();

        // --- ATOMIC UPDATE ---
        // Push the new column into the internal dataset.
        // This validates row length consistency automatically.
        self.data.add_column(
            as_name,
            ColumnVector::F64 {
                data: new_data,
                // If your ColumnVector supports bitmasks, you can pass 'validity' here.
            },
        )?;

        Ok(self)
    }
}
