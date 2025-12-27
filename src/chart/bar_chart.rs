use crate::chart::common::{Chart, SharedRenderingContext};
use crate::chart::data_processor::ProcessedChartData;
use crate::error::ChartonError;
use crate::mark::bar::MarkBar;
use crate::render::color_legend_renderer;
use crate::theme::Theme;
use crate::visual::color::SingleColor;
use polars::prelude::*;
use std::fmt::Write;

// Implementation specific to MarkBar with additional methods
impl Chart<MarkBar> {
    /// Create a new bar mark chart
    ///
    /// Initializes a chart with bar marks for data visualization. This is the starting point
    /// for creating bar charts, which can be further customized with various styling options.
    ///
    /// # Returns
    ///
    /// Returns the chart instance with bar mark initialized
    pub fn mark_bar(mut self) -> Self {
        self.mark = Some(MarkBar::new());
        self
    }

    /// Set the fill color for bars
    ///
    /// Defines the color used to fill the bar shapes. If None is provided, the chart
    /// will use colors from the default palette to distinguish different data groups.
    ///
    /// # Arguments
    ///
    /// * `color` - Optional SingleColor to fill the bars. If None, palette colors are used
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_bar_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity for bars
    ///
    /// Controls the transparency level of the filled bars. Values range from 0.0 (fully transparent)
    /// to 1.0 (fully opaque). Useful for overlapping bars or creating visual effects.
    ///
    /// # Arguments
    ///
    /// * `opacity` - A f64 value between 0.0 and 1.0 representing the opacity level
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_bar_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.opacity = opacity;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke color for bars
    ///
    /// Defines the color of the outline/border around bar shapes. If None is provided,
    /// no stroke will be drawn around the bars.
    ///
    /// # Arguments
    ///
    /// * `stroke` - Optional SingleColor for the bar outlines. If None, no stroke is applied
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_bar_stroke(mut self, stroke: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke = stroke;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke width for bars
    ///
    /// Controls the thickness of the outline/border around bar shapes. Larger values
    /// create more prominent borders, while smaller values create thinner ones.
    ///
    /// # Arguments
    ///
    /// * `stroke_width` - A f64 value representing the width of the stroke in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_bar_stroke_width(mut self, stroke_width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke_width = stroke_width;
        self.mark = Some(mark);
        self
    }

    /// Set the width for bars
    ///
    /// Defines the width of individual bars in data units. This value is used as the base width
    /// for calculating actual bar widths, which may be adjusted based on grouping and spacing.
    ///
    /// # Arguments
    ///
    /// * `width` - A f64 value representing the base width of bars in data units
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_bar_width(mut self, width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.width = width;
        self.mark = Some(mark);
        self
    }

    /// Set the spacing between bars in a group
    ///
    /// Controls the spacing between bars within the same group as a ratio of bar width.
    /// A value of 0.0 means no spacing, while 1.0 means spacing equal to bar width.
    ///
    /// # Arguments
    ///
    /// * `spacing` - A f64 value representing spacing ratio (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_bar_spacing(mut self, spacing: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.spacing = spacing;
        self.mark = Some(mark);
        self
    }

    /// Set the total span for each group of bars
    ///
    /// Defines the total width allocated for each group of bars in data units. This value
    /// determines how much space is reserved for all bars within a group, including spacing.
    ///
    /// # Arguments
    ///
    /// * `span` - A f64 value representing the total span for each bar group in data units
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_bar_span(mut self, span: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.span = span;
        self.mark = Some(mark);
        self
    }

    // Render bar elements with grouping support
    fn render_bars(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Process chart data using shared processor
        let processed_data = ProcessedChartData::new(self, context.coord_system)?;

        // Extract the mark from Option
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Internal("Mark should exist when rendering bars".to_string())
        })?;

        // Get bar configuration
        let user_width = mark.width;
        let spacing = mark.spacing; // Spacing between bars in a group
        let span = mark.span;

        // Check if we should stack bars
        let should_stack = self.encoding.y.as_ref().map(|y| y.stack).ok_or_else(|| {
            ChartonError::Internal("Y encoding should exist when rendering bars".to_string())
        })?;

        // Create or get color column data
        let color_series = if let Some(color_enc) = &self.encoding.color {
            let series = self.data.column(&color_enc.field)?;
            series.clone()
        } else {
            // Create a temporary color column with default value
            let len = processed_data.x_transformed_vals.len();
            let default_colors: Vec<&str> = vec!["group"; len];
            Series::new("color".into(), default_colors)
        };

        // Create a DataFrame for grouping by the group axis values
        let (group_vals, value_vals) = (
            &processed_data.x_transformed_vals,
            &processed_data.y_transformed_vals,
        );

        let group_series = Series::new("group".into(), group_vals);
        let value_series = Series::new("value".into(), value_vals);
        // There is a common issue with Polars that column names might get lost or
        // altered during certain operations. "color".into() is "\"color\" acutally,
        // so we need to use the following workaround
        let color_str_series = color_series.str()?;
        let color_named_series = color_str_series.clone().with_name("color".into());

        let df = DataFrame::new(vec![
            group_series.into(),
            value_series.into(),
            color_named_series.into_series().into(),
        ])?;

        // Group by group values
        let groups = df.partition_by_stable(["group"], true)?;

        // Render bars for each group
        for group_df in groups.iter() {
            let group_val = group_df.column("group")?.f64()?.get(0).unwrap();

            // Get all bars in this group (different colors)
            let color_col = group_df.column("color")?;
            let value_col = group_df.column("value")?;

            let colors: Vec<String> = color_col
                .str()?
                .into_no_null_iter()
                .map(|s| s.to_string())
                .collect();
            let values: Vec<f64> = value_col.f64()?.into_no_null_iter().collect();

            if should_stack {
                // Render stacked bars
                // Use 1.0 - spacing * 0.5 as the width for stacked bars
                let stacked_bar_width = 1.0 - spacing * 0.5;

                // Convert stacked_bar_width (data value) to bar_width_pixels (pixel value)
                let base_position = (context.x_mapper)(0.0);
                let bar_width_position = (context.x_mapper)(stacked_bar_width);
                let bar_width_pixels = (bar_width_position - base_position).abs();

                // Calculate cumulative values for stacking
                let mut cumulative_values: Vec<f64> = Vec::with_capacity(values.len() + 1);
                let mut cum_sum = 0.0;
                cumulative_values.push(cum_sum);
                for &value in &values {
                    cum_sum += value;
                    cumulative_values.push(cum_sum);
                }

                // Position of the group center on group axis
                let group_center = (context.x_mapper)(group_val);

                // Render each segment of the stacked bar
                for (idx, (color_label, _value)) in colors.iter().zip(values.iter()).enumerate() {
                    // Determine color for this bar segment
                    let fill_color = if self.encoding.color.is_some() {
                        // Find the index of this color_label in all unique colors
                        let unique_colors = color_series
                            .unique_stable()?
                            .str()?
                            .into_no_null_iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>();

                        let color_index = unique_colors
                            .iter()
                            .position(|c| c == color_label)
                            .ok_or_else(|| {
                                ChartonError::Internal(
                                    "color not found in unique values".to_string(),
                                )
                            })?;
                        Some(SingleColor::new(&self.mark_palette.get_color(color_index)))
                    } else {
                        mark.color.clone()
                    };

                    match !context.swapped_axes {
                        true => {
                            // Vertical stacked bars
                            let x_center = group_center;
                            let y_zero = (context.y_mapper)(cumulative_values[idx]);
                            let y_value = (context.y_mapper)(cumulative_values[idx + 1]);

                            // Render the bar segment using the bar renderer
                            crate::render::bar_renderer::render_vertical_bar(
                                svg,
                                crate::render::bar_renderer::VerticalBarConfig {
                                    x_center,
                                    y_zero,
                                    y_value,
                                    width: bar_width_pixels,
                                    fill_color: fill_color.clone(),
                                    stroke_color: mark.stroke.clone(),
                                    stroke_width: mark.stroke_width,
                                    opacity: mark.opacity,
                                },
                            )?;
                        }
                        false => {
                            // Horizontal stacked bars
                            let y_center = group_center;
                            let x_zero = (context.y_mapper)(cumulative_values[idx]);
                            let x_value = (context.y_mapper)(cumulative_values[idx + 1]);

                            // Render the bar segment using the bar renderer
                            crate::render::bar_renderer::render_horizontal_bar(
                                svg,
                                crate::render::bar_renderer::HorizontalBarConfig {
                                    x_zero,
                                    x_value,
                                    y_center,
                                    height: bar_width_pixels,
                                    fill_color: fill_color.clone(),
                                    stroke_color: mark.stroke.clone(),
                                    stroke_width: mark.stroke_width,
                                    opacity: mark.opacity,
                                },
                            )?;
                        }
                    }
                }
            } else {
                // Render regular grouped bars
                let groups_count = colors.len() as f64;

                // Calculate width using the provided formula or based on data range for continuous groups
                // Determine if group axis is continuous
                let group_scale = {
                    // For vertical bars, group is on x-axis
                    let x_enc = self.encoding.x.as_ref().unwrap();
                    x_enc.scale.clone().unwrap_or_else(|| {
                        let x_series = self.data.column(&x_enc.field).unwrap();
                        crate::data::determine_scale_for_dtype(x_series.dtype())
                    })
                };

                let bar_width_data = if matches!(group_scale, crate::coord::Scale::Discrete) {
                    // Discrete group axis - use the provided formula
                    user_width.min(span / (groups_count + (groups_count - 1.0) * spacing))
                } else {
                    // Continuous group axis - calculate width based on data range
                    let group_values: Vec<f64> = processed_data.x_transformed_vals.to_vec();

                    if group_values.len() > 1 {
                        // Calculate the range of group values
                        let min_group = group_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                        let max_group = group_values
                            .iter()
                            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                        let range = max_group - min_group;

                        // Calculate average distance between group values
                        let avg_distance = range / (group_values.len() - 1) as f64;

                        // Use a fraction of the average distance as bar width
                        avg_distance * 0.7 // 70% of the average distance
                    } else {
                        // Fallback to default width if only one group value
                        user_width.min(span)
                    }
                };

                // Get positions in pixel coordinates
                let base_position = (context.x_mapper)(0.0);
                let bar_width_position = (context.x_mapper)(bar_width_data);
                let bar_width_pixels = (bar_width_position - base_position).abs();

                // Calculate spacing in pixels
                let spacing_pixels = bar_width_pixels * spacing;

                // Position of the group center on group axis
                let group_center = (context.x_mapper)(group_val);

                // Render each bar in this group
                for (sub_idx, (color_label, value)) in colors.iter().zip(values.iter()).enumerate()
                {
                    // Calculate position for this specific bar within the group
                    let offset = (sub_idx as f64 - (groups_count - 1.0) / 2.0)
                        * (bar_width_pixels + spacing_pixels);
                    let bar_position = group_center + offset;

                    // Determine color for this bar
                    let fill_color = if self.encoding.color.is_some() {
                        // Find the index of this color_label in all unique colors
                        let unique_colors = color_series
                            .unique_stable()?
                            .str()?
                            .into_no_null_iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>();

                        let color_index = unique_colors
                            .iter()
                            .position(|c| c == color_label)
                            .ok_or_else(|| {
                                ChartonError::Internal(
                                    "color not found in unique values".to_string(),
                                )
                            })?;
                        Some(SingleColor::new(&self.mark_palette.get_color(color_index)))
                    } else {
                        mark.color.clone()
                    };

                    match !context.swapped_axes {
                        true => {
                            // Vertical bars: x is group, y is value
                            let x_center = bar_position;
                            let y_zero = (context.y_mapper)(0.0);
                            let y_value = (context.y_mapper)(*value);

                            // Render the bar using the bar renderer
                            crate::render::bar_renderer::render_vertical_bar(
                                svg,
                                crate::render::bar_renderer::VerticalBarConfig {
                                    x_center,
                                    y_zero,
                                    y_value,
                                    width: bar_width_pixels,
                                    fill_color: fill_color.clone(),
                                    stroke_color: mark.stroke.clone(),
                                    stroke_width: mark.stroke_width,
                                    opacity: mark.opacity,
                                },
                            )?;
                        }
                        false => {
                            // Horizontal bars: x is group, y is value, but the axes are transposed
                            let y_center = bar_position;
                            let x_zero = (context.y_mapper)(0.0);
                            let x_value = (context.y_mapper)(*value);

                            // Render the bar using the bar renderer
                            crate::render::bar_renderer::render_horizontal_bar(
                                svg,
                                crate::render::bar_renderer::HorizontalBarConfig {
                                    x_zero,
                                    x_value,
                                    y_center,
                                    height: bar_width_pixels,
                                    fill_color: fill_color.clone(),
                                    stroke_color: mark.stroke.clone(),
                                    stroke_width: mark.stroke_width,
                                    opacity: mark.opacity,
                                },
                            )?;
                        }
                    }
                }
            }
        }

        // Draw zero line after all bars have been rendered, based on y_encoding.zero setting
        let should_draw_zero_line = self.encoding.y.as_ref().unwrap().zero != Some(false);

        if should_draw_zero_line {
            if !context.swapped_axes {
                // For vertical bars, draw horizontal zero line
                let y_zero = (context.y_mapper)(0.0);
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-dasharray="5,5" />"#,
                    context.draw_x0,
                    y_zero,
                    context.draw_x0 + context.plot_width,
                    y_zero
                )?;
            } else {
                // For horizontal bars, draw vertical zero line
                let x_zero = (context.y_mapper)(0.0);
                writeln!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-dasharray="5,5" />"#,
                    x_zero,
                    context.draw_y0,
                    x_zero,
                    context.draw_y0 + context.plot_height
                )?;
            }
        }

        Ok(())
    }
}

// Implementation of MarkRenderer for Chart<MarkBar>
impl crate::chart::common::MarkRenderer for Chart<MarkBar> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_bars(svg, context)
    }
}

impl crate::chart::common::LegendRenderer for Chart<MarkBar> {
    fn render_legends(
        &self,
        svg: &mut String,
        theme: &Theme,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        color_legend_renderer::render_color_legend(svg, self, theme, context)?;

        Ok(())
    }
}
