//! `bridge`: Python interoperability (requires Python installed). More languages may be supported in the future.
mod python;
mod r;

pub mod base {
    use crate::error::ChartonError;
    use polars::prelude::{DataFrame, LazyFrame, ParquetReader, SerReader};
    use serde::{Deserialize, Serialize};
    use std::marker::PhantomData;

    /// A container that associates a name with a DataFrame value.
    ///
    /// This struct wraps a Polars DataFrame along with its string name, enabling named data
    /// exchange between Rust and other language environments. It is primarily used for
    /// passing data to visualization libraries through the bridge system.
    ///
    /// The struct derives `Serialize` and `Deserialize` traits, allowing it to be
    /// easily converted to and from various data formats (like JSON) when communicating
    /// with external systems.
    ///
    /// # Fields
    /// * `name` - A string identifier for the DataFrame
    /// * `df` - The actual Polars DataFrame containing the data
    #[derive(Serialize, Deserialize, Debug)]
    pub struct InputData {
        // The name identifier for this dataframe, typically derived from the variable name
        pub(crate) name: String,
        pub(crate) df: DataFrame,
    }

    /// A container that associates a name with a serialized data value.
    ///
    /// This struct wraps a serialized string representation of data along with its string name,
    /// It is primarily used for storing serialized data from `InputData`.
    ///
    /// The struct derives `Serialize` and `Deserialize` traits, allowing it to be
    /// easily converted to and from various data formats when communicating with external systems.
    ///
    /// # Fields
    /// * `name` - A string identifier for the serialized data
    /// * `value` - The serialized data as a string representation
    #[derive(Serialize, Deserialize, Debug)]
    pub struct SerializedData {
        // The name identifier for this value, typically derived from the variable name
        pub(crate) name: String,
        pub(crate) value: String,
    }

    impl InputData {
        /// Creates a new InputData instance with a custom name
        ///
        /// # Arguments
        /// * `name` - The variable name of the DataFrame
        /// * `df` - The DataFrame to wrap
        ///
        /// # Returns
        /// A new InputData instance
        fn new(name: &str, df: DataFrame) -> Self {
            Self {
                name: name.to_string(),
                df,
            }
        }
    }

    impl TryFrom<(&str, &DataFrame)> for InputData {
        type Error = ChartonError;

        fn try_from((name, df): (&str, &DataFrame)) -> Result<Self, Self::Error> {
            Ok(InputData::new(name, df.clone()))
        }
    }

    impl TryFrom<(&str, &LazyFrame)> for InputData {
        type Error = ChartonError;

        fn try_from((name, lf): (&str, &LazyFrame)) -> Result<Self, Self::Error> {
            let df = lf.clone().collect()?;
            Ok(InputData::new(name, df))
        }
    }

    impl TryFrom<(&str, &Vec<u8>)> for InputData {
        type Error = ChartonError;

        /// Creates a new InputData from a name and Parquet-encoded data.
        ///
        /// This allows users to pass DataFrames serialized as Parquet data,
        /// enabling interoperability between different Polars versions and
        /// external systems that export data in Parquet format.
        ///
        /// # Arguments
        /// * `name` - The variable name to associate with the DataFrame
        /// * `parquet_data` - A reference to the vector of bytes containing
        ///                    Parquet-serialized DataFrame
        ///
        /// # Returns
        /// A new InputData instance containing the deserialized DataFrame
        /// with the provided name.
        ///
        /// # Errors
        /// Returns a ChartonError if the Parquet data cannot be read into a DataFrame.
        fn try_from((name, parquet_data): (&str, &Vec<u8>)) -> Result<Self, Self::Error> {
            let cursor = std::io::Cursor::new(parquet_data);
            let df = ParquetReader::new(cursor).finish()?;
            Ok(InputData::new(name, df))
        }
    }

    impl SerializedData {
        pub(crate) fn new(name: &str, value: String) -> Self {
            Self {
                name: name.to_string(),
                value,
            }
        }
    }

    /// A macro that creates a `InputData` instance from a variable.
    ///
    /// This macro simplifies the creation of `InputData` instances by automatically
    /// using the variable's name as the string identifier. It converts the variable
    /// identifier to a string using `stringify!` and wraps the variable's value
    /// in a `InputData` container.
    ///
    /// # Parameters
    /// * `$var` - An identifier for a variable whose name will be used as the identifier
    ///            and whose value will be stored in the `InputData`
    ///
    /// # Example
    /// ```
    /// let dataframe = DataFrame::new(...);
    /// let named_value = data!(&dataframe)?;
    /// // This will use TryFrom implementation to convert the data
    /// ```
    ///
    /// # Returns
    /// A `InputData` instance with the variable's name as the identifier and the
    /// variable's value as the contained data.
    #[macro_export]
    macro_rules! data {
        (&$var:ident) => {
            <$crate::bridge::base::InputData as ::std::convert::TryFrom<_>>::try_from((
                stringify!($var),
                &$var,
            ))
        };
    }

    /// A marker trait for visualization library renderers.
    ///
    /// This trait serves as a common interface for different visualization backends
    /// that can be used to render plots. It doesn't define any methods itself, but
    /// acts as a type-level marker to ensure type safety when working with different
    /// rendering engines.
    ///
    /// Implementors of this trait represent specific visualization libraries such as:
    /// - `Altair`: For creating statistical visualizations using the Altair library
    /// - `Matplotlib`: For creating plots using the Matplotlib library
    ///
    /// This trait is used in conjunction with the `Plot` struct to enable generic
    /// programming over different visualization backends.
    pub trait Renderer {}

    /// A marker struct representing the Altair visualization library backend.
    ///
    /// This struct implements the `Renderer` trait and serves as a marker type
    /// to indicate that Altair should be used as the visualization backend.
    /// Altair is a statistical visualization library based on Vega-Lite that
    /// provides a declarative interface for creating interactive visualizations.
    ///
    /// This struct does not contain any fields or methods itself, but is used
    /// as a type parameter in the `Plot` struct to select Altair as the rendering
    /// engine for generating visualizations.
    ///
    /// # Example
    /// ```ignore
    /// let plot = Plot::<Altair>::build(&data)?;
    /// ```
    pub struct Altair {}
    impl Renderer for Altair {}

    /// A marker struct representing the Matplotlib visualization library backend.
    ///
    /// This struct implements the `Renderer` trait and serves as a marker type
    /// to indicate that Matplotlib should be used as the visualization backend.
    /// Matplotlib is a comprehensive library for creating static, animated, and
    /// interactive visualizations in Python.
    ///
    /// This struct does not contain any fields or methods itself, but is used
    /// as a type parameter in the `Plot` struct to select Matplotlib as the rendering
    /// engine for generating visualizations.
    ///
    /// # Example
    /// ```ignore
    /// let plot = Plot::<Matplotlib>::build(&data)?;
    /// ```
    pub struct Matplotlib {}
    impl Renderer for Matplotlib {}

    /// A trait that defines the core functionality for visualization libraries.
    ///
    /// This trait specifies the essential methods that any visualization backend
    /// must implement to integrate with the bridge system. It provides a unified
    /// interface for creating, configuring, and executing visualizations across
    /// different rendering engines such as Altair or Matplotlib.
    ///
    /// The trait is designed to work with `InputData` as input data and supports
    /// common operations like setting execution paths, customizing plotting code,
    /// displaying visualizations, saving to files.
    pub trait Visualization {
        /// Creates a new visualization instance with the provided data.
        ///
        /// This method initializes a visualization object with the given DataFrame
        /// wrapped in a `InputData`. The implementation will convert the DataFrame
        /// to a format suitable for the specific visualization library.
        ///
        /// # Parameters
        /// * `data` - A `InputData` containing a DataFrame to be visualized
        ///
        /// # Returns
        /// A Result containing either:
        /// - Ok(Self) with the new visualization instance
        /// - Err(ChartonError) if there was an error during initialization
        fn build(data: InputData) -> Result<Self, ChartonError>
        where
            Self: Sized;

        /// Sets the interpreter/executable path for running the visualization code
        ///
        /// # Parameters
        /// * `exe_path` - Path to the interpreter or executable that will run the visualization code
        ///
        /// # Returns
        /// Self with the updated executable path
        ///
        /// # Errors
        /// Returns a ChartonError if the provided path does not exist or is not executable
        fn with_exe_path<P: AsRef<std::path::Path>>(
            self,
            exe_path: P,
        ) -> Result<Self, ChartonError>
        where
            Self: Sized;

        // Change the with_plotting_code method signature
        /// Sets custom plotting code to be executed by the renderer.
        ///
        /// This method allows users to provide their own plotting code for generating
        /// visualizations.
        ///
        /// # Parameters
        /// * `plotting_code` - A string slice containing the plotting code to execute
        ///
        /// # Returns
        /// Self with the updated plotting code
        fn with_plotting_code(self, code: &str) -> Self;

        /// Executes the visualization code and displays the result in Jupyter.
        ///
        /// This method runs the generated or provided plotting code and renders
        /// the visualization directly in a Jupyter notebook environment.
        ///
        /// # Returns
        /// Result indicating success or a ChartonError if the operation fails
        fn show(&self) -> Result<(), ChartonError>;

        /// Executes the visualization code and saves the output to a file.
        ///
        /// This method runs the visualization code and saves the resulting plot
        /// to the specified file path. The format is typically inferred from
        /// the file extension. Currently, only SVG and PNG format are supported.
        ///
        /// # Parameters
        /// * `path` - A path-like object specifying where to save the visualization
        ///
        /// # Returns
        /// Result indicating success or a ChartonError if the operation fails
        fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ChartonError>;
    }

    // Helper trait for generating plotting scripts with common methods
    pub(crate) trait ExternalRendererExecutor {
        // Responsible for dynamically generating complete plotting scripts based on output format (json/svg/png).
        fn generate_full_plotting_code(&self, output_format: &str) -> Result<String, ChartonError>;

        // Responsible for executing the generated plotting script and returning the result.
        fn execute_plotting_code(&self, code: &str) -> Result<String, ChartonError>;
    }

    /// A generic struct for creating visualizations using different rendering backends.
    ///
    /// This struct represents a visualization that can be rendered using various
    /// visualization libraries (renderers) such as Altair or Matplotlib. It uses
    /// Rust's generics and the `Renderer` trait to provide a flexible interface
    /// for switching between different visualization backends at compile time.
    ///
    /// The struct holds the data to be visualized, the path to the execution environment,
    /// the plotting code to be run, and uses `PhantomData` to maintain type information
    /// about the specific renderer being used.
    ///
    /// # Type Parameters
    /// * `T` - The renderer type that implements the `Renderer` trait, such as `Altair` or `Matplotlib`
    ///
    /// # Fields
    /// * `data` - The data to be visualized, wrapped in a `SerializedData`
    /// * `exe_path` - Path to the interpreter or compiler for executing the visualization code
    /// * `raw_plotting_code` - The raw plotting code that generates the visualization by user
    /// * `_renderer` - PhantomData to hold type information about the renderer
    pub struct Plot<T: Renderer> {
        pub(crate) data: SerializedData,
        pub(crate) exe_path: String,
        pub(crate) raw_plotting_code: String,
        pub(crate) _renderer: PhantomData<T>,
    }
}

#[cfg(test)]
mod tests {
    use crate::data;
    use polars::prelude::df;

    #[test]
    fn data_works() {
        let df = df![
            "a" => [1, 2, 3],
            "b" => [4, 5, 6]
        ]
        .unwrap();
        let result = data!(&df).unwrap();
        assert_eq!(result.name, "df");
    }
}
