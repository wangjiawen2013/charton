#[macro_export]
macro_rules! register_polars_bridge {
    () => {
        /// Implement the bridge for &polars::prelude::DataFrame
        impl $crate::core::data::IntoChartonSource for &polars::prelude::DataFrame {
            fn into_source(self) -> Result<$crate::core::data::DataFrameSource, $crate::error::ChartonError> {
                use polars_io::parquet::{ParquetWriter, ParquetCompression};
                use std::io::Cursor;

                let mut buf = Vec::new();
                let mut wrapper = Cursor::new(&mut buf);

                // Serialize the user's DataFrame into a byte buffer
                polars_io::parquet::ParquetWriter::new(&mut wrapper)
                    .with_compression(polars_io::parquet::ParquetCompression::Uncompressed)
                    .finish(&mut self.clone())
                    .map_err(|e| $crate::error::ChartonError::Data(
                        format!("Cross-version DataFrame bridge failure: {}", e)
                    ))?;

                // Pass the serialized bytes to Charton
                $crate::core::data::IntoChartonSource::into_source(buf.as_slice())
            }
        }

        /// Implement the bridge for &polars::prelude::LazyFrame
        impl $crate::core::data::IntoChartonSource for &polars::prelude::LazyFrame {
            fn into_source(self) -> Result<$crate::core::data::DataFrameSource, $crate::error::ChartonError> {
                // Collect the user's LazyFrame into a user's DataFrame first
                let df = self.clone().collect()
                    .map_err(|e| $crate::error::ChartonError::Data(
                        format!("Cross-version LazyFrame collection failure: {}", e)
                    ))?;
                
                // Use the &DataFrame implementation above to finish the bridge
                $crate::core::data::IntoChartonSource::into_source(&df)
            }
        }
    };
}