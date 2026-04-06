use crate::chart::Chart;
use crate::core::data::ColumnVector;
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::Mark;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

impl<T: Mark> Chart<T> {
    /// Transforms data by calculating new columns using native Rust closures.
    ///
    /// It adds two new columns (ymin and ymax) to the dataset with high-performance, parallelized
    /// mapping logic.
    ///
    /// # Parameters
    /// * `ymin_name` - The name for the new minimum y-value column.
    /// * `ymax_name` - The name for the new maximum y-value column.
    /// * `f` - A closure that defines the calculation logic for a single row.
    ///         It receives the row index and returns a tuple of (ymin, ymax).
    pub fn transform_calculate<F>(
        mut self,
        ymin_name: &str,
        ymax_name: &str,
        f: F,
    ) -> Result<Self, ChartonError>
    where
        F: Fn(usize) -> (f64, f64) + Sync + Send,
    {
        let row_count = self.data.height();
        if row_count == 0 {
            return Ok(self);
        }

        // --- PARALLEL CALCULATION ---
        // Using Rayon's par_iter to distribute the workload across all CPU cores.
        // This is much faster than Polars for custom complex logic that
        // doesn't fit into standard SIMD kernels.
        let results: Vec<(f64, f64)> = (0..row_count).maybe_into_par_iter().map(|i| f(i)).collect();

        // Separate the interleaved results into two contiguous vectors.
        // This maintains the columnar memory layout.
        let (ymin_data, ymax_data): (Vec<f64>, Vec<f64>) = results.into_iter().unzip();

        // --- UPDATE DATASET ---
        // We add the new columns back to the existing dataset.
        // add_column will automatically validate length consistency.
        self.data
            .add_column(ymin_name, ColumnVector::F64 { data: ymin_data })?;
        self.data
            .add_column(ymax_name, ColumnVector::F64 { data: ymax_data })?;

        Ok(self)
    }
}
