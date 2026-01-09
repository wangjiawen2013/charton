use super::CoordinateTrait;
use crate::scale::ScaleTrait;

/// A 2D Cartesian coordinate system.
/// Maps independent X and Y scales onto a rectangular plane.
pub struct Cartesian2D {
    pub x_scale: Box<dyn ScaleTrait>,
    pub y_scale: Box<dyn ScaleTrait>,
    /// If true, data X maps to physical Y, and data Y maps to physical X.
    /// Primarily used for horizontal bar, histogram, boxplot charts.
    pub swapped: bool,
}

impl Cartesian2D {
    /// Creates a new Cartesian system from two boxed scales.
    pub fn new(
        x_scale: Box<dyn ScaleTrait>,
        y_scale: Box<dyn ScaleTrait>,
        swapped: bool,
    ) -> Self {
        Self {
            x_scale,
            y_scale,
            swapped,
        }
    }
}

impl CoordinateTrait for Cartesian2D {
    fn convert(&self, x_val: f64, y_val: f64) -> (f64, f64) {
        if self.swapped {
            // Swap logic: Data X -> Vertical, Data Y -> Horizontal
            (self.y_scale.map(y_val), self.x_scale.map(x_val))
        } else {
            // Standard: Data X -> Horizontal, Data Y -> Vertical
            (self.x_scale.map(x_val), self.y_scale.map(y_val))
        }
    }

    fn get_ranges(&self) -> ((f64, f64), (f64, f64)) {
        (self.x_scale.range(), self.y_scale.range())
    }
}