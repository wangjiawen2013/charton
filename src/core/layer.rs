use crate::error::ChartonError;
use crate::prelude::SingleColor;
use crate::scale::{Scale, ScaleDomain, ScaleTrait, Expansion};
use crate::core::context::SharedRenderingContext;
use crate::encode::Channel;
use std::sync::Arc;

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
    /// Executes the drawing logic for this layer's marks.
    /// 
    /// This is called after all scales have been "trained" and resolved.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError>;
}

/// `Layer` is the core trait for the layered grammar of graphics in Charton.
/// 
/// A layer represents a single component of a chart (e.g., a Scatter plot, a 
/// Regression line, or a Bar set). It integrates metadata discovery 
/// with the actual rendering logic.
///
/// The lifecycle of a Layer during rendering is:
/// 1. **Discovery**: `LayeredChart` queries `get_data_bounds` for all active channels.
/// 2. **Training**: The engine calculates a global domain based on all layers.
/// 3. **Resolution**: The engine calls `set_resolved_scale` to inject the final scale.
/// 4. **Rendering**: The engine calls `render_marks`.
pub trait Layer: MarkRenderer + Send + Sync {
    // --- Metadata Discovery Phase ---

    /// Returns true if this layer requires coordinate axes to be drawn.
    fn requires_axes(&self) -> bool;

    /// Returns the data field name mapped to a specific visual channel.
    fn get_field(&self, channel: Channel) -> Option<String>;

    /// Returns the user's preferred scale type (e.g., Linear, Log) for a channel.
    fn get_scale(&self, channel: Channel) -> Option<Scale>;

    /// Returns the user-defined domain override for a channel, if any.
    fn get_domain(&self, channel: Channel) -> Option<ScaleDomain>;
    
    /// Returns the expansion/padding rules for a channel.
    fn get_expand(&self, channel: Channel) -> Option<Expansion>;

    /// Calculates the raw data boundaries (Min/Max or Unique Categories) for this layer.
    /// 
    /// This is the primary input for the "Training" phase where global scales are built.
    fn get_data_bounds(&self, channel: Channel) -> Result<ScaleDomain, ChartonError>;

    // --- State Resolution (The "Back-filling" Phase) ---

    /// Injects the final, trained scale instance into the layer's encoding.
    ///
    /// This ensures that if multiple layers share an axis (e.g., two different 
    /// datasets on the same Y-axis), they both receive the same mathematical 
    /// mapping logic.
    /// 
    /// * `channel`: The visual aesthetic being resolved.
    /// * `scale`: The shared, thread-safe scale object.
    fn set_resolved_scale(&mut self, channel: Channel, scale: Arc<dyn ScaleTrait>);
}