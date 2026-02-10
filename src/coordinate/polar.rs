use super::{CoordinateTrait, CoordLayout, Rect};
use crate::scale::ScaleTrait;
use crate::visual::color::SingleColor;
use std::sync::Arc;
use std::f64::consts::PI;

/// A Polar coordinate system implementation.
/// 
/// In a Polar system:
/// - **X dimension** is mapped to the **Angle** (theta), typically [0, 2π].
/// - **Y dimension** is mapped to the **Radius** (r), typically [0, max_radius].
pub struct Polar {
    pub x_scale: Arc<dyn ScaleTrait>,
    pub y_scale: Arc<dyn ScaleTrait>,
    pub x_field: String,
    pub y_field: String,
    
    /// The starting angle in radians (default is -PI/2 to start from the top).
    pub start_angle: f64,
    /// The total angular span (default is 2π for a full circle).
    pub end_angle: f64,
    /// Inner radius ratio [0, 1]. Set > 0 for Donut charts.
    pub inner_radius: f64,
}

impl Polar {
    // Use a minimal constructor for essential scales, then apply geometric 
    // overrides to keep the API flexible and avoid parameter bloat.
    pub fn new(
        x_scale: Arc<dyn ScaleTrait>,
        y_scale: Arc<dyn ScaleTrait>,
        x_field: String,
        y_field: String,
    ) -> Self {
        Self {
            x_scale,
            y_scale,
            x_field,
            y_field,
            start_angle: -PI / 2.0, // Top center
            end_angle: 3.0 * PI / 2.0,
            inner_radius: 0.0,      // Default to Pie (not Donut)
        }
    }

    /// Internal helper to map normalized (x, y) to (theta, r)
    fn map_to_polar(&self, x_n: f64, y_n: f64) -> (f64, f64) {
        let theta = self.start_angle + x_n * (self.end_angle - self.start_angle);
        // Map Y norm to a radius between inner_radius and 1.0
        let r_norm = self.inner_radius + y_n * (1.0 - self.inner_radius);
        (theta, r_norm)
    }
}

impl CoordinateTrait for Polar {
    /// Transforms a single normalized point to pixel space.
    fn transform(&self, x_norm: f64, y_norm: f64, panel: &Rect) -> (f64, f64) {
        let (theta, r_norm) = self.map_to_polar(x_norm, y_norm);
        
        let center_x = panel.x + panel.width / 2.0;
        let center_y = panel.y + panel.height / 2.0;
        let max_r = panel.width.min(panel.height) / 2.0;
        
        let r_px = r_norm * max_r;
        
        let x_px = center_x + r_px * theta.cos();
        let y_px = center_y + r_px * theta.sin();
        
        (x_px, y_px)
    }

    /// Specialized method for drawing paths/polygons in Polar space.
    /// 
    /// Because a straight line in Cartesian space (e.g., the top of a bar) 
    /// becomes a curve in Polar space, we must interpolate points along 
    /// the X-axis (angular axis) to maintain the "circular" look.
    fn transform_path(&self, points: &[(f64, f64)], is_closed: bool, panel: &Rect) -> Vec<(f64, f64)> {
        if points.is_empty() { return vec![]; }
        
        // Pre-allocate space to minimize reallocations during adaptive path interpolation.
        // The actual number of points will be less or more.
        let mut result = Vec::with_capacity(points.len() * 4);
        let threshold = 0.01; 

        for i in 0..points.len() {
            let p1 = points[i];
            result.push(self.transform(p1.0, p1.1, panel));

            // Determine the next point to check for interpolation
            let next_point = if i + 1 < points.len() {
                Some(points[i + 1])
            } else if is_closed {
                // If closed, we must interpolate the segment from LAST point to FIRST point
                Some(points[0])
            } else {
                None
            };

            if let Some(p2) = next_point {
                // Adaptive interpolation logic...
                let dx = (p2.0 - p1.0).abs();
                if dx > threshold {
                    let steps = (dx / threshold).ceil() as usize;
                    for s in 1..steps {
                        let t = s as f64 / steps as f64;
                        result.push(self.transform(p1.0 + (p2.0 - p1.0) * t, p1.1 + (p2.1 - p1.1) * t, panel));
                    }
                }
            }
        }
        result
    }

    fn get_x_arc(&self) -> Arc<dyn ScaleTrait> { self.x_scale.clone() }
    fn get_y_arc(&self) -> Arc<dyn ScaleTrait> { self.y_scale.clone() }
    fn get_x_scale(&self) -> &dyn ScaleTrait { self.x_scale.as_ref() }
    fn get_y_scale(&self) -> &dyn ScaleTrait { self.y_scale.as_ref() }
    fn get_x_label(&self) -> &str { &self.x_field }
    fn get_y_label(&self) -> &str { &self.y_field }
    fn is_flipped(&self) -> bool { false }

    /// Returns layout hints optimized for radial/circular plots.
    fn layout_hints(&self) -> CoordLayout {
        CoordLayout {
            default_bar_stroke: SingleColor::new("white"),
            // Sectors occupy 100% of their angular slot to remain adjacent.
            default_bar_width: 1.0,
            
            // No spacing between sectors by default to maintain a solid circular shape.
            default_bar_spacing: 0.0,
            
            // The group spans the entire available angular step (full coverage).
            default_bar_span: 1.0,
            
            // Crucial: Straight horizontal edges in data space must be 
            // curved to follow the arc in Polar space.
            needs_interpolation: true,
        }
    }
}