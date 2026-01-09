pub mod cartesian;

/// The core interface for all coordinate systems.
/// This allows the renderer to work with any coordinate system (Cartesian, Polar, etc.)
/// without knowing its internal mapping math.
pub trait CoordinateTrait {
    /// Transforms a pair of data values into absolute pixel coordinates.
    /// Returns (x_pixel, y_pixel).
    fn convert(&self, x_val: f64, y_val: f64) -> (f64, f64);

    /// Returns the active physical boundaries (x_range, y_range).
    /// Essential for drawing axes, grids, and background elements.
    fn get_ranges(&self) -> ((f64, f64), (f64, f64));
}