use super::data_processor::ProcessedChartData;
use crate::chart::common::{Chart, SharedRenderingContext};
use crate::error::ChartonError;
use crate::mark::histogram::MarkHist;
use crate::render::color_legend_renderer;
use crate::theme::Theme;
use crate::visual::color::SingleColor;
use polars::prelude::*;

// Implementation specific to MarkHist with additional methods
impl Chart<MarkHist> {
    /// Create a new histogram mark chart
    ///
    /// This method initializes a `Chart` instance with a `MarkHist` marker to enable
    /// rendering of histogram charts. It sets up the basic structure required for
    /// histogram visualization.
    pub fn mark_hist(mut self) -> Self {
        self.mark = Some(MarkHist::new());
        self
    }

    /// Set the fill color for histogram bars
    ///
    /// Configures the interior color of the histogram bars. When `None` is provided,
    /// the system will use default coloring or palette-based colors if color encoding
    /// is applied.
    ///
    /// # Arguments
    /// * `color` - Optional `SingleColor` specifying the fill color for bars
    pub fn with_hist_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity for histogram bars
    ///
    /// Controls the transparency level of the histogram bars. Values range from 0.0
    /// (completely transparent) to 1.0 (completely opaque).
    ///
    /// # Arguments
    /// * `opacity` - A `f64` value between 0.0 and 1.0 representing the opacity level
    pub fn with_hist_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.opacity = opacity;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke color for histogram bars
    ///
    /// Defines the outline color of the histogram bars. When `None` is provided,
    /// no stroke will be rendered around the bars.
    ///
    /// # Arguments
    /// * `stroke` - Optional `SingleColor` specifying the stroke color for bar outlines
    pub fn with_hist_stroke(mut self, stroke: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke = stroke;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke width for histogram bars
    ///
    /// Specifies the thickness of the bar outlines in pixels. Larger values create
    /// thicker borders around the histogram bars.
    ///
    /// # Arguments
    /// * `stroke_width` - A `f64` value representing the stroke width in pixels
    pub fn with_hist_stroke_width(mut self, stroke_width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke_width = stroke_width;
        self.mark = Some(mark);
        self
    }

    // Render histogram with support for color encoding (grouped histogram)
    fn render_histogram(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Process chart data using shared processor
        let processed_data = ProcessedChartData::new(self, context.coord_system)?;

        // Extract the mark hist from Option
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Internal("Mark should exist when rendering histograms".to_string())
        })?;

        // Get unique values to avoid duplicates from color encoding
        let series = Series::new("x_vals".into(), processed_data.x_transformed_vals.clone());
        let unique_series = series.unique_stable()?;

        // Calculate bar width based on data spacing - since histogram bins are equally spaced,
        // we can just calculate the range and divide by number of bins
        let bar_width_data = if unique_series.n_unique()? > 1 {
            // For equally spaced histogram bins, we can calculate width as range / (n-1)
            let min_val = unique_series.min::<f64>()?.unwrap();
            let max_val = unique_series.max::<f64>()?.unwrap();
            // Calculate average spacing between consecutive bins
            (max_val - min_val) / (unique_series.n_unique()? - 1) as f64 * 0.95
        } else {
            1.0 // Default width if we only have one unique value or no values
        };

        let base_position = (context.x_mapper)(0.0);
        let bar_width_position = (context.x_mapper)(bar_width_data);
        let bar_width_pixels = (bar_width_position - base_position).abs();

        // Ensure we have a minimum bar width
        let bar_width_pixels = if bar_width_pixels < 1e-6 {
            10.0 // Default to 10 pixels if calculation fails
        } else {
            bar_width_pixels
        };

        // Create or get color column data - similar to line chart approach
        let color_series = if let Some(color_enc) = &self.encoding.color {
            // Use existing color column
            self.data.column(&color_enc.field)?
        } else {
            // Create a temporary color column with default value
            let len = processed_data.x_transformed_vals.len();
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

        // Render bars for each color group
        for (group_index, (_group_name, indices)) in group_data.iter().enumerate() {
            // Determine color for this group
            let fill_color = if self.encoding.color.is_some() {
                // Use palette colors for different groups
                Some(SingleColor::new(&self.mark_palette.get_color(group_index)))
            } else {
                mark.color.clone()
            };

            // Render all bars that belong to this color group
            for &i in indices {
                let x_val = processed_data.x_transformed_vals[i];
                let y_val = processed_data.y_transformed_vals[i];

                match !self.swapped_axes {
                    true => {
                        // Vertical histogram: x is bin value, y is count
                        let x_center = (context.x_mapper)(x_val);
                        let y_zero = (context.y_mapper)(0.0);
                        let y_value = (context.y_mapper)(y_val);

                        // Render the bar using the histogram renderer
                        crate::render::hist_renderer::render_vertical_histogram_bar(
                            svg,
                            x_center,
                            y_zero,
                            y_value,
                            bar_width_pixels,
                            &fill_color,
                            &mark.stroke,
                            mark.stroke_width,
                            mark.opacity,
                        )?;
                    }
                    false => {
                        // Horizontal histogram: y is bin value, x is count
                        let x_zero = (context.y_mapper)(0.0);
                        let x_value = (context.y_mapper)(y_val); // Note: for horizontal, count is in x direction
                        let y_center = (context.x_mapper)(x_val); // Note: for horizontal, bin value is in y direction

                        // Render the bar using the histogram renderer
                        crate::render::hist_renderer::render_horizontal_histogram_bar(
                            svg,
                            x_zero,
                            x_value,
                            y_center,
                            bar_width_pixels,
                            &fill_color,
                            &mark.stroke,
                            mark.stroke_width,
                            mark.opacity,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }
}

// Implementation of MarkRenderer for Chart<MarkHist>
impl crate::chart::common::MarkRenderer for Chart<MarkHist> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_histogram(svg, context)
    }
}

impl crate::chart::common::LegendRenderer for Chart<MarkHist> {
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
