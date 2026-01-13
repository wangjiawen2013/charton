use crate::core::layer::RenderBackend;
use std::fmt::Write;

/// SvgBackend implements the RenderBackend trait by writing 
/// SVG elements directly into a String buffer.
pub struct SvgBackend<'a> {
    /// A mutable reference to the string buffer where SVG tags are appended.
    pub buffer: &'a mut String,
}

impl<'a> SvgBackend<'a> {
    /// Creates a new SvgBackend instance.
    pub fn new(buffer: &'a mut String) -> Self {
        Self { buffer }
    }

    /// Internal helper to format color values for SVG.
    /// Handles "none" cases for fills or strokes.
    fn format_color(&self, color: &str) -> String {
        if color.is_empty() || color.to_lowercase() == "none" {
            "none".to_string()
        } else {
            color.to_string()
        }
    }
}

impl<'a> RenderBackend for SvgBackend<'a> {
    /// Draws a circle as an SVG <circle> element.
    fn draw_circle(&mut self, x: f64, y: f64, radius: f64, color: &str, opacity: f64) {
        let fill = self.format_color(color);
        let _ = writeln!(
            self.buffer,
            r#"<circle cx="{:.3}" cy="{:.3}" r="{:.3}" fill="{}" fill-opacity="{:.3}" />"#,
            x, y, radius, fill, opacity
        );
    }

    /// Draws a rectangle as an SVG <rect> element.
    fn draw_rect(&mut self, x: f64, y: f64, width: f64, height: f64, color: &str) {
        let fill = self.format_color(color);
        let _ = writeln!(
            self.buffer,
            r#"<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="{}" />"#,
            x, y, width, height, fill
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

    /// Draws complex shapes (Triangle, Star, Diamond) as an SVG <polygon> element.
    /// This is highly efficient as it reduces multiple draw calls into a single path.
    fn draw_polygon(&mut self, points: &[(f64, f64)], color: &str, opacity: f64) {
        let fill = self.format_color(color);
        let pts_str = points
            .iter()
            .map(|(px, py)| format!("{:.3},{:.3}", px, py))
            .collect::<Vec<_>>()
            .join(" ");

        let _ = writeln!(
            self.buffer,
            r#"<polygon points="{}" fill="{}" fill-opacity="{:.3}" />"#,
            pts_str, fill, opacity
        );
    }

    /// Extension: Implementation of stroke support for Marks.
    /// In a sophisticated SVG backend, we could combine fill and stroke into one tag.
    /// Here, we can add a specialized method or handle it within existing ones.
    fn draw_circle_with_stroke(
        &mut self,
        x: f64,
        y: f64,
        radius: f64,
        fill: &str,
        stroke: &str,
        stroke_width: f64,
        opacity: f64,
    ) {
        let f = self.format_color(fill);
        let s = self.format_color(stroke);
        let _ = writeln!(
            self.buffer,
            r#"<circle cx="{:.3}" cy="{:.3}" r="{:.3}" fill="{}" stroke="{}" stroke-width="{:.3}" fill-opacity="{:.3}" />"#,
            x, y, radius, f, s, stroke_width, opacity
        );
    }
}