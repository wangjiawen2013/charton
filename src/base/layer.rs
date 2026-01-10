use crate::error::ChartonError;
use crate::theme::Theme;
use crate::scale::Scale; 
use super::context::SharedRenderingContext;

/// `Layer` is the core trait for the layered grammar of graphics in Charton.
/// 
/// It integrates metadata discovery (for axis and scale calculation) 
/// with the actual rendering logic for both marks and legends.
pub trait Layer {
    // --- Rendering Interfaces ---

    /// Renders the visual elements (marks) like points, bars, or lines.
    /// 
    /// # Arguments
    /// * `svg` - The string buffer where SVG elements are appended.
    /// * `context` - The shared environment containing coordinate mappers and panel dimensions.
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError>;

    /// Renders the legends associated with this specific layer.
    fn render_legends(
        &self,
        svg: &mut String,
        theme: &Theme,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError>;

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

    /// Returns the preferred `Scale` strategy (Linear, Log, Discrete, etc.) for X.
    fn get_x_scale_type(&self) -> Result<Option<Scale>, ChartonError>;

    /// Returns the preferred `Scale` strategy for Y.
    fn get_y_scale_type(&self) -> Result<Option<Scale>, ChartonError>;

    // --- Padding Preferences ---

    fn preferred_x_axis_padding_min(&self) -> Option<f64>;
    fn preferred_x_axis_padding_max(&self) -> Option<f64>;
    fn preferred_y_axis_padding_min(&self) -> Option<f64>;
    fn preferred_y_axis_padding_max(&self) -> Option<f64>;

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