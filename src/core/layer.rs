use crate::error::ChartonError;
use crate::theme::Theme;
use crate::scale::{Scale, Expansion};
use super::context::SharedRenderingContext;

/// Abstract backend for rendering shapes.
/// Implementations could be SvgBackend (String) or WgpuBackend (GPU Buffers).
pub trait RenderBackend {
    /// Draws a circle with optional fill and stroke.
    fn draw_circle(
        &mut self,
        x: f64,
        y: f64,
        radius: f64,
        fill: Option<&str>,
        stroke: Option<&str>,
        stroke_width: f64,
        opacity: f64,
    );

    /// Draws a rectangle with optional fill and stroke.
    fn draw_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: Option<&str>,
        stroke: Option<&str>,
        stroke_width: f64,
        opacity: f64,
    );

    /// Draws an open path (e.g., for lines or curves) with a stroke.
    fn draw_path(
        &mut self, 
        points: &[(f64, f64)], 
        stroke: &str, 
        stroke_width: f64, 
        opacity: f64
    );

    /// Draws a closed polygon with optional fill and stroke.
    fn draw_polygon(
        &mut self,
        points: &[(f64, f64)],
        fill: Option<&str>,
        stroke: Option<&str>,
        stroke_width: f64,
        opacity: f64,
    );

    /// Renders text with specific alignment and weight.
    fn draw_text(
        &mut self,
        text: &str,
        x: f64,
        y: f64,
        font_size: f64,
        font_family: &str,
        color: &str,
        text_anchor: &str, // "start", "middle", "end"
        font_weight: &str, // "normal", "bold"
        opacity: f64,
    );
    // other methods for drawing lines, etc.
}

/// Trait for rendering chart marks.
pub trait MarkRenderer {
    /// Generic rendering method that doesn't care about the output format.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend, // svg backend, wgpu backend etc.
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

    /// Returns the list of categorical labels if the X axis is discrete.
    fn get_x_discrete_tick_labels(&self) -> Result<Option<Vec<String>>, ChartonError>;

    // Methods to get scale type for x axes
    fn get_x_scale_type_from_layer(&self) -> Option<Scale>;

    /// Returns the (min, max) data boundaries for a continuous Y axis.
    fn get_y_continuous_bounds(&self) -> Result<(f64, f64), ChartonError>;

    /// Returns the list of categorical labels if the Y axis is discrete.
    fn get_y_discrete_tick_labels(&self) -> Result<Option<Vec<String>>, ChartonError>;

    // Methods to get scale type for y axes
    fn get_y_scale_type_from_layer(&self) -> Option<Scale>;

    /// Returns the field name mapped to the X axis, used as the default axis title.
    fn get_x_encoding_field(&self) -> Option<String>;

    /// Returns the field name mapped to the Y axis, used as the default axis title.
    fn get_y_encoding_field(&self) -> Option<String>;

    /// Returns the (min, max) bounds for color if it is continuous.
    fn get_color_continuous_bounds(&self) -> Result<Option<(f64, f64)>, ChartonError>;

    /// Returns the unique labels for color if it is discrete.
    fn get_color_discrete_labels(&self) -> Result<Option<Vec<String>>, ChartonError>;
    
    /// Returns the scale type for color defined in this layer.
    fn get_color_scale_type_from_layer(&self) -> Option<Scale>;

    // --- Shape (Always Discrete) ---
    fn get_shape_discrete_labels(&self) -> Result<Option<Vec<String>>, ChartonError>;

    // Methods to get scale type for shape encoding
    fn get_shape_scale_type_from_layer(&self) -> Option<Scale>;

    // --- Size (Always Continuous) ---
    fn get_size_continuous_bounds(&self) -> Result<Option<(f64, f64)>, ChartonError>;

    // Methods to get scale type for size encoding
    fn get_size_scale_type_from_layer(&self) -> Option<Scale>;

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