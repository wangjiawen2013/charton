use super::common::{Chart, LegendRenderer, MarkRenderer, SharedRenderingContext};
use super::data_processor::ProcessedChartData;
use crate::error::ChartonError;
use crate::mark::text::{MarkText, TextAnchor, TextBaseline};
use crate::theme::Theme;
use crate::visual::color::SingleColor;

impl Chart<MarkText> {
    /// Initialize a text mark on the chart
    ///
    /// Creates a new text mark chart by initializing a `MarkText` instance.
    /// This enables rendering of text annotations at specific data coordinates.
    pub fn mark_text(mut self) -> Self {
        self.mark = Some(MarkText::new());
        self
    }

    /// Set the color of the text
    ///
    /// Configures the fill color of the text elements. When color encoding is used,
    /// this serves as a fallback color for text elements.
    ///
    /// # Arguments
    /// * `color` - A `SingleColor` specifying the text color
    pub fn with_text_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkText::new);
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the size of the text
    ///
    /// Controls the font size of the text elements in pixels. Larger values create
    /// more prominent text that is easier to read but takes up more space.
    ///
    /// # Arguments
    /// * `size` - A `f64` value representing the font size in pixels
    pub fn with_text_size(mut self, size: f64) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkText::new);
        mark.size = size;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity of the text
    ///
    /// Adjusts the transparency of the text elements. Values range from 0.0 (fully transparent)
    /// to 1.0 (fully opaque). Useful for de-emphasizing certain annotations or creating
    /// layered effects.
    ///
    /// # Arguments
    /// * `opacity` - A `f64` value between 0.0 and 1.0 representing the text opacity
    pub fn with_text_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkText::new);
        mark.opacity = opacity;
        self.mark = Some(mark);
        self
    }

    /// Set static text for all text marks
    ///
    /// Assigns the same text content to all text marks in the chart. When text encoding
    /// is used, this serves as a fallback text value.
    ///
    /// # Arguments
    /// * `text` - A string slice containing the text to display
    pub fn with_text_content(mut self, text: &str) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkText::new);
        mark.text = text.to_string();
        self.mark = Some(mark);
        self
    }

    /// Set the text anchor for alignment
    ///
    /// Controls the horizontal alignment of text relative to its anchor point.
    ///
    /// # Arguments
    /// * `anchor` - A `TextAnchor` enum value specifying the horizontal alignment
    ///   - `Start`: Left-aligned to the anchor point
    ///   - `Middle`: Centered on the anchor point
    ///   - `End`: Right-aligned to the anchor point
    pub fn with_text_anchor(mut self, anchor: TextAnchor) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkText::new);
        mark.anchor = anchor;
        self.mark = Some(mark);
        self
    }

    /// Set the text baseline for vertical alignment
    ///
    /// Controls the vertical alignment of text relative to its anchor point.
    ///
    /// # Arguments
    /// * `baseline` - A `TextBaseline` enum value specifying the vertical alignment
    ///   - `Auto`: Browser default baseline
    ///   - `Middle`: Vertically centered on the anchor point
    ///   - `Hanging`: Top of the text aligned with the anchor point
    pub fn with_text_baseline(mut self, baseline: TextBaseline) -> Self {
        let mut mark = self.mark.unwrap_or_else(MarkText::new);
        mark.baseline = baseline;
        self.mark = Some(mark);
        self
    }

    // Render all text marks for this chart
    fn render_texts(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        // Process chart data using shared processor
        let processed_data = ProcessedChartData::new(self, context.coord_system)?;

        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Internal("Mark should exist when rendering texts".to_string())
        })?;

        // Get text values from data if text encoding is used, otherwise use static text
        let text_values = if let Some(text_enc) = &self.encoding.text {
            self.data
                .column(&text_enc.field)?
                .str()?
                .into_no_null_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        } else {
            vec![mark.text.clone(); processed_data.x_transformed_vals.len()]
        };

        for (i, ((&x, &y), text)) in processed_data
            .x_transformed_vals
            .iter()
            .zip(&processed_data.y_transformed_vals)
            .zip(text_values.iter())
            .enumerate()
        {
            // Determine size for this text
            let size = processed_data
                .normalized_sizes
                .as_ref()
                .and_then(|sizes| sizes.get(i).copied())
                .unwrap_or_else(|| mark.size);

            // Determine color for this text based on the scale type
            let fill_color = if let Some((scale_type, color_values)) = &processed_data.color_info {
                let &normalized_value = &color_values[i];
                match scale_type {
                    // Continuous color mapping using colormap
                    crate::coord::Scale::Linear | crate::coord::Scale::Log => {
                        // Use the existing colormap implementation
                        let color = self.mark_cmap.get_color(normalized_value);
                        Some(SingleColor::new(&color))
                    }
                    // Discrete color mapping using palette
                    crate::coord::Scale::Discrete => {
                        // Use the existing palette implementation
                        let color = self.mark_palette.get_color(normalized_value as usize);
                        Some(SingleColor::new(&color))
                    }
                }
            } else {
                // No color encoding channel, use text's default color
                self.mark.as_ref().and_then(|m| m.color.clone())
            };

            let fill_color_str = fill_color
                .as_ref()
                .map(|c| c.get_color())
                .unwrap_or_else(|| "none".to_string());

            // Render the text
            let (x_pos, y_pos) = if context.swapped_axes {
                ((context.y_mapper)(y), (context.x_mapper)(x)) // Swap x and y when axes are swapped
            } else {
                ((context.x_mapper)(x), (context.y_mapper)(y)) // Normal order when axes are not swapped
            };

            let anchor = match mark.anchor {
                TextAnchor::Start => "start",
                TextAnchor::Middle => "middle",
                TextAnchor::End => "end",
            };

            let baseline = match mark.baseline {
                TextBaseline::Auto => "auto",
                TextBaseline::Middle => "middle",
                TextBaseline::Hanging => "hanging",
            };

            // Add text to SVG
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" font-size="{}" fill="{}" opacity="{}" text-anchor="{}" dominant-baseline="{}">{}</text>"#,
                x_pos, y_pos, size, fill_color_str, mark.opacity, anchor, baseline, text
            ));
        }

        Ok(())
    }
}

// Implement MarkRenderer for Chart<MarkText>
impl MarkRenderer for Chart<MarkText> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_texts(svg, context)
    }
}

// Implement LegendRenderer for Chart<MarkText>
impl LegendRenderer for Chart<MarkText> {
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

        // Render size legend if there's a size encoding
        if self.encoding.size.is_some() {
            crate::render::size_legend_renderer::render_size_legend(svg, self, theme, context)?;
        }

        Ok(())
    }
}
