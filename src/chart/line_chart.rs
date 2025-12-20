use super::common::{Chart, LegendRenderer, MarkRenderer};
use super::data_processor::ProcessedChartData;
use crate::chart::common::SharedRenderingContext;
use crate::error::ChartonError;
use crate::mark::line::MarkLine;
use crate::render::line_renderer::{PathInterpolation, render_line};
use crate::theme::Theme;
use crate::visual::color::SingleColor;
use polars::prelude::*;

// Implementation for Chart<MarkLine> with line-specific methods
impl Chart<MarkLine> {
    /// Create a new line chart marker
    ///
    /// Initializes the chart with a `MarkLine` instance, enabling line chart rendering.
    /// This method must be called to configure the chart for displaying line data.
    pub fn mark_line(mut self) -> Self {
        self.mark = Some(MarkLine::new());
        self
    }

    /// Set the color of the line
    ///
    /// Configures the color used to draw the line. If not set, the system will use
    /// palette colors based on groupings or a default color.
    ///
    /// # Arguments
    /// * `color` - A `SingleColor` specifying the line color
    pub fn with_line_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkLine::new);
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke width of the line
    ///
    /// Controls the thickness of the line in pixels. Thicker lines are more visible
    /// but may obscure details in dense plots.
    ///
    /// # Arguments
    /// * `width` - A `f64` value representing the stroke width in pixels
    pub fn with_line_stroke_width(mut self, width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkLine::new);
        mark.stroke_width = width;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity of the line
    ///
    /// Adjusts the transparency of the line. Values range from 0.0 (fully transparent)
    /// to 1.0 (fully opaque). Useful for overlapping lines or emphasizing certain data.
    ///
    /// # Arguments
    /// * `opacity` - A `f64` value between 0.0 and 1.0 representing the line opacity
    pub fn with_line_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkLine::new);
        mark.opacity = opacity;
        self.mark = Some(mark);
        self
    }

    /// Apply LOESS smoothing to the line data
    ///
    /// Transforms the line data using LOESS (Locally Estimated Scatterplot Smoothing)
    /// algorithm to create a smooth curve that follows the general trend of the data.
    /// This is particularly useful for noisy data where you want to highlight the
    /// underlying pattern.
    ///
    /// # Arguments
    /// * `bandwidth` - A `f64` value between 0.0 and 1.0 controlling the smoothing level.
    ///   Higher values produce smoother curves while lower values follow data points more closely.
    ///   Typical values range from 0.1 (less smooth) to 0.9 (very smooth).
    pub fn transform_loess(mut self, bandwidth: f64) -> Self {
        let mut mark = MarkLine::new();
        mark.use_loess = true;
        mark.loess_bandwidth = bandwidth;
        self.mark = Some(mark);

        self
    }

    /// Set the path interpolation method for the line
    ///
    /// This method configures how the line connects data points by setting the
    /// interpolation algorithm. Different interpolation methods can be used to
    /// better represent the nature of the data being visualized.
    ///
    /// # Arguments
    /// * `interpolation` - A `PathInterpolation` enum value specifying the
    ///   interpolation method to use:
    ///   - `PathInterpolation::Linear`: Straight line segments between points (default)
    ///   - `PathInterpolation::StepAfter`: Step function holding value until next point
    ///     (appropriate for empirical cumulative distribution functions)
    ///   - `PathInterpolation::StepBefore`: Step function jumping to next value immediately
    ///
    /// # Returns
    /// Returns the modified chart instance with the new interpolation setting,
    /// allowing for method chaining.
    ///
    pub fn with_interpolation(mut self, interpolation: PathInterpolation) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkLine::new);
        mark.interpolation = interpolation;
        self.mark = Some(mark);
        self
    }

    // Render all lines for this chart
    fn render_lines(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Process chart data using shared processor
        let processed_data = ProcessedChartData::new(self, context.coord_system)?;

        // Extract the mark from Option
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Internal("Mark should exist when rendering lines".to_string())
        })?;

        // Create or get color column data
        let color_series = if let Some(color_enc) = &self.encoding.color {
            // Use existing color column
            self.data
                .column(&color_enc.field)?
                .with_name("color".into())
        } else {
            // Create a temporary color column with default value to avoid duplication code
            let len = self.data.df.height();
            let default_colors: Vec<&str> = vec!["group"; len];
            let series = Series::new("color".into(), default_colors);
            series
        };

        // Use Polars' group_by functionality to get indices for each group
        let indices: Vec<u32> = (0..color_series.len() as u32).collect();
        let indices_series = Series::new("index".into(), &indices);
        // Create a DataFrame with indices and color values for grouping
        let temp_df = DataFrame::new(vec![indices_series.into(), color_series.into()])?;
        // Group order is the same as unique (ordered by their first appearance)
        let groups = temp_df.partition_by_stable(["color"], true)?;

        // Extract group names and their corresponding indices
        let mut group_data: Vec<(String, Vec<usize>)> = Vec::new();
        for group_df in groups {
            let color_col = group_df.column("color")?;
            let color_val = color_col.str()?.get(0).unwrap().to_string();

            let index_col = group_df.column("index")?;
            let indices: Vec<usize> = index_col
                .u32()?
                .into_no_null_iter()
                .map(|i| i as usize)
                .collect();

            group_data.push((color_val, indices));
        }

        // Find global min/max x values across all groups for consistent ECDF range
        let global_x_min = processed_data
            .x_transformed_vals
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let global_x_max = processed_data
            .x_transformed_vals
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        // Render separate line for each group
        for (group_index, (_group_name, indices)) in group_data.iter().enumerate() {
            // Extract data for this group from the processed data
            let group_x_vals: Vec<f64> = indices
                .iter()
                .map(|&i| processed_data.x_transformed_vals[i])
                .collect();
            let group_y_vals: Vec<f64> = indices
                .iter()
                .map(|&i| processed_data.y_transformed_vals[i])
                .collect();

            // Apply LOESS transformation if requested (per group)
            let (mut final_x_vals, mut final_y_vals) = if mark.use_loess {
                crate::stats::stat_loess::loess(&group_x_vals, &group_y_vals, mark.loess_bandwidth)
            } else {
                (group_x_vals, group_y_vals)
            };

            // For ECDF consistency, prepend starting point and append ending point
            // We only do this for StepAfter interpolation which is appropriate for ECDF
            if matches!(mark.interpolation, PathInterpolation::StepAfter) {
                // Add starting point only if it doesn't already exist
                if final_x_vals.first() != Some(&global_x_min) {
                    final_x_vals.insert(0, global_x_min);
                    final_y_vals.insert(0, 0.0);
                }

                // For the ending point, we should make sure it extends properly
                // Only add an explicit ending point if the current last point isn't at global_x_max
                let last_x = *final_x_vals.last().ok_or_else(|| {
                    ChartonError::Internal("Empty x values when rendering ECDF line".to_string())
                })?;
                if last_x != global_x_max {
                    // Extend horizontally at the maximum y-level to global_x_max
                    final_x_vals.push(global_x_max);
                    // Use the maximum y-value of this group (which is the last value in sorted ECDF)
                    let last_y = *final_y_vals.last().ok_or_else(|| {
                        ChartonError::Internal(
                            "Empty y values when rendering ECDF line".to_string(),
                        )
                    })?;
                    final_y_vals.push(last_y);
                }
            }

            // Create points for this group, accounting for swapped axes
            let points: Vec<(f64, f64)> = final_x_vals
                .iter()
                .zip(final_y_vals.iter())
                .map(|(&x, &y)| {
                    let x_pixel = (context.x_mapper)(x);
                    let y_pixel = (context.y_mapper)(y);

                    // When axes are swapped, we need to swap the coordinates
                    if context.swapped_axes {
                        (y_pixel, x_pixel) // Swap x and y when axes are swapped
                    } else {
                        (x_pixel, y_pixel) // Normal order when axes are not swapped
                    }
                })
                .collect();

            // Determine color for this group line
            let stroke_color = if self.encoding.color.is_some() {
                Some(SingleColor::new(&self.mark_palette.get_color(group_index)))
            } else {
                mark.color.clone()
            };

            // Render the line for this group
            render_line(
                svg,
                &points,
                &stroke_color,
                mark.stroke_width,
                mark.opacity,
                &mark.interpolation,
            )?;
        }

        Ok(())
    }
}

// Implement MarkRenderer for Chart<MarkLine>
impl MarkRenderer for Chart<MarkLine> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_lines(svg, context)
    }
}

// Implement LegendRenderer for Chart<MarkLine>
impl LegendRenderer for Chart<MarkLine> {
    fn render_legends(
        &self,
        svg: &mut String,
        theme: &Theme,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Render color legend or colorbar based on color encoding type
        let color_is_continuous = if let Some(color_enc) = &self.encoding.color {
            let color_series = self.data.column(&color_enc.field)?;
            let scale_type = crate::data::determine_scale_for_dtype(color_series.dtype());
            !matches!(scale_type, crate::coord::Scale::Discrete)
        } else {
            false
        };

        if color_is_continuous {
            // Render colorbar for continuous color scales
            crate::render::colorbar_renderer::render_colorbar(svg, self, theme, context)?;
        } else {
            // Render legend for discrete color scales or when there's no color encoding
            crate::render::color_legend_renderer::render_color_legend(svg, self, theme, context)?;
        }

        // Render stroke width legend if there's a stroke width encoding
        if self.encoding.stroke_width.is_some() {
            crate::render::size_legend_renderer::render_size_legend(svg, self, theme, context)?;
        }

        Ok(())
    }
}
