use crate::core::layer::RenderBackend;
use crate::coordinate::Rect;
use crate::prelude::SingleColor;
use std::fmt::Write;

/// `SvgBackend` implements the `RenderBackend` trait by generating SVG XML 
/// elements directly into a String buffer.
pub struct SvgBackend<'a> {
    /// A mutable reference to the string buffer where SVG tags are appended.
    pub buffer: &'a mut String,
    
    /// An optional identifier for the clip-path.
    clip_id: Option<String>,
}

impl<'a> SvgBackend<'a> {
    pub fn new(buffer: &'a mut String, panel: Option<&Rect>) -> Self {
        let mut clip_id = None;
        if let Some(p) = panel {
            let id = "plot-clip-area".to_string();
            let _ = writeln!(
                buffer,
                r#"<defs><clipPath id="{}"><rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" /></clipPath></defs>"#,
                id, p.x, p.y, p.width, p.height
            );
            clip_id = Some(id);
        }
        Self { buffer, clip_id }
    }

    fn get_clip_attr(&self) -> String {
        match &self.clip_id {
            Some(id) => format!(r#" clip-path="url(#{})""#, id),
            None => "".to_string(),
        }
    }
}

impl<'a> RenderBackend for SvgBackend<'a> {
    fn draw_circle(
        &mut self,
        x: f64, y: f64, radius: f64,
        fill: &SingleColor,
        stroke: &SingleColor,
        stroke_width: f64,
        opacity: f64,
    ) {
        // Direct call to as_str() - SingleColor already knows if it's "none"
        if fill.is_none() && stroke.is_none() { return; }

        let _ = writeln!(
            self.buffer,
            r#"<circle cx="{:.3}" cy="{:.3}" r="{:.3}" fill="{}" stroke="{}" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}"{} />"#,
            x, y, radius, fill.as_str(), stroke.as_str(), stroke_width, opacity, opacity, self.get_clip_attr()
        );
    }

    fn draw_rect(
        &mut self,
        x: f64, y: f64, width: f64, height: f64,
        fill: &SingleColor,
        stroke: &SingleColor,
        stroke_width: f64,
        opacity: f64,
    ) {
        if fill.is_none() && stroke.is_none() { return; }

        let _ = writeln!(
            self.buffer,
            r#"<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="{}" stroke="{}" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}"{} />"#,
            x, y, width, height, fill.as_str(), stroke.as_str(), stroke_width, opacity, opacity, self.get_clip_attr()
        );
    }

    fn draw_path(&mut self, points: &[(f64, f64)], stroke: &SingleColor, stroke_width: f64, opacity: f64) {
        if points.is_empty() || stroke.is_none() { return; }
        
        let mut path_data = String::new();
        for (i, (px, py)) in points.iter().enumerate() {
            if i == 0 { write!(path_data, "M {:.3} {:.3}", px, py).unwrap(); }
            else { write!(path_data, " L {:.3} {:.3}", px, py).unwrap(); }
        }

        let _ = writeln!(
            self.buffer,
            r#"<path d="{}" stroke="{}" stroke-width="{:.3}" stroke-opacity="{:.3}" fill="none" stroke-linejoin="round" stroke-linecap="round"{} />"#,
            path_data, stroke.as_str(), stroke_width, opacity, self.get_clip_attr()
        );
    }

    fn draw_polygon(
        &mut self,
        points: &[(f64, f64)],
        fill: &SingleColor,
        stroke: &SingleColor,
        stroke_width: f64,
        opacity: f64,
    ) {
        if points.is_empty() || (fill.is_none() && stroke.is_none()) { return; }
        
        let pts_str = points.iter()
            .map(|(px, py)| format!("{:.3},{:.3}", px, py))
            .collect::<Vec<_>>().join(" ");

        let _ = writeln!(
            self.buffer,
            r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{:.3}" fill-opacity="{:.3}" stroke-opacity="{:.3}"{} />"#,
            pts_str, fill.as_str(), stroke.as_str(), stroke_width, opacity, opacity, self.get_clip_attr()
        );
    }

    fn draw_text(
        &mut self,
        text: &str,
        x: f64, y: f64,
        font_size: f64,
        font_family: &str,
        color: &SingleColor,
        text_anchor: &str,
        font_weight: &str,
        opacity: f64,
    ) {
        if color.is_none() { return; }
        let safe_text = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        
        let _ = writeln!(
            self.buffer,
            r#"<text x="{:.3}" y="{:.3}" font-size="{:.1}" font-family="{}" fill="{}" fill-opacity="{:.3}" text-anchor="{}" font-weight="{}"{}>{}</text>"#,
            x, y, font_size, font_family, color.as_str(), opacity, text_anchor, font_weight, self.get_clip_attr(), safe_text
        );
    }

    fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, color: &SingleColor, width: f64) {
        if color.is_none() { return; }
        let _ = writeln!(
            self.buffer,
            r#"<line x1="{:.3}" y1="{:.3}" x2="{:.3}" y2="{:.3}" stroke="{}" stroke-width="{:.3}" />"#,
            x1, y1, x2, y2, color.as_str(), width
        );
    }

    fn draw_gradient_rect(
        &mut self,
        x: f64, y: f64, width: f64, height: f64,
        stops: &[(f64, SingleColor)],
        is_vertical: bool,
        id_suffix: &str,
    ) {
        let grad_id = format!("grad_{}", id_suffix);
        let (x2, y2) = if is_vertical { ("0%", "100%") } else { ("100%", "0%") };

        let _ = writeln!(self.buffer, r#"<defs><linearGradient id="{}" x1="0%" y1="0%" x2="{}" y2="{}">"#, grad_id, x2, y2);
        for (offset, color) in stops {
            let _ = writeln!(self.buffer, r#"  <stop offset="{:.1}%" stop-color="{}"/>"#, offset * 100.0, color.as_str());
        }
        let _ = writeln!(self.buffer, "</linearGradient></defs>");

        let _ = writeln!(self.buffer, r#"<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="url('#{}')"/>"#, x, y, width, height, grad_id);
    }
}