use crate::coordinate::{CoordinateTrait, Rect};

/// `SharedRenderingContext` provides the necessary tools for a `Layer` to draw itself.
/// 
/// It encapsulates the coordinate system logic and the physical drawing area (panel).
/// By using this context, a Layer doesn't need to know its absolute pixel position;
/// it only needs to map normalized data to the provided coordinate system.
pub struct SharedRenderingContext<'a> {
    /// The coordinate system responsible for mapping [0, 1] values to pixels.
    /// It can be Cartesian, Polar, or any other implementation of CoordinateTrait.
    pub coord: &'a dyn CoordinateTrait,

    /// The physical rectangle representing the plotting area inside the axes.
    pub panel: Rect,

    /// Flag indicating if the axes are swapped (e.g., for horizontal bar charts).
    pub swapped_axes: bool,

    /// Global configuration to toggle legend visibility.
    pub show_legend: bool,
}

impl<'a> SharedRenderingContext<'a> {
    /// Creates a new rendering context.
    pub fn new(
        coord: &'a dyn CoordinateTrait,
        panel: Rect,
        swapped_axes: bool,
        show_legend: bool,
    ) -> Self {
        Self {
            coord,
            panel,
            swapped_axes,
            show_legend,
        }
    }

    /// Convenience method to transform normalized data coordinates [0, 1] 
    /// to absolute canvas pixel coordinates.
    pub fn transform(&self, x_norm: f64, y_norm: f64) -> (f64, f64) {
        self.coord.transform(x_norm, y_norm, &self.panel)
    }
}