use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, SemanticType};
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::{AHashMap, AHashSet};

impl<T: Mark> Chart<T> {
    /// Handles grouping, binning, and gap-filling for Rect/Heatmap charts.
    /// This implementation optimizes performance by using native Rust hashes instead of Polars DataFrames.
    pub(crate) fn transform_rect_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Extract Encodings ---
        // We assume these exist based on previous validation stages.
        let x_enc = self.encoding.x.as_ref().ok_or_else(|| ChartonError::Encoding("X encoding missing".into()))?;
        let y_enc = self.encoding.y.as_ref().ok_or_else(|| ChartonError::Encoding("Y encoding missing".into()))?;
        let color_enc = self.encoding.color.as_ref().ok_or_else(|| ChartonError::Encoding("Color encoding missing".into()))?;

        // --- STEP 2: Determine Physical Data Types (SemanticType) ---
        let x_col = self.data.column(&x_enc.field)?;
        let y_col = self.data.column(&y_enc.field)?;
        let color_col = self.data.column(&color_enc.field)?;

        let x_is_discrete = matches!(x_col.semantic_type(), SemanticType::Discrete);
        let y_is_discrete = matches!(y_col.semantic_type(), SemanticType::Discrete);

        // --- STEP 3: Calculate Binning Parameters for Continuous Data ---
        // If the data is continuous, we calculate the bin width and start point.
        let x_bin_params = if !x_is_discrete {
            let (min, max) = x_col.min_max();
            let n = x_enc.bins.unwrap_or(10);
            let width = if n > 1 { (max - min) / (n as f64) } else { 1.0 };
            Some((min, n, width))
        } else { None };

        let y_bin_params = if !y_is_discrete {
            let (min, max) = y_col.min_max();
            let n = y_enc.bins.unwrap_or(10);
            let width = if n > 1 { (max - min) / (n as f64) } else { 1.0 };
            Some((min, n, width))
        } else { None };

        // --- STEP 4: First Pass - Aggregation & Coordinate Discovery ---
        // We use a HashMap to aggregate color values and HashSets to track unique X/Y labels.
        let row_count = self.data.height();
        let mut lookup: AHashMap<(String, String), f64> = AHashMap::new();
        
        let mut x_labels = Vec::new();
        let mut y_labels = Vec::new();
        let mut x_seen = AHashSet::new();
        let mut y_seen = AHashSet::new();

        for i in 0..row_count {
            // Determine the "Label" for X (either a bin midpoint string or the category name)
            let x_label = match x_bin_params {
                Some((min, n, width)) => {
                    let v = x_col.get_f64(i).unwrap_or(min);
                    let bin_idx = (((v - min) / width).floor() as usize).min(n - 1);
                    let middle = min + (bin_idx as f64 + 0.5) * width;
                    middle.to_string()
                }
                None => x_col.get_as_string(i).unwrap_or_else(|| "null".into()),
            };

            // Determine the "Label" for Y
            let y_label = match y_bin_params {
                Some((min, n, width)) => {
                    let v = y_col.get_f64(i).unwrap_or(min);
                    let bin_idx = (((v - min) / width).floor() as usize).min(n - 1);
                    let middle = min + (bin_idx as f64 + 0.5) * width;
                    middle.to_string()
                }
                None => y_col.get_as_string(i).unwrap_or_else(|| "null".into()),
            };

            // Track unique coordinates to build the Cartesian grid later
            if x_seen.insert(x_label.clone()) { x_labels.push(x_label.clone()); }
            if y_seen.insert(y_label.clone()) { y_labels.push(y_label.clone()); }

            // Aggregate the color value (Heatmap intensity)
            let c_val = color_col.get_f64(i).unwrap_or(0.0);
            *lookup.entry((x_label, y_label)).or_insert(0.0) += c_val;
        }

        // --- STEP 5: Second Pass - Cartesian Product & Gap Filling ---
        // We force-create a value for every possible (X, Y) combination.
        let total_cells = x_labels.len() * y_labels.len();
        let mut final_x = Vec::with_capacity(total_cells);
        let mut final_y = Vec::with_capacity(total_cells);
        let mut final_color = Vec::with_capacity(total_cells);

        for x in &x_labels {
            for y in &y_labels {
                // If the combination doesn't exist in our lookup, fill it with 0.0
                let val = lookup.get(&(x.clone(), y.clone())).copied().unwrap_or(0.0);
                
                final_x.push(x.clone());
                final_y.push(y.clone());
                final_color.push(val);
            }
        }

        // --- STEP 6: Rebuild Dataset ---
        let mut new_ds = Dataset::new();

        // Helper to convert the string labels back to appropriate ColumnVector types
        let cast_to_vector = |labels: Vec<String>, is_discrete: bool, is_binned: bool| -> ColumnVector {
            if !is_discrete || is_binned {
                // If it was continuous/binned, parse the midpoint strings back to f64
                let data = labels.iter().map(|s| s.parse::<f64>().unwrap_or(0.0)).collect();
                ColumnVector::F64 { data }
            } else {
                // If it was truly discrete, keep it as String
                ColumnVector::String { data: labels, validity: None }
            }
        };

        new_ds.add_column(&x_enc.field, cast_to_vector(final_x, x_is_discrete, x_bin_params.is_some()))?;
        new_ds.add_column(&y_enc.field, cast_to_vector(final_y, y_is_discrete, y_bin_params.is_some()))?;
        new_ds.add_column(&color_enc.field, ColumnVector::F64 { data: final_color })?;

        self.data = new_ds;
        Ok(self)
    }
}