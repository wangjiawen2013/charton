use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::scale::Scale;
use crate::transform::bin_transform::BinEdges;
use ahash::{AHashMap, AHashSet};

impl<T: Mark> Chart<T> {
    /// Handles grouping, binning, and aggregation for Rect/Heatmap marks.
    ///
    /// Key Behaviors:
    /// 1. **Sparse Data**: Does NOT fill missing (X, Y) combinations with 0.
    ///    Only coordinates present in the source data are generated.
    /// 2. **Stable Order**: Uses the dataset's internal discovery logic to ensure
    ///    categorical axes respect the data's natural order.
    /// 3. **Type Safety**: Binned continuous data is cast back to Float64 for proper scaling.
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

        // Determine if axes are discrete to decide between Categorical grouping or Binning
        let x_is_discrete = matches!(x_enc.scale_type.as_ref().unwrap(), Scale::Discrete);
        let y_is_discrete = matches!(y_enc.scale_type.as_ref().unwrap(), Scale::Discrete);

        // --- STEP 3: Calculate Binning Parameters (Only for Continuous axes) ---
        // One binning convention for the whole engine; see `BinEdges`.
        let x_edges = if x_is_discrete {
            None
        } else {
            BinEdges::new(x_col, x_enc.bins.unwrap_or(10))
        };
        let y_edges = if y_is_discrete {
            None
        } else {
            BinEdges::new(y_col, y_enc.bins.unwrap_or(10))
        };

        // --- STEP 4: Grouping Pass ---
        let row_count = self.data.height();
        let mut groups: AHashMap<(String, String), Vec<usize>> = AHashMap::new();
        let mut appearance_order = Vec::new();
        let mut seen_coords = AHashSet::new();

        for i in 0..row_count {
            // A missing x or y coordinate drops the cell (Vega-Lite/ggplot2),
            // rather than landing in the first bin or a "null" level.
            if x_col.is_null(i) || y_col.is_null(i) {
                continue;
            }
            // Resolve X coordinate identifier
            let x_key = match x_edges {
                Some(edges) => {
                    let value = x_col.get(i).to_f64().unwrap_or(edges.start_of(0));
                    edges.center_of(edges.index_of(value)).to_string()
                }
                None => x_col
                    .get(i)
                    .to_string()
                    .unwrap_or_else(|| "null".to_string()),
            };

            // Resolve Y coordinate identifier
            let y_key = match y_edges {
                Some(edges) => {
                    let value = y_col.get(i).to_f64().unwrap_or(edges.start_of(0));
                    edges.center_of(edges.index_of(value)).to_string()
                }
                None => y_col
                    .get(i)
                    .to_string()
                    .unwrap_or_else(|| "null".to_string()),
            };

            let coord = (x_key, y_key);

            // Track unique coordinates in order of discovery for stable rendering
            if seen_coords.insert(coord.clone()) {
                appearance_order.push(coord.clone());
            }

            // Collect the row index for later aggregation
            groups.entry(coord).or_default().push(i);
        }

        // --- STEP 5: Aggregation Pass ---
        let mut final_x = Vec::with_capacity(appearance_order.len());
        let mut final_y = Vec::with_capacity(appearance_order.len());
        let mut final_color = Vec::with_capacity(appearance_order.len());

        let agg_op = color_enc.aggregate;

        for coord in appearance_order {
            if let Some(indices) = groups.get(&coord) {
                let aggregated_val = agg_op.aggregate_by_index(color_col, indices);

                final_x.push(coord.0);
                final_y.push(coord.1);
                final_color.push(aggregated_val);
            }
        }

        // --- STEP 6: Reconstruct the Dataset ---
        let mut new_ds = Dataset::new();

        // Internal helper to cast string keys back to Float64 for numeric/binned scales
        let cast_vec = |labels: Vec<String>, is_discrete: bool, binned: bool| -> ColumnVector {
            if !is_discrete || binned {
                let data = labels
                    .iter()
                    .map(|s| s.parse::<f64>().unwrap_or(0.0))
                    .collect();
                // Migrated to Float64 variant with validity: None
                ColumnVector::Float64 {
                    data,
                    validity: None,
                }
            } else {
                ColumnVector::String {
                    data: labels,
                    validity: None,
                }
            }
        };

        new_ds.add_column(
            &x_enc.field,
            cast_vec(final_x, x_is_discrete, x_edges.is_some()),
        )?;
        new_ds.add_column(
            &y_enc.field,
            cast_vec(final_y, y_is_discrete, y_edges.is_some()),
        )?;

        // The color column is always numeric (f64) after aggregation.
        // Migrated to Float64 variant.
        new_ds.add_column(
            &color_enc.field,
            ColumnVector::Float64 {
                data: final_color,
                validity: None,
            },
        )?;

        self.data = new_ds;
        Ok(self)
    }
}
