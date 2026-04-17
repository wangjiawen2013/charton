use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;
use ahash::AHashMap;

/// Window-specific operations for computing window functions
///
/// These operations are used in window transformations to calculate
/// various statistics and rankings within sliding windows of data.
/// They correspond to window functions commonly found in SQL and data analysis.
#[derive(Debug, Clone)]
pub enum WindowOnlyOp {
    // Window-specific operations(see https://altair-viz.github.io/user_guide/generated/core/altair.WindowFieldDef.html#altair.WindowFieldDef)
    RowNumber,
    /// Assigns a rank to each data object based on its position in the sorted order
    /// Tied values receive the same rank, but the next rank is skipped
    Rank,
    /// Assigns a rank to each data object based on its position in the sorted order
    /// Tied values receive the same rank, and the next rank continues sequentially
    DenseRank,
    /// Calculates the relative rank of each data object as a percentage in the sorted order
    PercentRank,
    /// Calculates the cumulative distribution of data objects within a group
    CumeDist,
    /// Divides data objects into N buckets based on their sorted order
    Ntile(u32), // With parameter
    /// Returns the value of the data object that is at a specified offset prior to the current object
    Lag(Option<u32>), // With optional parameter
    /// Returns the value of the data object that is at a specified offset after the current object
    Lead(Option<u32>), // With optional parameter
    /// Returns the first value in the window frame
    FirstValue,
    /// Returns the last value in the window frame
    LastValue,
    /// Returns the value of the nth data object in the window frame
    NthValue(u32), // With parameter
}

impl WindowOnlyOp {
    fn as_str(&self) -> &'static str {
        match self {
            WindowOnlyOp::RowNumber => "row_number",
            WindowOnlyOp::Rank => "rank",
            WindowOnlyOp::DenseRank => "dense_rank",
            WindowOnlyOp::PercentRank => "percent_rank",
            WindowOnlyOp::CumeDist => "cume_dist",
            WindowOnlyOp::Ntile(_) => "ntile",
            WindowOnlyOp::Lag(_) => "lag",
            WindowOnlyOp::Lead(_) => "lead",
            WindowOnlyOp::FirstValue => "first_value",
            WindowOnlyOp::LastValue => "last_value",
            WindowOnlyOp::NthValue(_) => "nth_value",
        }
    }
}

impl std::fmt::Display for WindowOnlyOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Definition of a window field operation
///
/// This struct specifies which field to operate on, what window operation to apply,
/// and what to name the resulting column.
#[derive(Debug, Clone)]
pub struct WindowFieldDef {
    /// The data field for which to compute the window function
    pub field: String,
    /// The window operation to apply
    pub op: WindowOnlyOp,
    /// The output name for the window operation
    pub as_: String,
}

impl WindowFieldDef {
    /// Creates a new `WindowFieldDef` instance
    ///
    /// # Parameters
    /// * `field` - The name of the data field to apply the window operation to
    /// * `op` - The window operation to apply
    /// * `as_` - The name for the output column that will contain the result of the window operation
    ///
    /// # Returns
    /// A new `WindowFieldDef` instance with the specified parameters
    ///
    /// # Example
    /// ```rust,ignore
    /// let window_field = WindowFieldDef::new("sales", WindowOnlyOp::Rank, "sales_rank");
    /// ```
    pub fn new(field: &str, op: WindowOnlyOp, as_: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            op,
            as_: as_.into(),
        }
    }
}

/// Configuration parameters for window transformation
///
/// This struct encapsulates all the settings needed to perform window operations
/// on data, including the window field definition, frame specification, grouping,
/// and various options for output formatting.
#[derive(Debug, Clone)]
pub struct WindowTransform {
    /// The definition of the fields in the window, and what calculations to use
    pub window: WindowFieldDef,
    /// A frame specification as a two-element array indicating how the sliding window should proceed
    pub frame: [Option<f64>; 2],
    /// The data fields for partitioning the data objects into separate windows
    pub groupby: Option<String>,
    /// Indicates if the sliding window frame should ignore peer values
    pub ignore_peers: bool,
    /// If true, normalize the cumulative frequency to the range [0,1] in each group
    pub normalize: bool,
}

impl WindowTransform {
    /// Create a new WindowTransform with the specified window operation
    ///
    /// # Parameters
    /// * `window` - The window field definition specifying the field, operation, and output name
    ///
    /// # Returns
    /// A new `WindowTransform` instance with default settings:
    /// - Frame: [None, Some(0.0)] (unbounded preceding to current row)
    /// - No grouping
    /// - ignore_peers: false
    /// - normalize: false
    ///
    /// # Example
    /// ```rust,ignore
    /// let window_field = WindowFieldDef::new("value", WindowOnlyOp::Rank, "value_rank");
    /// let window_transform = WindowTransform::new(window_field);
    /// ```
    pub fn new(window: WindowFieldDef) -> Self {
        Self {
            window,
            frame: [None, Some(0.0)], // Default value: [null, 0]
            groupby: None,
            ignore_peers: false,
            normalize: false,
        }
    }

    /// Set the frame specification
    ///
    /// # Parameters
    /// * `frame` - A two-element array where the first element is the lower bound and
    ///   the second element is the upper bound of the window frame
    ///
    /// # Returns
    /// The modified `WindowTransform` instance with the updated frame setting
    ///
    /// # Example
    /// ```rust,ignore
    /// let window_transform = window_transform.with_frame([Some(-5.0), Some(5.0)]); // Window includes 5 rows before and after
    /// ```
    pub fn with_frame(mut self, frame: [Option<f64>; 2]) -> Self {
        self.frame = frame;
        self
    }

    /// Set the groupby field
    ///
    /// # Parameters
    /// * `groupby` - The name of the column to group by, with separate window calculations for each group
    ///
    /// # Returns
    /// The modified `WindowTransform` instance with the updated groupby setting
    ///
    /// # Example
    /// ```rust,ignore
    /// let window_transform = window_transform.with_groupby("category");
    /// ```
    pub fn with_groupby(mut self, groupby: &str) -> Self {
        self.groupby = Some(groupby.into());
        self
    }

    /// Set the ignore_peers flag
    ///
    /// # Parameters
    /// * `ignore_peers` - If true, the window frame will ignore peer values (values that are equal when sorted)
    ///
    /// # Returns
    /// The modified `WindowTransform` instance with the updated ignore_peers setting
    ///
    /// # Example
    /// ```rust,ignore
    /// let window_transform = window_transform.with_ignore_peers(true);
    /// ```
    pub fn with_ignore_peers(mut self, ignore_peers: bool) -> Self {
        self.ignore_peers = ignore_peers;
        self
    }

    /// Set the normalize flag
    ///
    /// # Parameters
    /// * `normalize` - If true, normalize the cumulative frequency to the range [0,1] in each group
    ///
    /// # Returns
    /// The modified `WindowTransform` instance with the updated normalize setting
    ///
    /// # Example
    /// ```rust,ignore
    /// let window_transform = window_transform.with_normalize(true);
    /// ```
    pub fn with_normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }
}

impl<T: Mark> Chart<T> {
    /// Performs window transformations (Ranking, Row Numbers, or ECDF) on the chart data.
    ///
    /// This implementation follows the "Mainstream Strategy":
    /// 1. **Order Persistence**: Uses `unique_values()` to maintain "First Appearance" order.
    /// 2. **Null Handling**: Rows with Null/NaN inputs result in NaN outputs.
    /// 3. **ECDF Soundness**: Filters out Nulls before calculation so they don't skew the distribution.
    pub fn transform_window(mut self, params: WindowTransform) -> Result<Self, ChartonError> {
        let n = self.data.height();
        if n == 0 {
            return Ok(self);
        }

        let field_name = &params.window.field;
        let output_name = &params.window.as_;
        let target_col = self.data.column(field_name)?;

        // --- PHASE 1: UNIFIED GROUPING ---
        let mut groups: AHashMap<Option<String>, Vec<usize>> = AHashMap::new();
        if let Some(ref group_field) = params.groupby {
            let group_col = self.data.column(group_field)?;
            for i in 0..n {
                groups.entry(group_col.get_str(i)).or_default().push(i);
            }
        } else {
            // If no groupby is specified, treat the entire dataset as a single group
            groups.insert(None, (0..n).collect());
        }

        // --- PHASE 2: STABLE ORDER DETERMINATION ---
        // We derive the iteration order from unique_values() to stay consistent
        // with the "First Appearance" logic used for visual scales/legends.
        let group_order: Vec<Option<String>> = if let Some(ref group_field) = params.groupby {
            self.data
                .column(group_field)?
                .unique_values()
                .into_iter()
                .map(Some)
                .collect()
        } else {
            vec![None]
        };

        // --- PHASE 3: OPERATION MATCHING ---
        match params.window.op {
            WindowOnlyOp::CumeDist => {
                // ECDF generates a new Dataset (row count changes due to padding)
                self.data = self.apply_ecdf_with_padding(groups, group_order, &params)?;
            }
            WindowOnlyOp::RowNumber | WindowOnlyOp::Rank => {
                // Ranking operations maintain original row count.
                // Default to NaN so rows with Null inputs remain Null in output.
                let mut results = vec![f64::NAN; n];

                for key in group_order {
                    if let Some(mut indices) = groups.remove(&key) {
                        // Sort within group: Nulls Last
                        indices.sort_by(|&a, &b| {
                            let va = target_col.get_f64(a);
                            let vb = target_col.get_f64(b);
                            match (va, vb) {
                                (Some(x), Some(y)) => {
                                    x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
                                }
                                (None, Some(_)) => std::cmp::Ordering::Greater,
                                (Some(_), None) => std::cmp::Ordering::Less,
                                (None, None) => std::cmp::Ordering::Equal,
                            }
                        });

                        let mut last_val: Option<f64> = None;
                        let mut last_rank = 0;
                        let mut valid_count = 0;

                        for &idx in &indices {
                            let val = target_col.get_f64(idx);

                            // Only calculate for non-null values
                            if val.is_none() {
                                continue;
                            }

                            valid_count += 1;
                            match params.window.op {
                                WindowOnlyOp::RowNumber => {
                                    results[idx] = valid_count as f64;
                                }
                                WindowOnlyOp::Rank => {
                                    let rank = if val == last_val {
                                        last_rank
                                    } else {
                                        valid_count
                                    };
                                    results[idx] = rank as f64;
                                    last_val = val;
                                    last_rank = rank;
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }
                self.data
                    .add_column(output_name, ColumnVector::F64 { data: results })?;
            }
            _ => {
                return Err(ChartonError::Unimplemented(format!(
                    "Operation {:?} not implemented",
                    params.window.op
                )));
            }
        }

        Ok(self)
    }

    /// Internal helper to handle ECDF logic with Domain Expansion (Padding).
    ///
    /// Padding ensures the curve starts at (min, 0) and ends at (max, 1),
    /// which is essential for visual alignment in step charts.
    fn apply_ecdf_with_padding(
        &self,
        mut groups: AHashMap<Option<String>, Vec<usize>>,
        group_order: Vec<Option<String>>,
        params: &WindowTransform,
    ) -> Result<Dataset, ChartonError> {
        let field_name = &params.window.field;
        let output_name = &params.window.as_;
        let target_col = self.data.column(field_name)?;

        // --- STEP 1: CALCULATE GLOBAL BOUNDARIES (Numerical values only) ---
        let mut global_min = f64::MAX;
        let mut global_max = f64::MIN;
        let mut found_any = false;

        for i in 0..self.data.height() {
            if let Some(v) = target_col.get_f64(i) {
                if v < global_min {
                    global_min = v;
                }
                if v > global_max {
                    global_max = v;
                }
                found_any = true;
            }
        }

        if !found_any {
            return Ok(Dataset::new());
        }

        // --- STEP 2: EXPAND ROWS ---
        let mut expanded_x = Vec::new();
        let mut expanded_y = Vec::new();
        let mut expanded_groups = Vec::new();

        for key in group_order {
            if let Some(indices) = groups.remove(&key) {
                // Filter out Nulls: ECDF cannot represent missing data points
                let mut valid_indices: Vec<usize> = indices
                    .into_iter()
                    .filter(|&idx| target_col.get_f64(idx).is_some())
                    .collect();

                if valid_indices.is_empty() {
                    continue;
                }

                // Sort valid numerical values
                valid_indices.sort_by(|&a, &b| {
                    target_col
                        .get_f64(a)
                        .partial_cmp(&target_col.get_f64(b))
                        .unwrap()
                });

                let group_size = valid_indices.len() as f64;
                let group_label = key.as_deref().unwrap_or("all").to_string();

                // A. Start Padding (X_min, 0.0)
                expanded_x.push(global_min);
                expanded_y.push(0.0);
                if params.groupby.is_some() {
                    expanded_groups.push(group_label.clone());
                }

                // B. Actual Cumulative Points
                for (i, &idx) in valid_indices.iter().enumerate() {
                    let x_val = target_col.get_f64(idx).unwrap();
                    let count = (i + 1) as f64;
                    let y_val = if params.normalize {
                        count / group_size
                    } else {
                        count
                    };

                    expanded_x.push(x_val);
                    expanded_y.push(y_val);
                    if params.groupby.is_some() {
                        expanded_groups.push(group_label.clone());
                    }
                }

                // C. End Padding (X_max, 1.0 or Max Count)
                expanded_x.push(global_max);
                expanded_y.push(if params.normalize { 1.0 } else { group_size });
                if params.groupby.is_some() {
                    expanded_groups.push(group_label);
                }
            }
        }

        // --- STEP 3: CONSTRUCT RESULT DATASET ---
        let mut new_ds = Dataset::new();
        new_ds.add_column(field_name, ColumnVector::F64 { data: expanded_x })?;
        new_ds.add_column(output_name, ColumnVector::F64 { data: expanded_y })?;

        if let Some(ref g_name) = params.groupby {
            new_ds.add_column(
                g_name,
                ColumnVector::String {
                    data: expanded_groups,
                    validity: None,
                },
            )?;
        }

        Ok(new_ds)
    }
}
