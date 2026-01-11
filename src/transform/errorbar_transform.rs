use crate::chart::Chart;
use crate::error::ChartonError;
use crate::mark::Mark;
use polars::prelude::*;

impl<T: Mark> Chart<T> {
    // Handle grouping and aggregation of data for errorbar chart
    pub(crate) fn transform_errorbar_data(mut self) -> Result<Self, ChartonError> {
        // Get encodings - we know these exist based on earlier validation
        let x_encoding = self.encoding.x.as_ref().unwrap();
        let y_encoding = self.encoding.y.as_ref().unwrap();

        // Create column names following the pattern: original_fieldname_min and original_fieldname_max
        let y_min_col = format!("__charton_temp_{}_min", y_encoding.field);
        let y_max_col = format!("__charton_temp_{}_max", y_encoding.field);

        // Group by x values and calculate mean, std, then create ymin and ymax columns
        self.data.df = self
            .data
            .df
            .clone()
            .lazy()
            .group_by_stable([col(&x_encoding.field)])
            .agg([
                col(&y_encoding.field).mean().alias(&y_encoding.field), // Mean values keep original y field name
                (col(&y_encoding.field).mean() - col(&y_encoding.field).std(1)).alias(&y_min_col),
                (col(&y_encoding.field).mean() + col(&y_encoding.field).std(1)).alias(&y_max_col),
            ])
            .collect()?;

        Ok(self)
    }
}
