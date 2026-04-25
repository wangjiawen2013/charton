use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::ColumnVector;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::scale::Scale;
use ahash::AHashMap;

impl<T: Mark> Chart<T> {
    /// Transforms point data for categorical axes by calculating sub-grouping (Dodge)
    /// and density-based horizontal distribution (Beeswarm).
    ///
    /// The transformation follows a two-step positioning logic:
    /// 1. Dodge: Assigns points to discrete sub-slots based on the color encoding.
    /// 2. Beeswarm: Assigns a relative 'rank' to points that collide vertically.
    pub(crate) fn transform_point_data(mut self) -> Result<Self, ChartonError> {
        // --- 1. PRE-FLIGHT CHECKS ---
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

        // Transformation is only triggered if 'color' is provided (for dodging/grouping)
        let color_field = match &self.encoding.color {
            Some(c) => &c.field,
            None => return Ok(self),
        };

        // Only applicable for Categorical (Discrete) X-axes.
        let x_scale_type = x_enc.scale_type.as_ref().ok_or_else(|| {
            ChartonError::Internal("Scale type must be resolved before transformation".into())
        })?;

        if matches!(x_scale_type, Scale::Linear | Scale::Log | Scale::Temporal) {
            return Ok(self);
        }

        let row_count = self.data.height();
        let x_col = self.data.column(&x_enc.field)?;
        let y_col = self.data.column(&y_enc.field)?;

        // --- 2. LOCAL DENSITY HEURISTIC ---
        // Since we don't have the final global Y-scale yet, we use a local
        // heuristic (2% of the column range) to detect vertical collisions.
        let (l_min, l_max) = y_col.min_max();
        let local_span = (l_max - l_min).abs().max(1e-12);
        let threshold = local_span * 0.02;

        // --- 3. GROUPING LOGIC ---
        // We group row indices by both X-category and Color-category.
        // This ensures that 'Dodge' slots are isolated from each other.
        let mut group_map: AHashMap<(String, String), Vec<usize>> = AHashMap::new();
        for i in 0..row_count {
            let x_val = x_col.get_str_or(i, "null");
            let c_val = self.data.get_str_or(color_field, i, "null");
            group_map.entry((x_val, c_val)).or_default().push(i);
        }

        // --- 4. RELATIVE SWARM RANK CALCULATION ---
        // Instead of calculating pixel offsets, we calculate an integer 'rank'.
        // Rank 0 = Center, Rank 1 = Right, Rank -1 = Left, etc.
        let mut swarm_ranks = vec![0.0; row_count];
        for (_, indices) in group_map.iter() {
            if indices.len() <= 1 {
                continue;
            }

            // Sort by Y for stable, sequential collision detection
            let mut sorted_group = indices.clone();
            sorted_group.sort_by(|&a, &b| {
                y_col
                    .get_f64(a)
                    .partial_cmp(&y_col.get_f64(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut last_y = -f64::INFINITY;
            let mut collision_stack = 0;

            for &idx in &sorted_group {
                let curr_y = y_col.get_f64(idx).unwrap_or(0.0);

                if (curr_y - last_y).abs() < threshold {
                    collision_stack += 1;
                } else {
                    collision_stack = 0;
                }

                // Determine horizontal displacement direction
                // local_idx 0 -> 0.0
                // local_idx 1 -> 1.0
                // local_idx 2 -> -1.0
                // local_idx 3 -> 2.0 ...
                let direction = if collision_stack % 2 == 0 { 1.0 } else { -1.0 };
                let magnitude = (collision_stack as f64 / 2.0).ceil();

                swarm_ranks[idx] = direction * magnitude;
                last_y = curr_y;
            }
        }

        // --- 5. DATA ASSEMBLY & ATOMIC REORDERING ---
        let mut sorted_indices = Vec::with_capacity(row_count);
        let mut final_sub_idx = Vec::with_capacity(row_count);
        let mut final_groups_count = Vec::with_capacity(row_count);
        let mut final_swarm = Vec::with_capacity(row_count);

        let x_uniques = x_col.unique_values();
        let c_uniques = self.data.column(color_field)?.unique_values();
        let total_groups = c_uniques.len() as f64;

        for x_val in &x_uniques {
            for (c_idx, c_val) in c_uniques.iter().enumerate() {
                if let Some(indices) = group_map.get(&(x_val.clone(), c_val.clone())) {
                    for &idx in indices {
                        sorted_indices.push(idx);
                        final_sub_idx.push(c_idx as f64);
                        final_groups_count.push(total_groups);
                        final_swarm.push(swarm_ranks[idx]);
                    }
                }
            }
        }

        // Use take_rows to perform a type-safe, null-safe reordering of all columns
        let mut new_ds = self.data.take_rows(&sorted_indices)?;

        // Append the calculated layout metadata as temporary columns for the renderer
        new_ds.add_column(
            format!("{}_sub_idx", TEMP_SUFFIX),
            ColumnVector::F64 {
                data: final_sub_idx,
            },
        )?;
        new_ds.add_column(
            format!("{}_groups_count", TEMP_SUFFIX),
            ColumnVector::F64 {
                data: final_groups_count,
            },
        )?;
        new_ds.add_column(
            format!("{}_swarm_rank", TEMP_SUFFIX),
            ColumnVector::F64 { data: final_swarm },
        )?;

        self.data = new_ds;
        Ok(self)
    }
}
