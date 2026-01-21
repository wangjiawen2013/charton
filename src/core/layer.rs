use crate::error::ChartonError;
use crate::prelude::SingleColor;
use crate::scale::{Scale, ScaleDomain};
use super::context::SharedRenderingContext;
use dyn_clone::{clone_trait_object, DynClone};

/// Abstract backend for rendering shapes.
/// Implementations could be SvgBackend (String) or WgpuBackend (GPU Buffers).
pub trait RenderBackend {
    /// Draws a circle with optional fill and stroke.
    fn draw_circle(
        &mut self,
        x: f32,
        y: f32,
        radius: f32,
        fill: &SingleColor,
        stroke: &SingleColor,
        stroke_width: f32,
        opacity: f32,
    );

    /// Draws a rectangle with optional fill and stroke.
    fn draw_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        fill: &SingleColor,
        stroke: &SingleColor,
        stroke_width: f32,
        opacity: f32,
    );

    /// Draws an open path (e.g., for lines or curves) with a stroke.
    fn draw_path(
        &mut self, 
        points: &[(f32, f32)], 
        stroke: &SingleColor, 
        stroke_width: f32, 
        opacity: f32
    );

    /// Draws a closed polygon with optional fill and stroke.
    fn draw_polygon(
        &mut self,
        points: &[(f32, f32)],
        fill: &SingleColor,
        stroke: &SingleColor,
        stroke_width: f32,
        opacity: f32,
    );

    /// Renders text with specific alignment and weight.
    fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        font_family: &str,
        color: &SingleColor,
        text_anchor: &str, // "start", "middle", "end"
        font_weight: &str, // "normal", "bold"
        opacity: f32,
    );

    /// Draws a simple straight line between two points.
    /// 
    /// Commonly used for rendering axis ticks or custom markers within guides.
    fn draw_line(
        &mut self, 
        x1: f32, 
        y1: f32, 
        x2: f32, 
        y2: f32, 
        color: &SingleColor, 
        width: f32
    );

    /// Draws a rectangle filled with a linear gradient.
    /// 
    /// # Arguments
    /// * `stops` - A slice of tuples containing (offset, color), where offset is 0.0 to 1.0.
    /// * `is_vertical` - If true, gradient runs from top to bottom; otherwise, left to right.
    /// * `id_suffix` - A unique identifier used to define the gradient ID in the backend (e.g., SVG <defs>).
    fn draw_gradient_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        stops: &[(f32, SingleColor)],
        is_vertical: bool,
        id_suffix: &str,
    );
    // other methods for drawing lines, etc.
}

/// Trait for rendering the actual geometric marks (points, lines, bars).
pub trait MarkRenderer {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError>;
}

/// `Layer` is the core trait for the layered grammar of graphics in Charton.
/// The Layer trait defines the interface for any renderable component of a chart.
/// It provides methods for both the data-to-geometry rendering and the 
/// metadata inspection required for axes and legends.
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
///
/// The Layer trait supports cloning of trait objects.
/// This allows LayeredChart to be cloned even though it contains Boxed traits.
pub trait Layer: MarkRenderer + DynClone {
    // --- Axis & Scale Metadata ---

    /// Returns true if this layer requires axes to be drawn (most charts do).
    fn requires_axes(&self) -> bool;

    /// Returns the field name mapped to the X axis, used as the default axis title.
    fn get_x_encoding_field(&self) -> Option<String>;

    /// Returns the (min, max) data boundaries for a continuous X axis.
    fn get_x_continuous_bounds(&self) -> Result<(f32, f32), ChartonError>;

    /// Returns the list of categorical labels if the X axis is discrete.
    fn get_x_discrete_tick_labels(&self) -> Result<Option<Vec<String>>, ChartonError>;

    // Methods to get scale type for x axes
    fn get_x_scale_type_from_layer(&self) -> Option<Scale>;

    /// Returns the field name mapped to the Y axis, used as the default axis title.
    fn get_y_encoding_field(&self) -> Option<String>;

    /// Returns the (min, max) data boundaries for a continuous Y axis.
    fn get_y_continuous_bounds(&self) -> Result<(f32, f32), ChartonError>;

    /// Returns the list of categorical labels if the Y axis is discrete.
    fn get_y_discrete_tick_labels(&self) -> Result<Option<Vec<String>>, ChartonError>;

    // Methods to get scale type for y axes
    fn get_y_scale_type_from_layer(&self) -> Option<Scale>;

    /// Returns the field mapped to the color channel
    fn get_color_encoding_field(&self) -> Option<String>;

    /// Returns the (min, max) bounds for color if it is continuous.
    fn get_color_continuous_bounds(&self) -> Result<Option<(f32, f32)>, ChartonError>;

    /// Returns the unique labels for color if it is discrete.
    fn get_color_discrete_labels(&self) -> Result<Option<Vec<String>>, ChartonError>;
    
    /// Returns the scale type for color defined in this layer.
    fn get_color_scale_type_from_layer(&self) -> Option<Scale>;

    /// Returns the field mapped to the shape channel
    fn get_shape_encoding_field(&self) -> Option<String>;

    // --- Shape (Always Discrete) ---
    fn get_shape_discrete_labels(&self) -> Result<Option<Vec<String>>, ChartonError>;

    // Methods to get scale type for shape encoding
    fn get_shape_scale_type_from_layer(&self) -> Option<Scale>;

    /// Returns the field mapped to the size channel
    fn get_size_encoding_field(&self) -> Option<String>;

    // --- Size (Always Continuous) ---
    fn get_size_continuous_bounds(&self) -> Result<Option<(f32, f32)>, ChartonError>;

    // Methods to get scale type for size encoding
    fn get_size_scale_type_from_layer(&self) -> Option<Scale>;

    // --- State Back-filling (The "Training" ("resolve_rendering_layout") Phase) ---

    /// Sets the resolved Scale type for a specific visual channel.
    /// 
    /// This is called by the LayeredChart during the rendering pipeline to ensure
    /// all layers use a consistent scale type (e.g., forcing a Linear scale to Log 
    /// if another layer requires it).
    ///
    /// * `channel`: The name of the visual channel (e.g., "color", "size").
    /// * `scale`: The resolved Scale enum to apply.
    fn set_scale_type(&mut self, channel: &str, scale: Scale);

    /// Sets the resolved Data Domain for a specific visual channel.
    /// 
    /// This is the key "back-fill" step. The LayeredChart calculates a unified 
    /// domain (e.g., global Min/Max) from all layers and pushes it back into 
    /// each individual layer to ensure visual synchronization across the chart.
    ///
    /// * `channel`: The name of the visual channel (e.g., "color", "shape", "size").
    /// * `domain`: The unified ScaleDomain calculated from the entire dataset.
    fn set_domain(&mut self, channel: &str, domain: ScaleDomain);
}

// This line is crucial: it implements the Clone trait for all Box<dyn Layer> types via a macro.
clone_trait_object!(Layer);