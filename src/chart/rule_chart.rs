use super::common::SharedRenderingContext;
use super::common::{Chart, LegendRenderer, MarkRenderer};
use super::data_processor::ProcessedChartData;
use crate::coord::Scale;
use crate::error::ChartonError;
use crate::mark::rule::MarkRule;
use crate::render::color_legend_renderer;
use crate::render::colorbar_renderer;
use crate::render::rule_renderer;
use crate::visual::color::SingleColor;

impl Chart<MarkRule> {
    /// Create a new rule mark chart
    ///
    /// Initializes the chart with a `MarkRule` instance, enabling rule/line rendering.
    /// Rule charts are used to display vertical or horizontal lines that can represent
    /// thresholds, ranges, or connections between data points.
    pub fn mark_rule(mut self) -> Self {
        self.mark = Some(MarkRule::new());
        self
    }

    /// Set the color for the rule line
    ///
    /// Configures the color used to draw the rule lines. If not set, the system will use
    /// palette colors based on groupings or a default color.
    ///
    /// # Arguments
    /// * `color` - A `SingleColor` specifying the rule line color
    pub fn with_rule_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity for the rule line
    ///
    /// Adjusts the transparency of the rule lines. Values range from 0.0 (fully transparent)
    /// to 1.0 (fully opaque). Useful for overlapping lines or emphasizing certain data.
    ///
    /// # Arguments
    /// * `opacity` - A `f64` value between 0.0 and 1.0 representing the rule line opacity
    pub fn with_rule_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.opacity = opacity;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke width for the rule line
    ///
    /// Controls the thickness of the rule lines in pixels. Thicker lines are more visible
    /// but may obscure other chart elements.
    ///
    /// # Arguments
    /// * `stroke_width` - A `f64` value representing the stroke width in pixels
    pub fn with_rule_stroke_width(mut self, stroke_width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke_width = stroke_width;
        self.mark = Some(mark);
        self
    }

    // Render rule lines
    fn render_rules(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Process chart data using shared processor
        let processed_data = ProcessedChartData::new(self, context.coord_system)?;

        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Internal("Mark should exist when rendering rules".to_string())
        })?;

        // Extract data values from processed data
        let x_vals = &processed_data.x_transformed_vals;
        let y_vals = &processed_data.y_transformed_vals;

        // Check if we have y2 values
        let y2_vals = if let Some(y2_encoding) = &self.encoding.y2 {
            let y2_series = self.data.column(&y2_encoding.field)?;
            let y2_vals_raw: Vec<f64> = y2_series.f64()?.into_no_null_iter().collect();

            // Transform y2 values according to y-axis scale
            let y2_vals_transformed: Vec<f64> = match context.coord_system.y_axis.scale {
                Scale::Log => y2_vals_raw.iter().map(|&y| y.log10()).collect(),
                Scale::Linear | Scale::Discrete => y2_vals_raw, // No transformation
            };

            Some(y2_vals_transformed)
        } else {
            None
        };

        // Render rules for each data point
        for i in 0..x_vals.len() {
            // Determine stroke color properties
            let stroke_color = if let Some(ref color_info) = processed_data.color_info {
                let (ref scale, ref color_vals) = *color_info;
                let &color_val = &color_vals[i];
                match scale {
                    Scale::Discrete => {
                        // For discrete scales, use palette colors
                        Some(SingleColor::new(
                            &self.mark_palette.get_color(color_val as usize),
                        ))
                    }
                    Scale::Linear | Scale::Log => {
                        // For continuous scales, use colormap
                        Some(SingleColor::new(&self.mark_cmap.get_color(color_val)))
                    }
                }
            } else {
                mark.color.clone()
            };

            let x_pos = (context.x_mapper)(x_vals[i]);
            let y_pos = (context.y_mapper)(y_vals[i]);

            if !context.swapped_axes {
                if let Some(ref y2_vals_data) = y2_vals {
                    // Draw vertical rule line from y to y2
                    let y2_pos = (context.y_mapper)(y2_vals_data[i]);
                    rule_renderer::render_vertical_rule(
                        svg,
                        rule_renderer::VerticalRuleConfig {
                            x: x_pos,
                            y1: y_pos,
                            y2: y2_pos,
                            stroke_color: stroke_color.clone(),
                            stroke_width: mark.stroke_width,
                            opacity: mark.opacity,
                        },
                    )?;
                } else {
                    // Draw vertical rule line (from top to bottom of plot area)
                    rule_renderer::render_vertical_rule(
                        svg,
                        rule_renderer::VerticalRuleConfig {
                            x: x_pos,
                            y1: context.draw_y0,
                            y2: context.draw_y0 + context.plot_height,
                            stroke_color: stroke_color.clone(),
                            stroke_width: mark.stroke_width,
                            opacity: mark.opacity,
                        },
                    )?;
                }
            } else if let Some(ref y2_vals_data) = y2_vals {
                // Draw horizontal rule line from y to y2 (appears vertical in swapped axes)
                let y2_pos = (context.x_mapper)(y2_vals_data[i]);
                rule_renderer::render_horizontal_rule(
                    svg,
                    rule_renderer::HorizontalRuleConfig {
                        x1: y_pos,
                        x2: y2_pos,
                        y: x_pos, // This is the y-coordinate for the horizontal line
                        stroke_color: stroke_color.clone(),
                        stroke_width: mark.stroke_width,
                        opacity: mark.opacity,
                    },
                )?;
            } else {
                // When axes are swapped, vertical and horizontal lines are swapped too
                // Draw horizontal rule line (appears vertical in swapped axes)
                rule_renderer::render_horizontal_rule(
                    svg,
                    rule_renderer::HorizontalRuleConfig {
                        x1: context.draw_x0,
                        x2: context.draw_x0 + context.plot_width,
                        y: x_pos,
                        stroke_color: stroke_color.clone(),
                        stroke_width: mark.stroke_width,
                        opacity: mark.opacity,
                    },
                )?;
            }
        }

        Ok(())
    }
}

// Implement MarkRenderer for Chart<MarkRule>
impl MarkRenderer for Chart<MarkRule> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_rules(svg, context)
    }
}

// Implement LegendRenderer for Chart<MarkRule>
impl LegendRenderer for Chart<MarkRule> {
    fn render_legends(
        &self,
        svg: &mut String,
        theme: &crate::theme::Theme,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Render colorbar if needed
        colorbar_renderer::render_colorbar(svg, self, theme, context)?;

        // Render color legend if needed
        color_legend_renderer::render_color_legend(svg, self, theme, context)?;

        Ok(())
    }
}
