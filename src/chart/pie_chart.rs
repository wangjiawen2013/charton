use super::common::{Chart, LegendRenderer, MarkRenderer, SharedRenderingContext};
use crate::error::ChartonError;
use crate::mark::arc::MarkArc;
use crate::theme::Theme;
use crate::visual::color::SingleColor;
use polars::prelude::*;

// Implementation for Chart<MarkArc>
impl Chart<MarkArc> {
    /// Create a new arc/pie chart marker
    ///
    /// Initializes the chart with a `MarkArc` instance, enabling pie/donut chart rendering.
    /// This method must be called to configure the chart for displaying categorical data
    /// as slices of a circle.
    pub fn mark_arc(mut self) -> Self {
        self.mark = Some(MarkArc::new());
        self
    }

    /// Set the fill color for pie slices
    ///
    /// Configures the base color used to fill the pie slices. If individual slice colors
    /// are needed, the palette system will be used automatically unless overridden.
    ///
    /// # Arguments
    /// * `color` - A `SingleColor` specifying the fill color for all slices
    pub fn with_arc_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkArc::new);
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity for pie slices
    ///
    /// Adjusts the transparency of the pie slices. Values range from 0.0 (fully transparent)
    /// to 1.0 (fully opaque). Useful for layering or emphasizing certain data.
    ///
    /// # Arguments
    /// * `opacity` - A `f64` value between 0.0 and 1.0 representing the slice opacity
    pub fn with_arc_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkArc::new);
        mark.opacity = opacity;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke color for pie slice borders
    ///
    /// Defines the color of the borders separating each pie slice. Borders help distinguish
    /// adjacent slices, especially when they have similar colors.
    ///
    /// # Arguments
    /// * `stroke` - A `SingleColor` specifying the border color for pie slices
    pub fn with_arc_stroke(mut self, stroke: SingleColor) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkArc::new);
        mark.stroke = Some(stroke);
        self.mark = Some(mark);
        self
    }

    /// Set the stroke width for pie slice borders
    ///
    /// Controls the thickness of the borders separating each pie slice in pixels.
    ///
    /// # Arguments
    /// * `stroke_width` - A `f64` value representing the border thickness in pixels
    pub fn with_arc_stroke_width(mut self, stroke_width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkArc::new);
        mark.stroke_width = stroke_width;
        self.mark = Some(mark);
        self
    }

    /// Set the inner radius ratio for donut charts
    ///
    /// Controls the size of the center hole in pie charts, transforming them into donut charts.
    /// The ratio determines the inner radius as a proportion of the outer radius.
    ///
    /// # Arguments
    /// * `ratio` - A `f64` value between 0.0 and 1.0 representing the ratio of inner radius to outer radius
    ///   - 0.0 creates a regular pie chart (no hole)
    ///   - Values between 0.0 and 1.0 create donut charts with varying hole sizes
    ///   - 1.0 would create an invisible chart (completely hollow)
    ///
    /// # Returns
    /// Returns `Self` with the updated inner radius ratio
    pub fn with_inner_radius_ratio(mut self, ratio: f64) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkArc::new);
        mark.inner_radius_ratio = ratio.clamp(0.0, 1.0);
        self.mark = Some(mark);
        self
    }

    // Render all arcs for this chart
    fn render_arcs(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Calculate center of the pie chart
        let center_x = (context.draw_x0 + context.plot_width / 2.0).round();
        let center_y = (context.draw_y0 + context.plot_height / 2.0).round();

        // Calculate radius (80% of the smaller dimension to leave some margin)
        let radius = context.plot_width.min(context.plot_height) * 0.4;

        // Create a working DataFrame - either the original or a grouped version
        let working_df = {
            // Get original order of categories
            let color_encoding = &self.encoding.color.as_ref().unwrap();
            let theta_encoding = &self.encoding.theta.as_ref().unwrap();

            //let original_color_series = self.data.df.column(&color_encoding.field)?;
            let original_color_series = self.data.column(&color_encoding.field)?;
            let unique_original_colors_series = original_color_series.unique_stable()?;
            let unique_original_colors: Vec<&str> = unique_original_colors_series
                .str()?
                .into_no_null_iter()
                .collect();

            // Using lazy API for simple groupby and sum operation
            let grouped_df = self
                .data
                .df
                .clone()
                .lazy()
                .group_by([col(&color_encoding.field)])
                .agg([col(&theta_encoding.field).sum()])
                .collect()?;

            // Create a sorting column based on original order
            let grouped_color_series = grouped_df.column(&color_encoding.field)?;
            let grouped_colors: Vec<&str> =
                grouped_color_series.str()?.into_no_null_iter().collect();

            // Create sort indices based on original order
            let mut sort_indices: Vec<u32> = Vec::with_capacity(grouped_colors.len());
            for category in &unique_original_colors {
                let pos = grouped_colors.iter().position(|&c| c == *category).unwrap();
                sort_indices.push(pos as u32);
            }

            // Sort the DataFrame according to original order
            let indices_series = Series::new("".into(), sort_indices);
            grouped_df.take(indices_series.u32()?)?
        };

        // Wrap the working DataFrame back into DataFrameSource
        let working_df_source = crate::data::DataFrameSource::new(working_df);

        // Get data columns from the working DataFrame
        let theta_series =
            working_df_source.column(self.encoding.theta.as_ref().unwrap().field.as_str())?;

        let theta_vals: Vec<f64> = theta_series.f64()?.into_no_null_iter().collect();
        // Check that all values are non-negative
        if theta_vals.iter().any(|&value| value < 0.0) {
            return Err(ChartonError::Data(
                "All theta values must be non-negative to render a pie chart".to_string(),
            ));
        }

        // Calculate total for percentage calculation
        let total: f64 = theta_vals.iter().sum();

        // Calculate start angles for each slice
        let mut cumulative_angle = -std::f64::consts::PI / 2.0; // Start from top

        for (i, &value) in theta_vals.iter().enumerate() {
            let slice_angle = 2.0 * std::f64::consts::PI * value / total;

            // Calculate start and end angles
            let start_angle = cumulative_angle;
            let end_angle = cumulative_angle + slice_angle;

            // Get mark properties
            let mark = self.mark.as_ref().ok_or_else(|| {
                ChartonError::Internal("Mark should exist when rendering arcs".to_string())
            })?;

            // Determine color for this slice
            let fill_color = if self.encoding.color.is_some() {
                Some(SingleColor::new(&self.mark_palette.get_color(i)))
            } else {
                mark.color.clone()
            };

            // Draw arc/slice using the renderer
            crate::render::arc_renderer::render_arc_slice(
                svg,
                center_x,
                center_y,
                radius,
                mark.inner_radius_ratio,
                start_angle,
                end_angle,
                &fill_color,
                &mark.stroke,
                mark.stroke_width,
                mark.opacity,
            )?;

            cumulative_angle = end_angle;
        }

        Ok(())
    }
}

// Implement MarkRenderer for Chart<MarkArc>
impl MarkRenderer for Chart<MarkArc> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_arcs(svg, context)
    }
}

// Implement LegendRenderer for Chart<MarkArc>
impl LegendRenderer for Chart<MarkArc> {
    fn render_legends(
        &self,
        svg: &mut String,
        theme: &Theme,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Render legend for discrete color scales
        if self.encoding.color.is_some() {
            crate::render::color_legend_renderer::render_color_legend(svg, self, theme, context)?;
        }

        Ok(())
    }
}
