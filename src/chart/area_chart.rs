use crate::chart::common::{Chart, SharedRenderingContext};
use crate::chart::data_processor::ProcessedChartData;
use crate::error::ChartonError;
use crate::mark::area::MarkArea;
use crate::render::color_legend_renderer;
use crate::theme::Theme;
use crate::visual::color::SingleColor;
use polars::prelude::*;
use std::fmt::Write;

// Implementation specific to MarkArea with additional methods
impl Chart<MarkArea> {
    /// Create a new area mark chart
    ///
    /// Initializes a chart with area marks for data visualization. This is the starting point
    /// for creating area charts, which can be further customized with various styling options.
    ///
    /// # Returns
    ///
    /// Returns the chart instance with area mark initialized
    pub fn mark_area(mut self) -> Self {
        self.mark = Some(MarkArea::new());
        self
    }

    /// Set the fill color for areas
    ///
    /// Defines the color used to fill the area polygons. If None is provided, the chart
    /// will use colors from the default palette to distinguish different data groups.
    ///
    /// # Arguments
    ///
    /// * `color` - Optional SingleColor to fill the areas. If None, palette colors are used
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_area_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity for areas
    ///
    /// Controls the transparency level of the filled areas. Values range from 0.0 (fully transparent)
    /// to 1.0 (fully opaque). Useful for overlapping areas or creating visual effects.
    ///
    /// # Arguments
    ///
    /// * `opacity` - A f64 value between 0.0 and 1.0 representing the opacity level
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_area_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.opacity = opacity;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke color for areas
    ///
    /// Defines the color of the outline/border around area shapes. If None is provided,
    /// no stroke will be drawn around the areas.
    ///
    /// # Arguments
    ///
    /// * `stroke` - Optional SingleColor for the area outlines. If None, no stroke is applied
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_area_stroke(mut self, stroke: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke = stroke;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke width for areas
    ///
    /// Controls the thickness of the outline/border around area shapes. Larger values
    /// create more prominent borders, while smaller values create thinner ones.
    ///
    /// # Arguments
    ///
    /// * `stroke_width` - A f64 value representing the width of the stroke in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_area_stroke_width(mut self, stroke_width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke_width = stroke_width;
        self.mark = Some(mark);
        self
    }

    // Render area elements
    fn render_areas(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Process chart data using shared processor
        let processed_data = ProcessedChartData::new(self, context.coord_system)?;

        // Extract the mark from Option
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Internal("Mark should exist when rendering rules".to_string())
        })?;

        // Create or get color column data
        let color_series = if let Some(color_enc) = &self.encoding.color {
            // Use existing color column
            self.data.column(&color_enc.field)?
        } else {
            // Create a temporary color column with default value to avoid duplication code
            let len = self.data.df.height();
            let default_colors: Vec<&str> = vec!["group"; len];

            Series::new("color".into(), default_colors)
        };

        // Use Polars' group_by functionality to get indices for each group
        let str_series = color_series.str()?;
        // Create a DataFrame with indices and color values for grouping
        let indices: Vec<u32> = (0..str_series.len() as u32).collect();
        let indices_series = Series::new("index".into(), &indices);
        let color_series_named = str_series.clone().with_name("color".into());

        let temp_df = DataFrame::new(vec![
            indices_series.into(),
            color_series_named.into_series().into(),
        ])?;
        // Group order is the same as unique (ordered by their first appearance)
        let groups = temp_df.partition_by_stable(["color"], true)?;

        // Extract group names and their corresponding indices
        let mut group_data: Vec<(String, Vec<usize>)> = Vec::new();
        for group_df in groups {
            let group_df = group_df;
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

        // Render separate area for each group
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

            // Create points for this group, accounting for swapped axes
            let mut points: Vec<(f64, f64)> = group_x_vals
                .iter()
                .zip(group_y_vals.iter())
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

            let fill_color = if self.encoding.color.is_some() {
                Some(SingleColor::new(&self.mark_palette.get_color(group_index)))
            } else {
                mark.color.clone()
            };

            // Create area path. Build area points with proper baseline filling
            let mut area_points: Vec<(f64, f64)> = Vec::new();

            if context.swapped_axes {
                // For swapped axes, fill from x=0 (vertical baseline) to x values
                let zero_x = (context.y_mapper)(0.0); // Note: y_mapper because axes are swapped

                // Sort points by y coordinate to ensure proper area drawing when axes are swapped
                points.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                // Start at the zero baseline for the first point
                area_points.push((zero_x, points[0].1));
                // Line to the first data point
                area_points.push((points[0].0, points[0].1));

                // Draw line to each subsequent point (this will be skipped if only one point)
                for p in points.iter().skip(1) {
                    area_points.push((p.0, p.1));
                }

                // Connect back to the zero baseline for the last point
                area_points.push((zero_x, points[points.len() - 1].1));
            } else {
                // For normal axes, fill from y=0 (horizontal baseline) to y values
                let zero_y = (context.y_mapper)(0.0);

                // Sort points by x coordinate to ensure proper area drawing
                points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

                // Start at the zero baseline for the first point
                area_points.push((points[0].0, zero_y));
                // Line to the first data point
                area_points.push((points[0].0, points[0].1));

                // Draw line to each subsequent point (this will be skipped if only one point)
                for p in points.iter().skip(1) {
                    area_points.push((p.0, p.1));
                }

                // Connect back to the zero baseline for the last point
                area_points.push((points[points.len() - 1].0, zero_y));
            }

            // Render the area using the area renderer
            crate::render::area_renderer::render_area(
                svg,
                crate::render::area_renderer::AreaConfig {
                    points: area_points,
                    fill_color: fill_color.clone(),
                    stroke_color: mark.stroke.clone(),
                    stroke_width: mark.stroke_width,
                    opacity: mark.opacity,
                    closed: false, // Don't close the path to create a filled area
                },
            )?;
        }

        // Draw zero line after all areas have been rendered, based on y_encoding.zero setting
        let should_draw_zero_line = self
            .encoding
            .y
            .as_ref()
            .map(|y_enc| y_enc.zero != Some(false))
            .unwrap_or(true);

        if should_draw_zero_line {
            if !context.swapped_axes {
                // For vertical areas, draw horizontal zero line
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
                // For horizontal areas, draw vertical zero line
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

// Implementation of MarkRenderer for Chart<MarkArea>
impl crate::chart::common::MarkRenderer for Chart<MarkArea> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_areas(svg, context)
    }
}

impl crate::chart::common::LegendRenderer for Chart<MarkArea> {
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
