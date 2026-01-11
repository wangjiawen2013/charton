use crate::error::ChartonError;
use crate::theme::Theme;
use crate::scale::Expansion;
use super::context::SharedRenderingContext;

/// Trait for rendering chart marks.
///
/// Defines how visual elements (points, bars, lines) are appended to the SVG.
pub trait MarkRenderer {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError>;
}

/// Trait for rendering chart legends.
///
/// Defines how legend elements are drawn based on the current theme and context.
pub trait LegendRenderer {
    fn render_legends(
        &self,
        svg: &mut String,
        theme: &Theme,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError>;
}

/// `Layer` is the core trait for the layered grammar of graphics in Charton.
/// 
/// It integrates metadata discovery (for axis and scale calculation) 
/// with the actual rendering logic by combining `MarkRenderer` and `LegendRenderer`.
///
/// The `Layer` trait defines the interface that all chart layers must implement
/// to be part of a layered chart. It combines `MarkRenderer` and `LegendRenderer`
/// traits to provide complete rendering capabilities for a chart layer.
///
/// This trait also defines methods for:
/// - Controlling axis rendering
/// - Providing axis padding preferences
/// - Getting data bounds for continuous axes
/// - Getting tick labels for discrete axes
/// - Retrieving encoding field names
/// - Getting axis scales
/// - Calculating legend width
/// - Checking if axes should be swapped
pub trait Layer: MarkRenderer + LegendRenderer {
    // --- Axis & Scale Metadata ---

    /// Returns true if this layer requires axes to be drawn (most charts do).
    fn requires_axes(&self) -> bool;

    /// Returns the (min, max) data boundaries for a continuous X axis.
    fn get_x_continuous_bounds(&self) -> Result<(f64, f64), ChartonError>;

    /// Returns the (min, max) data boundaries for a continuous Y axis.
    fn get_y_continuous_bounds(&self) -> Result<(f64, f64), ChartonError>;

    /// Returns the list of categorical labels if the X axis is discrete.
    fn get_x_discrete_tick_labels(&self) -> Result<Option<Vec<String>>, ChartonError>;

    /// Returns the list of categorical labels if the Y axis is discrete.
    fn get_y_discrete_tick_labels(&self) -> Result<Option<Vec<String>>, ChartonError>;

    /// Returns the field name mapped to the X axis, used as the default axis title.
    fn get_x_encoding_field(&self) -> Option<String>;

    /// Returns the field name mapped to the Y axis, used as the default axis title.
    fn get_y_encoding_field(&self) -> Option<String>;

    /// Returns the data type of the X encoding column.
    fn get_x_data_type(&self) -> Option<polars::datatypes::DataType>;

    /// Returns the data type of the Y encoding column.
    fn get_y_data_type(&self) -> Option<polars::datatypes::DataType>;

    // --- Padding Preferences ---

    /// Method to get preferred axis expanding for this layer
    fn preferred_x_axis_expanding(&self) -> Expansion;
    fn preferred_y_axis_expanding(&self) -> Expansion;

    // --- Layout Calculation ---

    /// Calculates the horizontal space required to render this layer's legend.
    fn calculate_legend_width(
        &self,
        theme: &Theme,
        chart_height: f64,
        top_margin: f64,
        bottom_margin: f64,
    ) -> f64;
}