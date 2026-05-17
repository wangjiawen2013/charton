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
    ///   Returning `None` will be treated as a Null value (validity = 0).
    pub fn transform_calculate<F>(mut self, as_name: &str, f: F) -> Result<Self, ChartonError>
    where
        F: Fn(RowAccessor) -> Option<f64> + Sync + Send,
    {
        let row_count = self.data.height();
        if row_count == 0 {
            return Ok(self);
        }

        let ds_ref = &self.data;

        // Step 1: Compute values in parallel
        // We collect into Option<f64> first to maintain thread safety via RowAccessor
        let results: Vec<Option<f64>> = (0..row_count)
            .maybe_into_par_iter()
            .map(|i| f(RowAccessor::new(ds_ref, i)))
            .collect();

        // Step 2: Flatten into physical data and a u8 validity mask
        let mut data = Vec::with_capacity(row_count);
        let mut validity_mask = Vec::with_capacity(row_count);
        let mut has_nulls = false;

        for res in results {
            match res {
                Some(v) => {
                    data.push(v);
                    validity_mask.push(1u8); // 1 = Valid
                }
                None => {
                    // Use NaN as a placeholder in the data vector
                    data.push(f64::NAN);
                    validity_mask.push(0u8); // 0 = Null/Invalid
                    has_nulls = true;
                }
            }
        }

        // Step 3: Insert into the dataset using the Float64 variant
        self.data.add_column(
            as_name,
            ColumnVector::Float64 {
                data,
                // Only allocate/store the mask if nulls actually exist
                validity: if has_nulls { Some(validity_mask) } else { None },
            },
        )?;

        Ok(self)
    }
}
