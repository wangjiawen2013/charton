use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::core::data::ColumnVector;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::scale::Scale;
use ahash::AHashMap;

impl<T: Mark> Chart<T> {
    /// Transforms the point data to support categorical layouts such as Dodging and Beeswarm.
    ///
    /// ### Logic Overview:
    /// 1. **Early Exit for Continuous Scales**: If the X-axis is a numerical or temporal scale
    ///    (Linear, Log, Temporal), the raw coordinates are preserved to maintain mathematical precision.
    /// 2. **Aesthetic Grouping**: If a `color` encoding is present (and differs from the X field),
    ///    the points are assigned to discrete "slots" (lanes) within each X-category.
    /// 3. **Layout Helper Injections**:
    ///     * `sub_idx`: The zero-indexed slot position for the point's group.
    ///     * `groups_count`: The total number of groups at that X-position (used for width normalization).
    ///     * `swarm_local_idx`: A sequential counter for points sharing the same logic-space,
    ///        serving as the processing order for Quadtree-based collision resolution.
    pub(crate) fn transform_point_data(mut self) -> Result<Self, ChartonError> {
        // --- STEP 1: Pre-flight Validation ---
        let x_enc = self.encoding.x.as_ref().ok_or_else(|| {
            ChartonError::Encoding("X encoding is required for transformation".into())
        })?;

        let x_field = &x_enc.field;

        // Transformation logic requires resolved scales to distinguish between Discrete and Continuous axes.
        let x_scale_type = x_enc.scale_type.as_ref().ok_or_else(|| {
            ChartonError::Internal(
                "Scale type must be resolved prior to data transformation".into(),
            )
        })?;

        // Early Exit: Continuous axes do not support dodging as they rely on exact coordinate mapping.
        if matches!(x_scale_type, Scale::Linear | Scale::Log | Scale::Temporal) {
            return Ok(self);
        }

        // --- STEP 2: Grouping Context Identification ---
        // Grouping/Dodging is typically triggered by a secondary aesthetic (usually Color).
        let color_field = match &self.encoding.color {
            Some(c) => &c.field,
            None => return Ok(self), // No grouping aesthetic; use default single-column layout.
        };

        // If the color field is identical to the X field, it's a 1-to-1 mapping (no dodging needed).
        if color_field == x_field {
            return Ok(self);
        }

        // --- STEP 3: Categorical Indexing ---
        let x_col = self.data.column(x_field)?;
        let color_col = self.data.column(color_field)?;

        // Determine unique groups to establish deterministic slot ordering.
        let color_uniques = color_col.unique_values();
        let color_map: AHashMap<String, usize> = color_uniques
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();

        let row_count = self.data.height();
        let total_groups = color_uniques.len() as f64;

        // Pre-allocate vectors for columnar injection.
        let mut final_sub_idx = Vec::with_capacity(row_count);
        let mut final_groups_count = Vec::with_capacity(row_count);
        let mut final_swarm_idx = Vec::with_capacity(row_count);

        // Counter to track point density within a specific (Category, Color) intersection.
        let mut group_counters: AHashMap<(String, String), usize> = AHashMap::new();

        // --- STEP 4: Iterative Metadata Generation ---
        for i in 0..row_count {
            let x_val = x_col.get_str_or(i, "null");
            let c_val = color_col.get_str_or(i, "null");

            // A. sub_idx: Maps the point to its specific dodge-lane.
            let c_idx = *color_map.get(&c_val).unwrap_or(&0);
            final_sub_idx.push(c_idx as f64);

            // B. groups_count: Defines the divisor for calculating lane widths in the renderer.
            final_groups_count.push(total_groups);

            // C. swarm_local_idx: Provides the sequence ID for Force-Directed Beeswarm layouts.
            // Using Entry API for efficient local group counting.
            let b_idx = group_counters.entry((x_val, c_val)).or_insert(0);
            final_swarm_idx.push(*b_idx as f64);
            *b_idx += 1;
        }

        // --- STEP 5: Dataset Augmentation ---
        // Injected temporary columns allow the MarkRenderer to remain stateless and parallelizable.
        self.data.add_column(
            format!("{}_sub_idx", TEMP_SUFFIX),
            ColumnVector::F64 {
                data: final_sub_idx,
            },
        )?;
        self.data.add_column(
            format!("{}_groups_count", TEMP_SUFFIX),
            ColumnVector::F64 {
                data: final_groups_count,
            },
        )?;
        self.data.add_column(
            format!("{}_swarm_local_idx", TEMP_SUFFIX),
            ColumnVector::F64 {
                data: final_swarm_idx,
            },
        )?;

        Ok(self)
    }
}
