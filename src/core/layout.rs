use super::context::SharedRenderingContext;
use super::legend::{LegendSpec, LegendPosition};
use crate::theme::Theme;

/// Represents the reserved space for axes on each side of the plot panel.
/// 
/// In the Grammar of Graphics, the 'Panel' is the pure data area. To ensure 
/// labels and titles do not clip, we calculate these pixel constraints 
/// during the layout phase and shrink the panel accordingly.
#[derive(Default, Debug, Clone, Copy)]
pub struct AxisLayoutConstraints {
    /// Height required for the horizontal axis (usually X, or Y if flipped).
    pub bottom: f64, 
    /// Width required for the vertical axis (usually Y, or X if flipped).
    pub left: f64,   
}

/// Internal helper structure representing the whitespace or reserved area 
/// on each side of the plot panel caused by legend placement.
#[derive(Default, Debug, Clone, Copy)]
pub struct LegendLayoutConstraints {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

/// The `LayoutEngine` is responsible for the geometric partitioning of the chart canvas.
/// 
/// It performs "pre-flight" measurements of non-data elements (axes, legends, titles)
/// to determine the final dimensions of the Plot Panel.
pub struct LayoutEngine;

impl LayoutEngine {
    /// Estimates the required margins for axes before rendering occurs.
    /// 
    /// This function accounts for:
    /// 1. The length of tick marks.
    /// 2. The bounding box of tick labels (considering rotation).
    /// 3. The padding and font size of the axis title.
    pub fn calculate_axis_constraints(
        ctx: &SharedRenderingContext,
        theme: &Theme,
        x_label: &str,
        y_label: &str,
    ) -> AxisLayoutConstraints {
        let mut constraints = AxisLayoutConstraints::default();
        let coord = ctx.coord;
        let is_flipped = coord.is_flipped();

        // 1. Calculate Physical Bottom Axis space.
        // If flipped, the physical bottom axis represents the Y data scale.
        let (bottom_scale, bottom_angle, bottom_title, bottom_padding) = if is_flipped {
            (coord.get_y_scale(), theme.y_tick_label_angle, y_label, theme.y_label_padding)
        } else {
            (coord.get_x_scale(), theme.x_tick_label_angle, x_label, theme.x_label_padding)
        };

        constraints.bottom = Self::estimate_axis_dimension(
            bottom_scale,
            bottom_angle,
            bottom_title,
            bottom_padding,
            theme,
            true // is_physically_bottom
        );

        // 2. Calculate Physical Left Axis space.
        // If flipped, the physical left axis represents the X data scale.
        let (left_scale, left_angle, left_title, left_padding) = if is_flipped {
            (coord.get_x_scale(), theme.x_tick_label_angle, x_label, theme.x_label_padding)
        } else {
            (coord.get_y_scale(), theme.y_tick_label_angle, y_label, theme.y_label_padding)
        };

        constraints.left = Self::estimate_axis_dimension(
            left_scale,
            left_angle,
            left_title,
            left_padding,
            theme,
            false // is_physically_bottom
        );

        constraints
    }

    /// Internal helper to calculate the depth (width or height) of an axis area.
    /// 
    /// It projects the rotated label bounds onto the axis normal to find the 
    /// maximum required clearance.
    fn estimate_axis_dimension(
        scale: &dyn crate::scale::ScaleTrait,
        angle_deg: f64,
        title: &str,
        label_padding: f64,
        theme: &Theme,
        is_physically_bottom: bool,
    ) -> f64 {
        let tick_line_len = 6.0;
        let safety_buffer = 5.0;
        let angle_rad = angle_deg.to_radians();
        let ticks = scale.ticks(8);

        // Calculate the maximum footprint (projection) of tick labels.
        let max_label_footprint = ticks.iter()
            .map(|t| {
                let w = estimate_text_width(&t.label, theme.tick_label_font_size);
                let h = theme.tick_label_font_size;
                if is_physically_bottom {
                    // For the bottom axis, height is the vertical projection.
                    w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
                } else {
                    // For the left axis, width is the horizontal projection.
                    w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
                }
            })
            .fold(0.0, f64::max);

        // Calculate space for the axis title if it exists.
        let title_area = if title.is_empty() {
            0.0
        } else {
            label_padding + theme.label_font_size + safety_buffer
        };

        tick_line_len + max_label_footprint + safety_buffer + title_area
    }

    /// Dynamically calculates the space required for legends.
    pub fn calculate_legend_constraints(
        specs: &[LegendSpec],
        position: LegendPosition,
        margin: f64,
        theme: &Theme,
    ) -> LegendLayoutConstraints {
        let mut constraints = LegendLayoutConstraints::default();
        if position == LegendPosition::None || specs.is_empty() {
            return constraints;
        }

        let font_size = theme.legend_font_size.unwrap_or(theme.tick_label_font_size);
        let font_family = theme.legend_font_family.as_ref().unwrap_or(&theme.tick_label_font_family);
        let width_factor = if font_family.contains("Mono") { 0.65 } else { 0.55 };

        match position {
            LegendPosition::Right | LegendPosition::Left => {
                let mut max_w = 0.0;
                for spec in specs {
                    // Estimation logic for legend items.
                    let title_w = spec.title.len() as f64 * (font_size * width_factor * 1.1);
                    max_w = f64::max(max_w, title_w + 40.0); // 40px buffer for markers
                }
                let total_needed = max_w + margin + 10.0;
                if position == LegendPosition::Right { constraints.right = total_needed; } 
                else { constraints.left = total_needed; }
            }
            LegendPosition::Top | LegendPosition::Bottom => {
                let total_needed = (font_size * 1.2) + (font_size + 20.0) + margin;
                if position == LegendPosition::Top { constraints.top = total_needed; } 
                else { constraints.bottom = total_needed; }
            }
            _ => {}
        }
        constraints
    }
}

/// Estimates text width using character categorization.
pub(crate) fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    let mut narrow_chars = 0;
    let mut uppercase_chars = 0;
    let mut other_chars = 0;

    for c in text.chars() {
        if matches!(c, '.'|','|':'|';'|'!'|'i'|'j'|'l'|'-'|'|'|'1'|'t'|'f'|'r') {
            narrow_chars += 1;
        } else if c.is_ascii_uppercase() {
            uppercase_chars += 1;
        } else {
            other_chars += 1;
        }
    }

    (narrow_chars as f64 * 0.3 + uppercase_chars as f64 * 0.65 + other_chars as f64 * 0.55) * font_size
}