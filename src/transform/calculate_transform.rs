use crate::chart::Chart;
use crate::core::data::{ColumnVector, RowAccessor};
use crate::core::utils::IntoParallelizable;
use crate::error::ChartonError;
use crate::mark::Mark;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

impl<T: Mark> Chart<T> {
    /// Adds a new calculated column to the dataset.
    ///
    /// # Parameters
    /// * `as_name`: The name of the new column.
    /// * `f`: A closure mapping a row to an optional value.
    ///        Returning `None` results in an `f64::NAN` (Null).
    /// # Example
    /// ```rust,ignore
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
        if row_count == 0 {
            return Ok(self);
        }

        let ds_ref = &self.data;

        // Optimized parallel mapping: Option<f64> -> f64 (with NaN as Null)
        let new_data: Vec<f64> = (0..row_count)
            .maybe_into_par_iter()
            .map(|i| f(RowAccessor::new(ds_ref, i)).unwrap_or(f64::NAN))
            .collect();

        self.data
            .add_column(as_name, ColumnVector::F64 { data: new_data })?;
        Ok(self)
    }
}
