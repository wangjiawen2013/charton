/// Loads a Polars DataFrame into a Charton Dataset using a macro.
///
/// This macro performs "duck typing" at compile time, meaning it will work
/// with any version of Polars the user has in their project, as long as
/// the core methods (`align_chunks`, `iter_chunks`, `to_arrow_rs`) are available.
///
/// # Requirements
/// The user's `polars` dependency must have the `"arrow_rs"` feature enabled.
///
/// # Arguments
/// * `$df` - An expression that evaluates to a `polars::prelude::DataFrame`.
///
/// # Example
/// ```ignore
/// let df = df!["col1" => [1, 2]]?;
/// let dataset = load_polars_df!(df)?;
/// ```
#[macro_export]
macro_rules! load_polars_df {
    ($df:expr) => {{
        // 1. Capture the DataFrame. We use a local mutable binding.
        let mut df = $df;

        // 2. Align chunks to ensure memory continuity.
        // This is crucial for high-performance processing of 10M+ rows.
        df.align_chunks();

        // 3. Convert Polars chunks to Arrow RecordBatches.
        // We use fully qualified paths to ensure it works even if the user
        // hasn't imported RecordBatch.
        let batches_result: ::std::result::Result<::std::vec::Vec<_>, _> = df
            .iter_chunks(true)
            .map(|chunk| chunk.to_arrow_rs())
            .collect();

        // 4. Map error and pass batches to the Dataset constructor.
        // We let the compiler infer the RecordBatch type to avoid path issues.
        batches_result
            .map_err(|e| {
                $crate::error::ChartonError::Data(format!(
                    "Polars to Arrow conversion failed: {}",
                    e
                ))
            })
            .and_then(|batches| $crate::core::data::Dataset::from_record_batches(&batches))
    }};
}
