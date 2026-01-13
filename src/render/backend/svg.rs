use std::fmt::Write;
use crate::core::layer::RenderBackend;

/// A backend implementation that generates SVG strings.
pub struct SvgBackend<'a> {
    /// A mutable reference to the SVG string buffer.
    buffer: &'a mut String,
}

impl<'a> SvgBackend<'a> {
    /// Creates a new SVG backend.
    pub fn new(buffer: &'a mut String) -> Self {
        Self { buffer }
    }
}

impl<'a> RenderBackend for SvgBackend<'a> {
    fn draw_circle(&mut self, x: f64, y: f64, radius: f64, color: &str, opacity: f64) {
        let _ = writeln!(
            self.buffer,
            r#"<circle cx="{:.3}" cy="{:.3}" r="{:.3}" fill="{}" fill-opacity="{:.3}" />"#,
            x, y, radius, color, opacity
        );
    }

    fn draw_rect(&mut self, x: f64, y: f64, width: f64, height: f64, color: &str) {
        let _ = writeln!(
            self.buffer,
            r#"<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="{}" />"#,
            x, y, width, height, color
        );
    }

    fn draw_path(&mut self, points: &[(f64, f64)], stroke_color: &str, stroke_width: f64) {
        if points.is_empty() { return; }
        
        let mut path_data = String::new();
        for (i, (px, py)) in points.iter().enumerate() {
            if i == 0 {
                write!(path_data, "M {:.3} {:.3}", px, py).unwrap();
            } else {
                write!(path_data, " L {:.3} {:.3}", px, py).unwrap();
            }
        }

        let _ = writeln!(
            self.buffer,
            r#"<path d="{}" stroke="{}" stroke-width="{:.3}" fill="none" />"#,
            path_data, stroke_color, stroke_width
        );
    }
}