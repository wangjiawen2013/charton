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
    /// Renders a circle. If fill/stroke is None, it is set to "none".
    fn draw_circle(
        &mut self,
        x: f64,
        y: f64,
        radius: f64,
        fill: Option<&str>,
        stroke: Option<&str>,
        stroke_width: f64,
        opacity: f64,
    ) {
        let f = fill.map(|c| self.format_color(c)).unwrap_or_else(|| "none".to_string());
        let s = stroke.map(|c| self.format_color(c)).unwrap_or_else(|| "none".to_string());
        
        let _ = writeln!(
            self.buffer,
            r#"<circle cx="{:.3}" cy="{:.3}" r="{:.3}" fill="{}" stroke="{}" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}" />"#,
            x, y, radius, f, s, stroke_width, opacity, opacity
        );
    }

    /// Renders a rectangle. Useful for bar charts or panel backgrounds.
    fn draw_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: Option<&str>,
        stroke: Option<&str>,
        stroke_width: f64,
        opacity: f64,
    ) {
        let f = fill.map(|c| self.format_color(c)).unwrap_or_else(|| "none".to_string());
        let s = stroke.map(|c| self.format_color(c)).unwrap_or_else(|| "none".to_string());

        let _ = writeln!(
            self.buffer,
            r#"<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="{}" stroke="{}" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}" />"#,
            x, y, width, height, f, s, stroke_width, opacity, opacity
        );
    }

    /// Renders a multi-point path. Commonly used for geom_line.
    fn draw_path(&mut self, points: &[(f64, f64)], stroke: &str, stroke_width: f64, opacity: f64) {
        if points.is_empty() { return; }
        
        let mut path_data = String::new();
        for (i, (px, py)) in points.iter().enumerate() {
            if i == 0 {
                let _ = write!(path_data, "M {:.3} {:.3}", px, py);
            } else {
                let _ = write!(path_data, " L {:.3} {:.3}", px, py);
            }
        }

        let _ = writeln!(
            self.buffer,
            r#"<path d="{}" stroke="{}" stroke-width="{:.3}" stroke-opacity="{:.3}" fill="none" stroke-linejoin="round" stroke-linecap="round" />"#,
            path_data, stroke, stroke_width, opacity
        );
    }

    /// Renders a closed polygon. Used for complex point shapes (triangles, diamonds) or area charts.
    fn draw_polygon(
        &mut self,
        points: &[(f64, f64)],
        fill: Option<&str>,
        stroke: Option<&str>,
        stroke_width: f64,
        opacity: f64,
    ) {
        if points.is_empty() { return; }
        
        let f = fill.map(|c| self.format_color(c)).unwrap_or_else(|| "none".to_string());
        let s = stroke.map(|c| self.format_color(c)).unwrap_or_else(|| "none".to_string());
        
        let pts_str = points
            .iter()
            .map(|(px, py)| format!("{:.3},{:.3}", px, py))
            .collect::<Vec<_>>()
            .join(" ");

        let _ = writeln!(
            self.buffer,
            r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}" />"#,
            pts_str, f, s, stroke_width, opacity, opacity
        );
    }

    /// Renders text. Handles basic XML escaping for safety.
    fn draw_text(
        &mut self,
        text: &str,
        x: f64,
        y: f64,
        font_size: f64,
        font_family: &str,
        color: &str,
        text_anchor: &str,
        font_weight: &str,
        opacity: f64,
    ) {
        let safe_text = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let _ = writeln!(
            self.buffer,
            r#"<text x="{:.3}" y="{:.3}" font-size="{:.1}" font-family="{}" fill="{}" fill-opacity="{:.3}" text-anchor="{}" font-weight="{}">{}</text>"#,
            x, y, font_size, font_family, color, opacity, text_anchor, font_weight, safe_text
        );
    }
}