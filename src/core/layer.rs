use crate::error::ChartonError;
use crate::prelude::SingleColor;
use crate::scale::{Scale, ScaleDomain, Expansion};
use crate::core::context::PanelContext;
use crate::coordinate::CoordinateTrait;
use super::aesthetics::GlobalAesthetics;
use crate::encode::Channel;
use std::sync::Arc;

/// Abstract backend for rendering shapes.
/// Implementations could be SvgBackend (String) or WgpuBackend (GPU Buffers).
pub trait RenderBackend {
    /// Draws a circle with optional fill and stroke.
    fn draw_circle(
        &mut self,
        x: f64,
        y: f64,
        radius: f64,
        fill: &SingleColor,
        stroke: &SingleColor,
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
        fill: &SingleColor,
        stroke: &SingleColor,
        stroke_width: f64,
        opacity: f64,
    );

    /// Draws an open path (e.g., for lines or curves) with a stroke.
    fn draw_path(
        &mut self, 
        points: &[(f64, f64)], 
        stroke: &SingleColor, 
        stroke_width: f64, 
        opacity: f64
    );

    /// Draws a closed polygon with optional fill and stroke.
    fn draw_polygon(
        &mut self,
        points: &[(f64, f64)],
        fill: &SingleColor,
        stroke: &SingleColor,
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
        color: &SingleColor,
        text_anchor: &str, // "start", "middle", "end"
        font_weight: &str, // "normal", "bold"
        opacity: f64,
    );

    /// Draws a simple straight line between two points.
    /// 
    /// Commonly used for rendering axis ticks or custom markers within guides.
    fn draw_line(
        &mut self, 
        x1: f64, 
        y1: f64, 
        x2: f64, 
        y2: f64, 
        color: &SingleColor, 
        width: f64
    );

    /// Draws a rectangle filled with a linear gradient.
    /// 
    /// # Arguments
    /// * `stops` - A slice of tuples containing (offset, color), where offset is 0.0 to 1.0.
    /// * `is_vertical` - If true, gradient runs from top to bottom; otherwise, left to right.
    /// * `id_suffix` - A unique identifier used to define the gradient ID in the backend (e.g., SVG <defs>).
    fn draw_gradient_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        stops: &[(f64, SingleColor)],
        is_vertical: bool,
        id_suffix: &str,
    );
    // other methods for drawing lines, etc.
}

/// `MarkRenderer` defines the contract for drawing geometric primitives.
///
/// Implementations of this trait (like PointRenderer or LineRenderer) focus 
/// purely on the visual output using the provided coordinate tools.
pub trait MarkRenderer {
    /// Executes the drawing logic for this layer's marks.
    /// 
    /// # Arguments
    /// * `backend`: The drawing engine (e.g., SVG, Canvas) that provides low-level primitives.
    /// * `context`: The localized `PanelContext` containing the coordinate system 
    ///   and the physical area for this specific rendering pass.
    /// 
    /// # Faceting Logic:
    /// In a faceted chart, this method may be called multiple times for a single layer,
    /// each time receiving a different `PanelContext` with a new `Rect` and potentially 
    /// different `Scale` bounds.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError>;
}

/// `Layer` is the core trait for the layered grammar of graphics in Charton.
/// 
/// A layer represents a single component of a chart (e.g., a Scatter plot, a 
/// Regression line, or a Bar set). It integrates metadata discovery 
/// with the actual rendering logic.
///
/// The lifecycle of a Layer during the rendering pipeline is:
/// 1. **Discovery**: `LayeredChart` queries `get_data_bounds` to understand data ranges.
/// 2. **Training**: The engine aggregates bounds from all layers to build global scales.
/// 3. **Injection**: The engine calls `inject_resolved_scales` to "back-fill" the final scales into the layer.
/// 4. **Rendering**: The engine calls `render_marks` to produce the final geometry.
pub trait Layer: MarkRenderer + Send + Sync {
    // --- Metadata Discovery Phase ---

    /// Returns true if this layer requires coordinate axes (X/Y) to be drawn.
    /// Some layers, like annotations or background fills, might not need them.
    fn requires_axes(&self) -> bool;

    /// Returns the data field name mapped to a specific visual channel (e.g., "horsepower" -> Color).
    fn get_field(&self, channel: Channel) -> Option<String>;

    /// Returns the preferred scale type (e.g., Linear, Log, Discrete) for a channel.
    fn get_scale(&self, channel: Channel) -> Option<Scale>;

    /// Returns any explicit user-defined domain override (e.g., fixed axis limits [0, 100]).
    fn get_domain(&self, channel: Channel) -> Option<ScaleDomain>;
    
    /// Returns the expansion rules (padding/margins) requested by this layer for a channel.
    fn get_expand(&self, channel: Channel) -> Option<Expansion>;

    /// Calculates the raw data boundaries (Min/Max for continuous, unique labels for discrete)
    /// contained within this specific layer's dataset.
    /// 
    /// This is the primary input for the "Training" phase where unified global scales are resolved.
    fn get_data_bounds(&self, channel: Channel) -> Result<ScaleDomain, ChartonError>;

    // --- State Resolution (The "Back-filling" Phase) ---

    /// Injects the resolved global state (Coordinate system and Aesthetic mappings) into the layer.
    /// 
    /// This ensures the layer has access to the final, unified scales before rendering starts.
    /// We use `&self` here; implementations typically use interior mutability (e.g., `OnceLock` 
    /// or `RwLock`) to cache these values safely.
    /// 
    /// # Arguments
    /// * `coord`: The shared coordinate system (containing X and Y scales).
    /// * `aesthetics`: The shared global aesthetics (containing Color, Shape, and Size scales).
    fn inject_resolved_scales(
        &self, 
        coord: Arc<dyn CoordinateTrait>, 
        aesthetics: &GlobalAesthetics
    );
}