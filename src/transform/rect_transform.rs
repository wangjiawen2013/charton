use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, SemanticType};
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::{AHashMap, AHashSet};

impl<T: Mark> Chart<T> {
    /// Handles grouping, binning, and aggregation for Rect/Heatmap marks.
    ///
    /// Key Behaviors:
    /// 1. **Sparse Data**: Does NOT fill missing (X, Y) combinations with 0.
    ///    Only coordinates present in the source data are generated.
    /// 2. **Stable Order**: Uses the dataset's internal discovery logic to ensure
    ///    categorical axes respect the data's natural order.
    /// 3. **Type Safety**: Binned continuous data is cast back to F64 for proper scaling.
    pub(crate) fn transform_rect_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Extract Encodings ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X encoding missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y encoding missing".into()))?;
        let color_enc = self
            .encoding
            .color
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Color encoding missing".into()))?;

        // --- STEP 2: Access Source Columns ---
        let x_col = self.data.column(&x_enc.field)?;
        let y_col = self.data.column(&y_enc.field)?;
        let color_col = self.data.column(&color_enc.field)?;

        let x_is_discrete = matches!(x_col.semantic_type(), SemanticType::Discrete);
        let y_is_discrete = matches!(y_col.semantic_type(), SemanticType::Discrete);

        // --- STEP 3: Calculate Binning Parameters (Only for Continuous) ---
        let x_bin_params = if !x_is_discrete {
            let (min, max) = x_col.min_max();
            let n = x_enc.bins.unwrap_or(10);
            let width = if n > 1 { (max - min) / (n as f64) } else { 1.0 };
            Some((min, n, width))
        } else {
            None
        };

        let y_bin_params = if !y_is_discrete {
            let (min, max) = y_col.min_max();
            let n = y_enc.bins.unwrap_or(10);
            let width = if n > 1 { (max - min) / (n as f64) } else { 1.0 };
            Some((min, n, width))
        } else {
            None
        };

        // --- STEP 4: Aggregation Pass ---
        let row_count = self.data.height();
        // Storage for aggregated sums: Map<(X_Label, Y_Label), Summed_Value>
        let mut lookup: AHashMap<(String, String), f64> = AHashMap::new();
        // Use a Vec to track the order of appearance for unique (X, Y) pairs
        let mut appearance_order = Vec::new();
        let mut seen_coords = AHashSet::new();

        for i in 0..row_count {
            // Determine X identifier
            let x_key = match x_bin_params {
                Some((min, n, width)) => {
                    let v = x_col.get_f64(i).unwrap_or(min);
                    let bin_idx = (((v - min) / width).floor() as usize).min(n - 1);
                    (min + (bin_idx as f64 + 0.5) * width).to_string()
                }
                None => x_col.get_str_or(i, "null"),
            };

            // Determine Y identifier
            let y_key = match y_bin_params {
                Some((min, n, width)) => {
                    let v = y_col.get_f64(i).unwrap_or(min);
                    let bin_idx = (((v - min) / width).floor() as usize).min(n - 1);
                    (min + (bin_idx as f64 + 0.5) * width).to_string()
                }
                None => y_col.get_str_or(i, "null"),
            };

            let coord = (x_key, y_key);
            let val = color_col.get_f64(i).unwrap_or(0.0);

            // Record this pair if it's the first time we see it
            if seen_coords.insert(coord.clone()) {
                appearance_order.push(coord.clone());
            }

            // Aggregate (Sum) values sharing the same cell
            *lookup.entry(coord).or_insert(0.0) += val;
        }

        // --- STEP 5: Reconstruct the Dataset (Sparse Implementation) ---
        let mut final_x = Vec::with_capacity(appearance_order.len());
        let mut final_y = Vec::with_capacity(appearance_order.len());
        let mut final_color = Vec::with_capacity(appearance_order.len());

        // Iterate through appearance_order instead of a Cartesian Product.
        // This ensures:
        // 1. Missing data points are truly skipped (not rendered as 0).
        // 2. The resultant Dataset rows follow the input data's temporal/logical order.
        for coord in appearance_order {
            if let Some(&val) = lookup.get(&coord) {
                final_x.push(coord.0);
                final_y.push(coord.1);
                final_color.push(val);
            }
        }

        let mut new_ds = Dataset::new();

        // Helper to convert internal string keys back to native types
        let cast_vec = |labels: Vec<String>, is_discrete: bool, binned: bool| -> ColumnVector {
            if !is_discrete || binned {
                // If numeric, parse back to F64 for the renderer/scales
                let data = labels
                    .iter()
                    .map(|s| s.parse::<f64>().unwrap_or(0.0))
                    .collect();
                ColumnVector::F64 { data }
            } else {
                // If categorical, keep as String
                ColumnVector::String {
                    data: labels,
                    validity: None,
                }
            }
        };

        new_ds.add_column(
            &x_enc.field,
            cast_vec(final_x, x_is_discrete, x_bin_params.is_some()),
        )?;
        new_ds.add_column(
            &y_enc.field,
            cast_vec(final_y, y_is_discrete, y_bin_params.is_some()),
        )?;
        new_ds.add_column(&color_enc.field, ColumnVector::F64 { data: final_color })?;

        self.data = new_ds;
        Ok(self)
    }
}
