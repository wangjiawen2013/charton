#[macro_export]
macro_rules! register_polars_bridge {
    () => {
        /// Implementation for &polars::prelude::DataFrame.
        /// Bridges different Polars versions by serializing data through 
        /// Charton's internal Parquet writer.
        impl $crate::core::data::IntoChartonSource for &polars::prelude::DataFrame {
            fn into_source(self) -> Result<$crate::core::data::DataFrameSource, $crate::error::ChartonError> {
                use std::io::Cursor;

                let mut buf = Vec::new();
                let mut wrapper = Cursor::new(&mut buf);

                // Use Charton's internal polars-io to perform the serialization.
                // This guarantees version compatibility for the serialization step.
                $crate::__private_polars_io::parquet::ParquetWriter::new(&mut wrapper)
                    .with_compression($crate::__private_polars_io::parquet::ParquetCompression::Uncompressed)
                    .finish(&mut self.clone())
                    .map_err(|e| $crate::error::ChartonError::Data(
                        format!("Cross-version DataFrame bridge serialization failed: {}", e)
                    ))?;

                // Pass the resulting buffer to Charton's internal byte-slice implementation.
                $crate::core::data::IntoChartonSource::into_source(buf.as_slice())
            }
        }

        /// Implementation for &polars::prelude::LazyFrame.
        /// Collects the external LazyFrame into a DataFrame before bridging.
        impl $crate::core::data::IntoChartonSource for &polars::prelude::LazyFrame {
            fn into_source(self) -> Result<$crate::core::data::DataFrameSource, $crate::error::ChartonError> {
                // Collect using the user's local Polars engine.
                let df = self.clone().collect()
                    .map_err(|e| $crate::error::ChartonError::Data(
                        format!("Cross-version LazyFrame collection failed: {}", e)
                    ))?;
                
                // Delegate to the &DataFrame bridge implementation defined above.
                $crate::core::data::IntoChartonSource::into_source(&df)
            }
        }
    };
}