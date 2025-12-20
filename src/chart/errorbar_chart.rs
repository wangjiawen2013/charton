use super::common::SharedRenderingContext;
use super::common::{Chart, LegendRenderer, MarkRenderer};
use crate::coord::Scale;
use crate::error::ChartonError;
use crate::mark::errorbar::MarkErrorBar;
use crate::visual::color::SingleColor;
use std::fmt::Write;

impl Chart<MarkErrorBar> {
    /// Create a new errorbar mark chart
    pub fn mark_errorbar(mut self) -> Self {
        self.mark = Some(MarkErrorBar::new());
        self
    }

    /// Set the color for error bars
    ///
    /// # Parameters
    /// * `color` - The color to apply to the error bars
    pub fn with_errorbar_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkErrorBar::new);
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity for error bars
    ///
    /// # Parameters
    /// * `opacity` - The opacity value (typically between 0.0 and 1.0)
    pub fn with_errorbar_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkErrorBar::new);
        mark.opacity = opacity;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke width for error bars
    ///
    /// # Parameters
    /// * `stroke_width` - The width of the error bar lines
    pub fn with_errorbar_stroke_width(mut self, stroke_width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkErrorBar::new);
        mark.stroke_width = stroke_width;
        self.mark = Some(mark);
        self
    }

    /// Set the cap length for error bars
    ///
    /// # Parameters
    /// * `cap_length` - The length of the horizontal caps at the ends of error bars
    pub fn with_errorbar_cap_length(mut self, cap_length: f64) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkErrorBar::new);
        mark.cap_length = cap_length;
        self.mark = Some(mark);
        self
    }

    /// Control whether to show center point on error bars
    ///
    /// # Parameters
    /// * `show` - Whether to display a marker at the center of each error bar
    pub fn with_errorbar_center(mut self, show: bool) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkErrorBar::new);
        mark.show_center = show;
        self.mark = Some(mark);
        self
    }

    // Render all error bars for this chart
    fn render_errorbars(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Process chart data using shared processor
        let processed_data =
            super::data_processor::ProcessedChartData::new(self, context.coord_system)?;

        // Extract data values from processed data
        let x_vals = &processed_data.x_transformed_vals;

        // Get the appropriate field name and min/max column names
        // For error bars, we always use the y encoding field since that's the continuous axis
        // where we show the min/max variation, regardless of visual orientation
        let (y_min_field, y_max_field) = if self.encoding.y2.is_some() {
            // When y2 encoding exists, use the explicit field names
            (
                self.encoding
                    .y
                    .as_ref()
                    .map(|y| y.field.clone())
                    .unwrap_or("y".to_string()),
                self.encoding
                    .y2
                    .as_ref()
                    .map(|y2| y2.field.clone())
                    .unwrap_or("y".to_string()),
            )
        } else {
            // When y2 encoding doesn't exist, use the auto-generated field names from transform_errorbar_data
            let y_field = self
                .encoding
                .y
                .as_ref()
                .map(|y| y.field.clone())
                .unwrap_or("y".to_string());
            (
                format!("__charton_temp_{}_min", y_field),
                format!("__charton_temp_{}_max", y_field),
            )
        };

        let y_min_series = self.data.column(&y_min_field)?;
        let y_max_series = self.data.column(&y_max_field)?;

        // Transform error bar limits according to appropriate axis scale
        // Always transform based on y-axis scale since error bars represent variation in y values
        let (y_mean_vals, y_min_vals, y_max_vals) = {
            let y_mean_transformed: Vec<f64> = match context.coord_system.y_axis.scale {
                Scale::Log => {
                    // Calculate arithmetic mean and then apply log transformation directly from series
                    y_min_series
                        .f64()?
                        .into_no_null_iter()
                        .zip(y_max_series.f64()?.into_no_null_iter())
                        .map(|(min, max)| ((min + max) / 2.0).log10())
                        .collect()
                }
                Scale::Linear | Scale::Discrete => {
                    // Calculate arithmetic mean without transformation for linear scale directly from series
                    y_min_series
                        .f64()?
                        .into_no_null_iter()
                        .zip(y_max_series.f64()?.into_no_null_iter())
                        .map(|(min, max)| (min + max) / 2.0)
                        .collect()
                }
            };

            let y_min_transformed: Vec<f64> = match context.coord_system.y_axis.scale {
                Scale::Log => y_min_series
                    .f64()?
                    .into_no_null_iter()
                    .map(|y| y.log10())
                    .collect(),
                Scale::Linear | Scale::Discrete => {
                    y_min_series.f64()?.into_no_null_iter().collect()
                }
            };

            let y_max_transformed: Vec<f64> = match context.coord_system.y_axis.scale {
                Scale::Log => y_max_series
                    .f64()?
                    .into_no_null_iter()
                    .map(|y| y.log10())
                    .collect(),
                Scale::Linear | Scale::Discrete => {
                    y_max_series.f64()?.into_no_null_iter().collect()
                }
            };

            (y_mean_transformed, y_min_transformed, y_max_transformed)
        };

        // Get mark properties
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Internal("Mark should exist when rendering error bars".to_string())
        })?;

        let stroke_color = mark
            .color
            .as_ref()
            .map(|c| c.get_color())
            .unwrap_or("none".to_string());

        // Render error bars
        for i in 0..x_vals.len() {
            // Position of center point - always use the mappers correctly based on what data they map
            let x_center_pixel = (context.x_mapper)(x_vals[i]); // x_mapper maps x data to screen x-coordinate
            let y_center_pixel = (context.y_mapper)(y_mean_vals[i]); // y_mapper maps y data to screen y-coordinate

            // For min/max values of error bars, we always use y_mapper because error bars
            // represent variation in the y values regardless of visual orientation
            let y_min_pixel = (context.y_mapper)(y_min_vals[i]); // Map min y-value to screen coordinate
            let y_max_pixel = (context.y_mapper)(y_max_vals[i]); // Map max y-value to screen coordinate

            if !context.swapped_axes {
                // Draw vertical error bars in normal orientation
                // Main vertical line
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    x_center_pixel,
                    y_min_pixel,
                    x_center_pixel,
                    y_max_pixel,
                    stroke_color,
                    mark.stroke_width,
                    mark.opacity
                )?;

                // Optionally draw center point
                if mark.show_center {
                    writeln!(
                        svg,
                        r#"<circle cx="{}" cy="{}" r="3" fill="{}" opacity="{}"/>"#,
                        x_center_pixel, y_center_pixel, stroke_color, mark.opacity
                    )?;
                }

                // Top cap (horizontal line at max_val)
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    x_center_pixel - mark.cap_length,
                    y_max_pixel,
                    x_center_pixel + mark.cap_length,
                    y_max_pixel,
                    stroke_color,
                    mark.stroke_width,
                    mark.opacity
                )?;

                // Bottom cap (horizontal line at min_val)
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    x_center_pixel - mark.cap_length,
                    y_min_pixel,
                    x_center_pixel + mark.cap_length,
                    y_min_pixel,
                    stroke_color,
                    mark.stroke_width,
                    mark.opacity
                )?;
            } else {
                // Draw horizontal error bars (axes swapped)
                // Main horizontal line
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    y_min_pixel,
                    x_center_pixel,
                    y_max_pixel,
                    x_center_pixel,
                    stroke_color,
                    mark.stroke_width,
                    mark.opacity
                )?;

                // Optionally draw center point - NOTE: when axes are swapped, x and y coordinates are swapped for the center point
                if mark.show_center {
                    writeln!(
                        svg,
                        r#"<circle cx="{}" cy="{}" r="3" fill="{}" opacity="{}"/>"#,
                        y_center_pixel,
                        x_center_pixel,
                        stroke_color,
                        mark.opacity // Swapped x and y coordinates
                    )?;
                }

                // Right cap (vertical line at max_val)
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    y_max_pixel,
                    x_center_pixel - mark.cap_length,
                    y_max_pixel,
                    x_center_pixel + mark.cap_length,
                    stroke_color,
                    mark.stroke_width,
                    mark.opacity
                )?;

                // Left cap (vertical line at min_val)
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    y_min_pixel,
                    x_center_pixel - mark.cap_length,
                    y_min_pixel,
                    x_center_pixel + mark.cap_length,
                    stroke_color,
                    mark.stroke_width,
                    mark.opacity
                )?;
            }
        }

        Ok(())
    }
}

// Implement MarkRenderer for Chart<MarkErrorBar>
impl MarkRenderer for Chart<MarkErrorBar> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_errorbars(svg, context)
    }
}

// Implement LegendRenderer for Chart<MarkErrorBar>
impl LegendRenderer for Chart<MarkErrorBar> {
    fn render_legends(
        &self,
        _svg: &mut String,
        _theme: &crate::theme::Theme,
        _context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Error bar charts typically don't have complex legends like point charts
        // Only basic mark properties (color, stroke width) which are usually consistent
        // across all error bars, so no legend rendering is needed
        Ok(())
    }
}
