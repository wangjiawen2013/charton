use super::{CoordinateTrait, Rect};
use crate::scale::ScaleTrait;

/// A 2D Cartesian coordinate system.
/// 
/// It maps normalized scale values [0, 1] onto a rectangular plane.
/// This implementation handles axis swapping (coord_flip) and 
/// the translation from mathematical space to screen space.
pub struct Cartesian2D {
    pub x_scale: Box<dyn ScaleTrait>,
    pub y_scale: Box<dyn ScaleTrait>,
    /// If true, the X and Y axes are swapped (equivalent to ggplot2's coord_flip).
    /// Data X maps to physical Height, Data Y maps to physical Width.
    pub flipped: bool,
}

impl Cartesian2D {
    /// Creates a new Cartesian system from two boxed scales.
    pub fn new(
        x_scale: Box<dyn ScaleTrait>,
        y_scale: Box<dyn ScaleTrait>,
        flipped: bool,
    ) -> Self {
        Self {
            x_scale,
            y_scale,
            flipped,
        }
    }
}

impl CoordinateTrait for Cartesian2D {
    /// Transforms logical data coordinates [0, 1] into physical screen pixels. 
    /// Use this for rendering Mark geometries; for Axis rendering, calculate positions 
    /// directly from the panel boundaries to ensure the visual frame remains fixed.
    /// 
    /// Following standard screen coordinates:
    /// - X increases from Left to Right.
    /// - Y increases from Top to Bottom (so we invert the normalized Y).
    fn transform(&self, x_norm: f64, y_norm: f64, panel: &Rect) -> (f64, f64) {
        let (mut x_p, mut y_p) = (x_norm, y_norm);

        // 1. Handle axis swapping (coord_flip)
        if self.flipped {
            std::mem::swap(&mut x_p, &mut y_p);
        }

        // 2. Map normalized ratio to physical pixels within the panel
        // x_pixel = panel_left + (ratio * panel_width)
        let final_x = panel.x + (x_p * panel.width);
        
        // 3. Invert Y-axis: 0.0 (min) should be at the bottom of the panel, 
        // 1.0 (max) should be at the top.
        let final_y = panel.y + ((1.0 - y_p) * panel.height);

        (final_x, final_y)
    }

    /// Returns references to the underlying scales.
    /// The renderer uses these to access domain info and generate ticks.
    fn get_x_scale(&self) -> &dyn ScaleTrait {
        self.x_scale.as_ref()
    }

    fn get_y_scale(&self) -> &dyn ScaleTrait {
        self.y_scale.as_ref()
    }

    fn is_flipped(&self) -> bool {
        self.flipped
    }

    /// Cartesian coordinates typically clip data that falls outside the panel.
    fn is_clipped(&self) -> bool {
        true
    }
}