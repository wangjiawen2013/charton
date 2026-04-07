/// Loads a Polars DataFrame into a Charton Dataset using a macro.
///
/// This version utilizes `rechunk_into_arrow` to provide a memory-efficient,
/// ownership-taking conversion. By consuming the DataFrame, we minimize memory
/// overhead—critical for high-performance processing of 10M+ rows.
///
/// This macro is "GPU-ready," as it ensures each column is a single, contiguous
/// memory chunk, making it ideal for future `wgpu` buffer uploads.
///
/// # Arguments
/// * `$df` - An expression that evaluates to a `polars::prelude::DataFrame`.
///
/// # Example
/// ```ignore
/// let df = df!["col1" => [1, 2, 3]]?;
/// let dataset = load_polars_df!(df)?;
/// ```
#[macro_export]
macro_rules! load_polars_df {
    ($df:expr) => {{
        // 1. Capture the DataFrame and take ownership.
        // Taking ownership allows us to use `into_arrow` methods which can
        // be more memory-efficient than borrowing.
        let df = $df;

        // 2. Define the Arrow compatibility level.
        // We use the default (usually Newest) to ensure Polars logic types
        // are mapped to the most modern Arrow physical layouts.
        let compat_level = ::polars::prelude::CompatLevel::default();

        // 3. Get the Schema before consuming the DataFrame.
        // We need the schema to maintain metadata and field names in the Dataset.
        let schema = df.schema().clone();

        // 4. Perform the core conversion.
        // `rechunk_into_arrow` handles two critical tasks for 10M+ data:
        // - Parallel Rechunking: Merges fragmented memory into a single contiguous block per column.
        // - Arrow Conversion: Returns Vec<Box<dyn Array>>, which is the optimal
        //   Structure of Arrays (SoA) layout for GPU buffer mapping (wgpu).
        let columns = df.rechunk_into_arrow(compat_level);

        // 5. Construct the Dataset from the contiguous Arrow arrays.
        // We use the fully qualified path to our internal Dataset constructor.
        $crate::core::data::Dataset::from_arrays(schema, columns)
            .map_err(|e| {
                $crate::error::ChartonError::Data(format!(
                    "Polars to Arrow conversion (into_arrow) failed: {}",
                    e
                ))
            })
    }};
}
