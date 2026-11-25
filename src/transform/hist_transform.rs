use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, MISSING_CATEGORY};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::transform::bin_transform::BinEdges;
use ahash::AHashMap;

impl<T: Mark> Chart<T> {
    /// Handle grouping and aggregation of data for histogram chart.
    /// Uses explicit unique_values() for color to maintain consistent appearance order.
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

        // One binning convention for the whole engine; see `BinEdges`.
        let edges = BinEdges::new(x_col, x_enc.bins.unwrap_or(10));
        let n_bins = edges.map_or(0, |edges| edges.count());

        // Pre-calculate bin midpoints (Natural numeric order for X-axis)
        let bin_middles: Vec<f64> = (0..n_bins)
            .map(|index| edges.map_or(0.0, |edges| edges.center_of(index)))
            .collect();

        // --- STEP 3: Establish Deterministic Order for Color ---
        let color_list: Vec<String> = if let Some(c_enc) = color_enc {
            self.data.column(&c_enc.field)?.labels_with_missing()
        } else {
            vec![format!("{}_default", TEMP_SUFFIX)]
        };

        // --- STEP 4: Aggregate Counts (The "Group By" phase) ---
        let mut lookup: AHashMap<(usize, String), f64> = AHashMap::new();
        let row_count = self.data.height();

        for i in 0..row_count {
            // Missing values in the binned value or the colour group drop the
            // row (Vega-Lite/ggplot2).
            if x_col.is_null(i) {
                continue;
            }
            let Some(edges) = edges else {
                break;
            };
            let value = x_col.get(i).to_f64().unwrap_or(edges.start_of(0));
            let bin_idx = edges.index_of(value);

            // Colour is non-positional: a missing value is kept as the reserved
            // grey "NA" series rather than dropping a valid observation.
            let color_label = if let Some(c_enc) = color_enc {
                self.data
                    .get(&c_enc.field, i)
                    .to_string()
                    .unwrap_or_else(|| MISSING_CATEGORY.to_string())
            } else {
                format!("{}_default", TEMP_SUFFIX)
            };

            *lookup.entry((bin_idx, color_label)).or_insert(0.0) += 1.0;
        }

        // --- STEP 5: Apply Normalization (Optional) ---
        if y_enc.normalize {
            if color_enc.is_some() {
                let mut color_sums = AHashMap::new();
                for ((_, color), count) in &lookup {
                    *color_sums.entry(color.clone()).or_insert(0.0) += *count;
                }
                for ((_, color), count) in lookup.iter_mut() {
                    let total = color_sums.get(color).copied().unwrap_or(1.0);
                    if total > 0.0 {
                        *count /= total;
                    }
                }
            } else {
                let total: f64 = lookup.values().sum();
                if total > 0.0 {
                    for count in lookup.values_mut() {
                        *count /= total;
                    }
                }
            }
        }

        // --- STEP 6: Cartesian Product & Gap Filling ---
        let mut final_x = Vec::new();
        let mut final_y = Vec::new();
        let mut final_color = Vec::new();

        for (bin_idx, &mid) in bin_middles.iter().enumerate().take(n_bins) {
            for color in &color_list {
                let count = lookup
                    .get(&(bin_idx, color.clone()))
                    .copied()
                    .unwrap_or(0.0);

                final_x.push(mid);
                final_y.push(count);

                if color_enc.is_some() {
                    final_color.push(color.clone());
                }
            }
        }

        // --- STEP 7: Rebuild Dataset ---
        let mut new_ds = Dataset::new();

        // Updated to use Float64 variant with validity: None
        // (Calculated bins and counts are guaranteed to be present)
        new_ds.add_column(
            bin_field,
            ColumnVector::Float64 {
                data: final_x,
                validity: None,
            },
        )?;
        new_ds.add_column(
            count_field,
            ColumnVector::Float64 {
                data: final_y,
                validity: None,
            },
        )?;

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
