use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::{AHashMap, AHashSet};

impl<T: Mark> Chart<T> {
    /// Handle grouping and aggregation of data for histogram chart without Polars dependency.
    pub(crate) fn transform_histogram_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Extract Encodings ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y missing".into()))?;
        let color_enc = self.encoding.color.as_ref();

        let bin_field = &x_enc.field;
        let count_field = &y_enc.field;

        // --- STEP 2: Calculate Binning Parameters ---
        let x_col = self.data.column(bin_field)?;
        let (min_val, max_val) = x_col.min_max();
        let n_bins = x_enc.bins.unwrap_or(10);

        let bin_width = if n_bins > 1 {
            (max_val - min_val) / (n_bins as f64)
        } else {
            1.0
        };

        // Pre-calculate bin midpoints (Labels for the X-axis)
        let bin_middles: Vec<f64> = (0..n_bins)
            .map(|i| min_val + (i as f64 + 0.5) * bin_width)
            .collect();

        // --- STEP 3: Aggregate Counts (The "Group By" phase) ---
        // Key: (bin_index, color_label), Value: count
        let mut lookup: AHashMap<(usize, String), f64> = AHashMap::new();
        let mut color_values = AHashSet::new();
        let row_count = self.data.height();

        for i in 0..row_count {
            let val = x_col.get_f64(i).unwrap_or(min_val);
            // Calculate which bin this value falls into
            let bin_idx = (((val - min_val) / bin_width).floor() as usize).min(n_bins - 1);

            let color_label = if let Some(c_enc) = color_enc {
                let label = self.data.get_str_or(&c_enc.field, i, "null");
                color_values.insert(label.clone());
                label
            } else {
                "__default__".to_string()
            };

            *lookup.entry((bin_idx, color_label)).or_insert(0.0) += 1.0;
        }

        // --- STEP 4: Apply Normalization (Optional) ---
        if y_enc.normalize {
            if let Some(_c_enc) = color_enc {
                // Normalize within each color group: sum(counts per color) = 1.0
                let mut color_sums = AHashMap::new();
                for ((_, color), count) in &lookup {
                    *color_sums.entry(color.clone()).or_insert(0.0) += *count;
                }
                for ((_, color), count) in lookup.iter_mut() {
                    let total = color_sums.get(color).copied().unwrap_or(1.0);
                    *count /= total;
                }
            } else {
                // Global normalization: sum(all counts) = 1.0
                let total: f64 = lookup.values().sum();
                if total > 0.0 {
                    for count in lookup.values_mut() {
                        *count /= total;
                    }
                }
            }
        }

        // --- STEP 5: Cartesian Product & Gap Filling ---
        // Ensure every bin exists for every color, even if count is 0
        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        let mut final_color = Vec::new();

        let color_list: Vec<String> = if color_enc.is_some() {
            color_values.into_iter().collect()
        } else {
            vec!["__default__".to_string()]
        };

        for bin_idx in 0..n_bins {
            for color in &color_list {
                let count = lookup
                    .get(&(bin_idx, color.clone()))
                    .copied()
                    .unwrap_or(0.0);

                final_x.push(bin_middles[bin_idx]);
                final_y.push(count);
                if color_enc.is_some() {
                    final_color.push(color.clone());
                }
            }
        }

        // --- STEP 6: Rebuild Dataset ---
        let mut new_ds = Dataset::new();
        new_ds.add_column(bin_field, ColumnVector::F64 { data: final_x })?;
        new_ds.add_column(count_field, ColumnVector::F64 { data: final_y })?;

        if let Some(c_enc) = color_enc {
            new_ds.add_column(
                &c_enc.field,
                ColumnVector::String {
                    data: final_color,
                    validity: None,
                },
            )?;
        }

        self.data = new_ds;
        Ok(self)
    }
}
