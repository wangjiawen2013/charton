use crate::core::layer::RenderBackend;
use crate::coordinate::Rect;
use std::fmt::Write;

/// `SvgBackend` implements the `RenderBackend` trait by generating SVG XML 
/// elements directly into a String buffer.
///
/// It supports automatic clipping based on the plot's 'panel' area to ensure 
/// that data geometries do not bleed into the margins or axes.
pub struct SvgBackend<'a> {
    /// A mutable reference to the string buffer where SVG tags are appended.
    pub buffer: &'a mut String,
    
    /// An optional identifier for the clip-path used to constrain drawing 
    /// within the plot panel.
    clip_id: Option<String>,
}

impl<'a> SvgBackend<'a> {
    /// Creates a new `SvgBackend` and immediately defines a `<clipPath>` 
    /// if a panel is provided.
    ///
    /// # Arguments
    /// * `buffer` - The target string for SVG output.
    /// * `panel` - The physical rectangular area designated for data marks.
    pub fn new(buffer: &'a mut String, panel: Option<&Rect>) -> Self {
        let mut clip_id = None;

        if let Some(p) = panel {
            let id = "plot-clip-area".to_string();
            // Define the clipPath at the current position in the SVG.
            // This mask ensures that any element referencing this ID will only 
            // be visible within the specified rectangle.
            let _ = writeln!(
                buffer,
                r#"<defs><clipPath id="{}"><rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" /></clipPath></defs>"#,
                id, p.x, p.y, p.width, p.height
            );
            clip_id = Some(id);
        }

        Self { buffer, clip_id }
    }

    /// Internal helper to format color values, handling "none" transparent states.
    fn format_color(&self, color: &str) -> String {
        if color.is_empty() || color.to_lowercase() == "none" {
            "none".to_string()
        } else {
            color.to_string()
        }
    }

    /// Wraps the SVG element attributes with a reference to the clip-path if active.
    fn get_clip_attr(&self) -> String {
        match &self.clip_id {
            Some(id) => format!(r#" clip-path="url(#{})""#, id),
            None => "".to_string(),
        }
    }
}

impl<'a> RenderBackend for SvgBackend<'a> {
    /// Renders a circle, typically used for scatter plots (geom_point).
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
        let clip = self.get_clip_attr();
        
        let _ = writeln!(
            self.buffer,
            r#"<circle cx="{:.3}" cy="{:.3}" r="{:.3}" fill="{}" stroke="{}" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}"{} />"#,
            x, y, radius, f, s, stroke_width, opacity, opacity, clip
        );
    }

    /// Renders a rectangle, commonly used for bar charts (geom_bar).
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
        let clip = self.get_clip_attr();

        let _ = writeln!(
            self.buffer,
            r#"<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="{}" stroke="{}" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}"{} />"#,
            x, y, width, height, f, s, stroke_width, opacity, opacity, clip
        );
    }

    /// Renders a multi-point polyline, used for line charts (geom_line).
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

        let clip = self.get_clip_attr();
        let _ = writeln!(
            self.buffer,
            r#"<path d="{}" stroke="{}" stroke-width="{:.3}" stroke-opacity="{:.3}" fill="none" stroke-linejoin="round" stroke-linecap="round"{} />"#,
            path_data, stroke, stroke_width, opacity, clip
        );
    }

    /// Renders a closed polygon for shapes like triangles or area charts.
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
        let clip = self.get_clip_attr();
        
        let pts_str = points
            .iter()
            .map(|(px, py)| format!("{:.3},{:.3}", px, py))
            .collect::<Vec<_>>()
            .join(" ");

        let _ = writeln!(
            self.buffer,
            r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}"{} />"#,
            pts_str, f, s, stroke_width, opacity, opacity, clip
        );
    }

    /// Renders text with basic XML entity escaping for safe rendering in browsers.
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
        let clip = self.get_clip_attr();
        let _ = writeln!(
            self.buffer,
            r#"<text x="{:.3}" y="{:.3}" font-size="{:.1}" font-family="{}" fill="{}" fill-opacity="{:.3}" text-anchor="{}" font-weight="{}"{}>{}</text>"#,
            x, y, font_size, font_family, color, opacity, text_anchor, font_weight, clip, safe_text
        );
    }
}