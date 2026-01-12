pub mod cartesian;

use crate::scale::ScaleTrait;

/// A simple rectangle representing a physical area on the canvas.
/// This defines where the coordinate system is allowed to draw.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }
}

/// The core interface for all coordinate systems in Charton.
/// 
/// Following the ggplot2 philosophy, a Coordinate System is responsible for:
/// 1. Mapping normalized data [0, 1] into screen pixels.
/// 2. Defining the shape of the plotting area (Cartesian, Polar, etc.).
/// 3. Providing metadata for rendering axes and grids.
pub trait CoordinateTrait {
    /// Transforms normalized data values into absolute pixel coordinates.
    /// 
    /// # Arguments
    /// * `x_norm` - A value from 0.0 to 1.0 (usually from x_scale.normalize).
    /// * `y_norm` - A value from 0.0 to 1.0 (usually from y_scale.normalize).
    /// * `panel` - The physical rectangular area available for drawing.
    /// 
    /// # Returns
    /// A tuple of (x_pixel, y_pixel).
    fn transform(&self, x_norm: f64, y_norm: f64, panel: &Rect) -> (f64, f64);

    /// Get the scale for the first dimension (e.g., X).
    fn get_x_scale(&self) -> &dyn ScaleTrait;

    /// Get the scale for the second dimension (e.g., Y).
    fn get_y_scale(&self) -> &dyn ScaleTrait;

    /// If true, the renderer should clip shapes extending beyond panel boundaries.
    fn is_clipped(&self) -> bool {
        true
    }
}

/// Supported coordinate systems for the chart.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CoordSystem {
    /// Standard 2D Cartesian coordinates (X and Y axes).
    #[default]
    Cartesian2D,
    /// Polar coordinates (Radius and Angle).
    Polar,
}