use crate::core::context::SharedRenderingContext;
use crate::chart::Chart;
use crate::scale::Scale;
use crate::error::ChartonError;
use crate::mark::errorbar::MarkErrorBar;
use std::fmt::Write;

/// Extension implementation for `Chart` to support Error Bar Plots (MarkErrorBar).
impl Chart<MarkErrorBar> {
    /// Initializes a new `MarkErrorBar` layer.
    /// 
    /// If a mark configuration already exists, it is preserved; 
    /// otherwise, a new `MarkErrorBar` with default settings is created.
    pub fn mark_errorbar(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkErrorBar::default());
        }
        self
    }

    /// Configures the visual properties of the error bar mark using a closure.
    /// 
    /// # Example
    /// ```
    /// chart.mark_errorbar()
    ///      .configure_errorbar(|m| m.color("red").cap_length(5.0).show_center(true))
    /// ```
    pub fn configure_errorbar<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkErrorBar) -> MarkErrorBar 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }

    // Render all error bars for this chart
    fn render_errorbars(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        let processed_data =
            super::data_processor::ProcessedChartData::new(self, context.coord_system)?;

        let x_vals = &processed_data.x_transformed_vals;

        let (y_min_field, y_max_field) = if self.encoding.y2.is_some() {
            (
                self.encoding.y.as_ref().map(|y| y.field.clone()).unwrap_or_else(|| "y".to_string()),
                self.encoding.y2.as_ref().map(|y2| y2.field.clone()).unwrap_or_else(|| "y".to_string()),
            )
        } else {
            let y_field = self.encoding.y.as_ref().map(|y| y.field.clone()).unwrap_or_else(|| "y".to_string());
            (
                format!("__charton_temp_{}_min", y_field),
                format!("__charton_temp_{}_max", y_field),
            )
        };

        let y_min_series = self.data.column(&y_min_field)?;
        let y_max_series = self.data.column(&y_max_field)?;

        let (y_mean_vals, y_min_vals, y_max_vals) = {
            let y_mean_transformed: Vec<f32> = match context.coord_system.y_axis.scale {
                Scale::Log => {
                    y_min_series.f64()?.into_no_null_iter()
                        .zip(y_max_series.f64()?.into_no_null_iter())
                        .map(|(min, max)| ((min + max) / 2.0).log10() as f32)
                        .collect()
                }
                Scale::Linear | Scale::Discrete => {
                    y_min_series.f64()?.into_no_null_iter()
                        .zip(y_max_series.f64()?.into_no_null_iter())
                        .map(|(min, max)| ((min + max) / 2.0) as f32)
                        .collect()
                }
            };

            let y_min_transformed: Vec<f32> = match context.coord_system.y_axis.scale {
                Scale::Log => y_min_series.f64()?.into_no_null_iter().map(|y| y.log10() as f32).collect(),
                Scale::Linear | Scale::Discrete => y_min_series.f64()?.into_no_null_iter().map(|y| y as f32).collect(),
            };

            let y_max_transformed: Vec<f32> = match context.coord_system.y_axis.scale {
                Scale::Log => y_max_series.f64()?.into_no_null_iter().map(|y| y.log10() as f32).collect(),
                Scale::Linear | Scale::Discrete => y_max_series.f64()?.into_no_null_iter().map(|y| y as f32).collect(),
            };

            (y_mean_transformed, y_min_transformed, y_max_transformed)
        };

        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Internal("Mark should exist when rendering error bars".to_string())
        })?;

        let stroke_color = mark.color.get_color();

        for i in 0..x_vals.len() {
            let x_center_pixel = (context.x_mapper)(x_vals[i] as f64);
            let y_center_pixel = (context.y_mapper)(y_mean_vals[i] as f64);
            let y_min_pixel = (context.y_mapper)(y_min_vals[i] as f64);
            let y_max_pixel = (context.y_mapper)(y_max_vals[i] as f64);

            if !context.swapped_axes {
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    x_center_pixel, y_min_pixel, x_center_pixel, y_max_pixel,
                    stroke_color, mark.stroke_width, mark.opacity
                )?;

                if mark.show_center {
                    writeln!(
                        svg,
                        r#"<circle cx="{}" cy="{}" r="3" fill="{}" opacity="{}"/>"#,
                        x_center_pixel, y_center_pixel, stroke_color, mark.opacity
                    )?;
                }

                // Caps
                writeln!(svg, r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    x_center_pixel - mark.cap_length as f64, y_max_pixel, x_center_pixel + mark.cap_length as f64, y_max_pixel,
                    stroke_color, mark.stroke_width, mark.opacity)?;
                writeln!(svg, r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    x_center_pixel - mark.cap_length as f64, y_min_pixel, x_center_pixel + mark.cap_length as f64, y_min_pixel,
                    stroke_color, mark.stroke_width, mark.opacity)?;
            } else {
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    y_min_pixel, x_center_pixel, y_max_pixel, x_center_pixel,
                    stroke_color, mark.stroke_width, mark.opacity
                )?;

                if mark.show_center {
                    writeln!(
                        svg,
                        r#"<circle cx="{}" cy="{}" r="3" fill="{}" opacity="{}"/>"#,
                        y_center_pixel, x_center_pixel, stroke_color, mark.opacity
                    )?;
                }

                // Caps (Swapped)
                writeln!(svg, r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    y_max_pixel, x_center_pixel - mark.cap_length as f64, y_max_pixel, x_center_pixel + mark.cap_length as f64,
                    stroke_color, mark.stroke_width, mark.opacity)?;
                writeln!(svg, r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
                    y_min_pixel, x_center_pixel - mark.cap_length as f64, y_min_pixel, x_center_pixel + mark.cap_length as f64,
                    stroke_color, mark.stroke_width, mark.opacity)?;
            }
        }

        Ok(())
    }
}

impl MarkRenderer for Chart<MarkErrorBar> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_errorbars(svg, context)
    }
}

impl LegendRenderer for Chart<MarkErrorBar> {
    fn render_legends(
        &self,
        _svg: &mut String,
        _theme: &crate::theme::Theme,
        _context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        Ok(())
    }
}