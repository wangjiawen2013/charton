use crate::chart::common::Chart;
use crate::error::ChartonError;
use crate::mark::Mark;
use polars::prelude::*;

impl<T: Mark> Chart<T> {
    /// Transform data by calculating new columns using Polars expressions
    ///
    /// This method allows you to add or modify columns in the chart's dataset by applying
    /// Polars expressions to compute new values. The calculated columns can be used for
    /// determining the y-axis minimum and maximum values for plotting.
    ///
    /// # Parameters
    /// * `ymin` - A Polars expression that defines how to calculate the minimum y-value
    /// * `ymax` - A Polars expression that defines how to calculate the maximum y-value
    ///
    /// # Returns
    /// * `Result<Self, ChartonError>` - The modified chart instance or an error if the transformation fails
    ///
    /// # Type Parameters
    /// * `E1` - A type that can be converted into a Polars `Expr`
    /// * `E2` - A type that can be converted into a Polars `Expr`
    pub fn transform_calculate<E1, E2>(mut self, ymin: E1, ymax: E2) -> Result<Self, ChartonError>
    where
        E1: Into<Expr>,
        E2: Into<Expr>,
    {
        // Convert expressions
        let ymin_expr = ymin.into();
        let ymax_expr = ymax.into();

        let df = self
            .data
            .df
            .clone()
            .lazy()
            .with_columns([ymin_expr, ymax_expr])
            .collect()?;

        self.data.df = df;
        Ok(self)
    }
}
