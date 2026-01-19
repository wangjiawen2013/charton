use crate::coordinate::{CoordinateTrait, Rect};
use crate::core::aesthetics::GlobalAesthetics;
use crate::core::guide::LegendPosition;

/// `SharedRenderingContext` provides the environmental data and transformation tools 
/// required by any `Layer` to render its content.
///
/// It encapsulates the coordinate system logic, the physical drawing area (panel),
/// and a reference to the global aesthetic mappings.
pub struct SharedRenderingContext<'a> {
    /// The coordinate system used to map normalized values [0, 1] to screen pixels.
    pub coord: &'a dyn CoordinateTrait,

    /// The physical rectangular area (in pixels) designated for the plot.
    pub panel: Rect,

    pub legend_position: LegendPosition,
    pub legend_margin: f64,

    /// Refers to global aesthetic rules (Color, Shape, Size).
    /// Using a reference '&'a' avoids the need for expensive or impossible Clones 
    /// of Trait Objects (Box<dyn ScaleTrait>).
    pub aesthetics: &'a GlobalAesthetics,
}

impl<'a> SharedRenderingContext<'a> {
    /// Creates a new shared rendering context by borrowing components.
    pub fn new(
        coord: &'a dyn CoordinateTrait,
        panel: Rect,
        legend_position: LegendPosition,
        legend_margin: f64,
        aesthetics: &'a GlobalAesthetics,
    ) -> Self {
        Self {
            coord,
            panel,
            legend_position,
            legend_margin,
            aesthetics,
        }
    }

    /// Transforms normalized data coordinates (range [0, 1]) to absolute canvas pixel coordinates.
    ///
    /// # Arguments
    /// * `x_norm` - The normalized X value.
    /// * `y_norm` - The normalized Y value.
    ///
    /// # Returns
    /// A tuple `(x_pixel, y_pixel)`.
    pub fn transform(&self, x_norm: f64, y_norm: f64) -> (f64, f64) {
        self.coord.transform(x_norm, y_norm, &self.panel)
    }

    /// Transforms only the normalized X coordinate to a pixel X position.
    ///
    /// Useful for drawing vertical elements like grid lines or X-axis ticks.
    pub fn x_to_px(&self, x_norm: f64) -> f64 {
        // We pass 0.0 for Y as it doesn't affect the X result in Cartesian systems.
        self.transform(x_norm, 0.0).0
    }

    /// Transforms only the normalized Y coordinate to a pixel Y position.
    ///
    /// Useful for drawing horizontal elements like grid lines or Y-axis ticks.
    pub fn y_to_px(&self, y_norm: f64) -> f64 {
        // We pass 0.0 for X as it doesn't affect the Y result in Cartesian systems.
        self.transform(0.0, y_norm).1
    }

    /// Returns the width of the plotting panel in pixels.
    pub fn width(&self) -> f64 {
        self.panel.width
    }

    /// Returns the height of the plotting panel in pixels.
    pub fn height(&self) -> f64 {
        self.panel.height
    }

    /// Returns the left-most pixel coordinate of the panel.
    pub fn x0(&self) -> f64 {
        self.panel.x
    }

    /// Returns the top-most pixel coordinate of the panel.
    pub fn y0(&self) -> f64 {
        self.panel.y
    }
}