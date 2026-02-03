use crate::chart::Chart;
use crate::error::ChartonError;
use crate::mark::Mark;
use polars::prelude::*;

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
    /// ```
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
    /// ```
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
    /// ```
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
    /// ```
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
    /// ```
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
    /// ```
    /// let window_transform = window_transform.with_normalize(true);
    /// ```
    pub fn with_normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }
}

impl<T: Mark> Chart<T> {
    /// Transform data by performing window operations
    ///
    /// This method computes window functions on the data, such as cumulative distribution,
    /// ranking, or lag/lead operations. The computation can be grouped by a specified field
    /// and configured with various window parameters.
    ///
    /// # Parameters
    /// * `params` - A `WindowTransform` configuration object specifying the window operation details
    ///
    /// # Returns
    /// * `Result<Self, ChartonError>` - The chart with transformed window data or an error if the transformation fails
    ///
    /// # Example
    /// ```
    /// let window_field = WindowFieldDef::new("value", WindowOnlyOp::CumeDist, "cumulative_dist");
    /// let window_params = WindowTransform::new(window_field).with_groupby("category");
    ///
    /// chart.transform_window(window_params)?;
    /// ```
    pub fn transform_window(mut self, params: WindowTransform) -> Result<Self, ChartonError> {
        // Process the window operation
        let field_name = &params.window.field;
        let window_op = &params.window.op;
        let output_field_name = &params.window.as_;
        let normalize = params.normalize;

        // Determine the group field name once to avoid duplication code
        let group_field_name = params
            .groupby
            .clone()
            .unwrap_or_else(|| format!("__charton_temp_group_{}", crate::TEMP_SUFFIX));

        // Create a working DataFrame with grouping column
        let working_df = if let Some(ref group_field) = params.groupby {
            // Use existing group field
            self.data.df.select([field_name, group_field])?
        } else {
            // Create a temp grouping column with a single group
            let temp_group_series = Series::new(
                (&group_field_name).into(),
                vec!["temp"; self.data.df.height()],
            );
            self.data
                .df
                .select([field_name])?
                .with_column(temp_group_series)?
                .clone()
        };

        // Apply window operations using the working_df with guaranteed group column
        match window_op {
            WindowOnlyOp::CumeDist => {
                // Use uuid as column name to avoid column name conflicts
                let cumulative_freq_col = format!(
                    "__charton_temp_cumulative_freq_{}",
                    crate::TEMP_SUFFIX
                );
                let total_freq_col =
                    format!("__charton_temp_total_freq_{}", crate::TEMP_SUFFIX);
                let group_order_col =
                    format!("__charton_temp_group_order_{}", crate::TEMP_SUFFIX);

                // Compute original group appearance order
                let group_order_df = working_df
                    .clone()
                    .lazy()
                    .select([col(&group_field_name)])
                    .unique_stable(None, UniqueKeepStrategy::First) // Keep first occurrence
                    .with_row_index(&group_order_col, None); // Assign sequential numbers to each group

                // Compute cumulative frequency per group (optionally normalized)
                let mut dataset = working_df
                    .lazy()
                    .with_columns([as_struct(vec![col(field_name)])
                        .rank(
                            RankOptions {
                                method: RankMethod::Max, // Take maximum rank when tied
                                descending: false, // The smallest value gets rank = 1, otherwise the largest value gets rank = 1
                            },
                            None,
                        )
                        .over([col(&group_field_name)]) // Rank within groups
                        .cast(DataType::Float64)
                        .alias(&cumulative_freq_col)])
                    // Join group order back
                    .join(
                        group_order_df,
                        [col(&group_field_name)],
                        [col(&group_field_name)],
                        JoinArgs::new(JoinType::Left),
                    )
                    // Sort: first by group appearance order, then by field ascending within groups
                    .sort_by_exprs(
                        &[col(&group_order_col), col(field_name)],
                        SortMultipleOptions::default()
                            .with_order_descending_multi(vec![false, false]),
                    )
                    .drop([group_order_col]) // Drop temporary column after use
                    // Deduplicate: keep only first occurrence of cumulative frequency within each group
                    .unique_stable(
                        Some(vec![
                            group_field_name.clone().into(),
                            cumulative_freq_col.clone().into(),
                        ]),
                        UniqueKeepStrategy::First,
                    );

                // Compute total frequency per group
                let total_frequency_per_group = dataset
                    .clone()
                    .group_by([col(&group_field_name)])
                    .agg([col(&cumulative_freq_col).max().alias(&total_freq_col)]);

                // Join the total frequency back to the main dataset
                dataset = dataset.join(
                    total_frequency_per_group,
                    [col(&group_field_name)],
                    [col(&group_field_name)],
                    JoinArgs::new(JoinType::Left),
                );

                // Conditionally normalize cumulative frequency
                // Using `when().then().otherwise()` keeps it within the lazy pipeline
                let dataset = dataset
                    .with_columns([
                        when(lit(normalize))
                            .then(col(&cumulative_freq_col) / col(&total_freq_col))
                            .otherwise(col(&cumulative_freq_col))
                            .alias(output_field_name), // Use output_field_name as final column name
                    ])
                    // Drop temporary columns to clean up the final result
                    .drop([cumulative_freq_col, total_freq_col]);

                self.data.df = dataset.collect()?;
            }
            WindowOnlyOp::RowNumber => {
                // Add row number column using Polars' lazy API with grouping
            }
            _ => {
                return Err(ChartonError::Unimplemented(format!(
                    "Window operation {:?} is not yet implemented",
                    window_op
                )));
            }
        }

        // If no real groupby was specified, remove the temp group column
        if params.groupby.is_none() {
            self.data.df = self.data.df.lazy().drop([group_field_name]).collect()?;
        }

        Ok(self)
    }
}
