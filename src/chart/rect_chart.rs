use super::common::{Chart, LegendRenderer, MarkRenderer, SharedRenderingContext};
use crate::chart::data_processor::ProcessedChartData;
use crate::error::ChartonError;
use crate::mark::rect::MarkRect;
use crate::render::rect_renderer::render_rect;
use crate::theme::Theme;
use crate::visual::color::SingleColor;
use ordered_float::OrderedFloat;
use std::collections::HashSet;

impl Chart<MarkRect> {
    /// Create a new rectangle mark chart (heatmap)
    ///
    /// Initializes the chart with a `MarkRect` instance, enabling heatmap-style rendering
    /// where data points are represented as colored rectangles. This is typically used
    /// for displaying matrix-like data or binned aggregations.
    pub fn mark_rect(mut self) -> Self {
        self.mark = Some(MarkRect::new());
        self
    }

    /// Set the fill color for rectangles
    ///
    /// Configures the base fill color used for rectangles. In most cases, rectangles
    /// will be colored based on data values using a colormap, but this provides a
    /// fallback color when needed.
    ///
    /// # Arguments
    /// * `color` - A `SingleColor` specifying the fill color for rectangles
    pub fn with_rect_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity for rectangles
    ///
    /// Adjusts the transparency of the rectangles. Values range from 0.0 (fully transparent)
    /// to 1.0 (fully opaque). Useful for layering or when rectangles overlap.
    ///
    /// # Arguments
    /// * `opacity` - A `f64` value between 0.0 and 1.0 representing the rectangle opacity
    pub fn with_rect_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.opacity = opacity;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke color for rectangles
    ///
    /// Defines the color of the borders around each rectangle. Borders can help
    /// distinguish adjacent rectangles, especially when they have similar colors.
    ///
    /// # Arguments
    /// * `stroke` - A `SingleColor` specifying the border color for rectangles
    pub fn with_rect_stroke(mut self, stroke: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke = stroke;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke width for rectangles
    ///
    /// Controls the thickness of the borders around each rectangle in pixels.
    ///
    /// # Arguments
    /// * `stroke_width` - A `f64` value representing the border thickness in pixels
    pub fn with_rect_stroke_width(mut self, stroke_width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke_width = stroke_width;
        self.mark = Some(mark);
        self
    }

    fn render_rects(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Use ProcessedChartData to handle data processing
        let processed_data = ProcessedChartData::new(self, context.coord_system)?;

        // Extract the mark from Option
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Internal("Mark should exist when rendering rectangles".to_string())
        })?;

        // Extract processed values
        let x_transformed_vals = processed_data.x_transformed_vals;
        let y_transformed_vals = processed_data.y_transformed_vals;

        // For rect charts, color encoding is f64 and guaranteed to exist
        let (_, normalized_color_values) = processed_data.color_info.unwrap();

        // Calculate min, max, and unique count for x values using transformed data
        let x_min = x_transformed_vals
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let x_max = x_transformed_vals
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let x_unique_count: HashSet<OrderedFloat<f64>> = x_transformed_vals
            .clone()
            .into_iter()
            .map(OrderedFloat)
            .collect();

        // Calculate min, max, and unique count for y values using transformed data
        let y_min = y_transformed_vals
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let y_max = y_transformed_vals
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let y_unique_count: HashSet<OrderedFloat<f64>> = y_transformed_vals
            .clone()
            .into_iter()
            .map(OrderedFloat)
            .collect();

        // Calculate rectangle width and height
        let (rect_h_pixels, rect_v_pixels) = {
            // When axes are swapped, we need to adjust how we calculate dimensions
            let x_pixels = ((context.x_mapper)(x_max) - (context.x_mapper)(x_min)).abs()
                / (x_unique_count.len() as f64 - 1.0);
            let y_pixels = ((context.y_mapper)(y_max) - (context.y_mapper)(y_min)).abs()
                / (y_unique_count.len() as f64 - 1.0);
            if context.swapped_axes {
                (y_pixels, x_pixels)
            } else {
                (x_pixels, y_pixels)
            }
        };

        // Render rectangles
        for (i, (&x_val, &y_val)) in x_transformed_vals
            .iter()
            .zip(&y_transformed_vals)
            .enumerate()
        {
            // When axes are swapped, we need to swap the coordinates passed to the mappers
            let (h_pixel, v_pixel) = if context.swapped_axes {
                ((context.y_mapper)(y_val), (context.x_mapper)(x_val)) // Swap x and y when axes are swapped
            } else {
                ((context.x_mapper)(x_val), (context.y_mapper)(y_val)) // Normal order when axes are not swapped
            };

            // Adjust position based on whether data is discrete or continuous
            let (adjusted_h_pixel, adjusted_v_pixel) =
                (h_pixel - rect_h_pixels / 2.0, v_pixel - rect_v_pixels / 2.0);

            // Color determination using pre-normalized values from ProcessedChartData
            let color = self.mark_cmap.get_color(normalized_color_values[i]);

            let stroke_color_str = mark
                .stroke
                .as_ref()
                .map(|c| c.get_color())
                .unwrap_or_else(|| "none".to_string());

            // Add rectangle to SVG
            render_rect(
                svg,
                adjusted_h_pixel,
                adjusted_v_pixel,
                rect_h_pixels,
                rect_v_pixels,
                &color,
                mark.opacity,
                &stroke_color_str,
                mark.stroke_width,
            )?;
        }

        Ok(())
    }
}

// Implement MarkRenderer for Chart<MarkRect>
impl MarkRenderer for Chart<MarkRect> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_rects(svg, context)
    }
}

// Implement LegendRenderer for Chart<MarkRect>
impl LegendRenderer for Chart<MarkRect> {
    fn render_legends(
        &self,
        svg: &mut String,
        theme: &Theme,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        crate::render::colorbar_renderer::render_colorbar(svg, self, theme, context)?;

        Ok(())
    }
}
