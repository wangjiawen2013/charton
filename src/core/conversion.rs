use crate::chart::Chart;
use crate::core::composite::LayeredChart;
use crate::mark::Mark;

/// Implementation of the [From] trait to convert a single-layer [Chart] into a [LayeredChart].
/// 
/// This implementation allows seamless conversion from a single-chart instance to a layered
/// chart instance, enabling users to start with simple charts and later add complexity by
/// combining multiple layers.
/// 
/// The conversion works by creating a new [LayeredChart] with default settings and adding
/// the source chart as the first (and initially only) layer.
/// 
/// # Type Parameters
/// * `T` - The mark type implementing [Mark] trait, defining the chart type
/// 
/// # Where Clause Requirements
/// * `T: Mark + 'static` - The mark type must implement [Mark] trait and have static lifetime
/// * `Chart<T>: crate::core::layer::Layer` - The chart must implement the [Layer] trait to be usable as a layer
/// 
/// # Example
/// ```rust
/// use charton::prelude::*;
/// use polars::prelude::*;
/// 
/// let df = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
/// let chart = Chart::<MarkPoint>::build(&df)?
///     .mark_point()
///     .encode(x("x"), y("y"))?;
/// 
/// // Convert using the From trait
/// let layered_chart: LayeredChart = chart.into();
/// ```
impl<T: Mark + 'static> From<Chart<T>> for LayeredChart
where
    Chart<T>: crate::core::layer::Layer,
{
    /// Converts a single-layer chart into a layered chart by adding it as the first layer.
    /// 
    /// This method creates a new [LayeredChart] with default settings and adds the input
    /// chart as its first layer. The resulting [LayeredChart] can then accept additional
    /// layers via the [LayeredChart::add_layer] method.
    /// 
    /// # Arguments
    /// * `val` - The source [Chart] to convert into a [LayeredChart]
    /// 
    /// # Returns
    /// A new [LayeredChart] instance with the input chart as its first layer
    fn from(val: Chart<T>) -> Self {
        LayeredChart::new().add_layer(val)
    }
}

/// Extension implementation for [Chart] to provide the `into_layered()` method.
/// 
/// This implementation extends the [Chart] type with a convenience method that allows
/// users to convert a single chart to a layered chart with a more intuitive API.
/// 
/// The method delegates to the [From] trait implementation, ensuring consistent behavior
/// across both conversion approaches.
/// 
/// # Type Parameters
/// * `T` - The mark type implementing [Mark] trait, defining the chart type
/// 
/// # Where Clause Requirements
/// * `T: Mark + 'static` - The mark type must implement [Mark] trait and have static lifetime
/// * `Chart<T>: crate::core::layer::Layer` - The chart must implement the [Layer] trait to be usable as a layer
/// 
/// # Example
/// ```rust
/// use charton::prelude::*;
/// use polars::prelude::*;
/// 
/// let df = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
/// let chart = Chart::<MarkPoint>::build(&df)?
///     .mark_point()
///     .encode(x("x"), y("y"))?;
/// 
/// // Convert using the convenience method
/// let layered_chart = chart.into_layered();
/// ```
impl<T: Mark + 'static> Chart<T>
where
    Chart<T>: crate::core::layer::Layer,
{
    /// Converts the current single-layer chart into a [LayeredChart] instance.
    /// 
    /// This convenience method provides a more readable alternative to using the [From] trait
    /// directly. It enables fluent-style programming where charts can be easily transformed
    /// from single-layer to multi-layer format.
    /// 
    /// The method internally uses the [From] trait implementation, ensuring consistent behavior.
    /// After conversion, additional layers can be added to the returned [LayeredChart] using
    /// the [LayeredChart::add_layer] method.
    /// 
    /// # Returns
    /// A new [LayeredChart] instance with this chart as its first layer
    /// 
    /// # Example
    /// ```rust
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    /// 
    /// let df = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
    /// let chart = Chart::<MarkPoint>::build(&df)?
    ///     .mark_point()
    ///     .encode(x("x"), y("y"))?;
    /// 
    /// // Convert to layered chart for further composition
    /// let layered_chart = chart.into_layered();
    /// 
    /// // Now additional layers can be added
    /// let final_chart = layered_chart
    ///     .with_title("Combined Visualization")
    ///     .with_size(600, 400);
    /// ```
    pub fn into_layered(self) -> LayeredChart {
        self.into()
    }
}
