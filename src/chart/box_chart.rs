use crate::chart::common::{Chart, SharedRenderingContext};
use crate::error::ChartonError;
use crate::mark::boxplot::MarkBoxplot;
use crate::render::color_legend_renderer;
use crate::theme::Theme;
use crate::visual::color::SingleColor;
use indexmap::IndexSet;
use polars::prelude::*;
use uuid::Uuid;

// Statistics for a single box plot
#[derive(Debug, Clone)]
struct BoxStats {
    min: f64,
    q1: f64,
    median: f64,
    q3: f64,
    max: f64,
    outliers: Vec<f64>,
    _mean: f64,
}

impl Chart<MarkBoxplot> {
    /// Create a new box whisker chart
    ///
    /// Initializes a chart with box plot marks for statistical data visualization. This is the starting point
    /// for creating box charts, which display the distribution of data through quartiles and outliers.
    ///
    /// # Returns
    ///
    /// Returns the chart instance with box plot mark initialized
    pub fn mark_boxplot(mut self) -> Self {
        self.mark = Some(MarkBoxplot::new());
        self
    }

    /// Set the fill color for boxes
    ///
    /// Defines the color used to fill the box shapes representing the interquartile range (IQR).
    /// If None is provided, the chart will use colors from the default palette to distinguish
    /// different data groups.
    ///
    /// # Arguments
    ///
    /// * `color` - Optional SingleColor to fill the boxes. If None, palette colors are used
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_box_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity for boxes
    ///
    /// Controls the transparency level of the filled boxes. Values range from 0.0 (fully transparent)
    /// to 1.0 (fully opaque). Useful for overlapping boxes or creating visual effects.
    ///
    /// # Arguments
    ///
    /// * `opacity` - A f64 value between 0.0 and 1.0 representing the opacity level
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_box_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.opacity = opacity;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke color for boxes
    ///
    /// Defines the color of the outline/border around box shapes and whiskers. If None is provided,
    /// no stroke will be drawn around the boxes.
    ///
    /// # Arguments
    ///
    /// * `stroke` - Optional SingleColor for the box outlines and whiskers. If None, no stroke is applied
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_box_stroke(mut self, stroke: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke = stroke;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke width for boxes
    ///
    /// Controls the thickness of the outline/border around box shapes and whiskers. Larger values
    /// create more prominent borders and whiskers, while smaller values create thinner ones.
    ///
    /// # Arguments
    ///
    /// * `stroke_width` - A f64 value representing the width of the stroke in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_box_stroke_width(mut self, stroke_width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke_width = stroke_width;
        self.mark = Some(mark);
        self
    }

    /// Set the color for outliers
    ///
    /// Defines the color used to render outlier points. Outliers are data points that fall
    /// outside the range defined by Q1 - 1.5*IQR to Q3 + 1.5*IQR. If None is provided,
    /// the chart will use the same color as the box or colors from the palette.
    ///
    /// # Arguments
    ///
    /// * `color` - Optional SingleColor for outlier points. If None, box color is used
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_outlier_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.outlier_color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the size for outliers
    ///
    /// Controls the size of outlier points rendered on the box plot. Larger values create
    /// more prominent outlier markers, while smaller values create subtler ones.
    ///
    /// # Arguments
    ///
    /// * `size` - A f64 value representing the radius of outlier points in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_outlier_size(mut self, size: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.outlier_size = size;
        self.mark = Some(mark);
        self
    }

    /// Set the width of individual box plots
    ///
    /// Defines the width of individual box plots in data units. This value is used as the base width
    /// for calculating actual box widths, which may be adjusted based on grouping and spacing.
    ///
    /// # Arguments
    ///
    /// * `width` - A f64 value representing the base width of boxes in data units
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_box_width(mut self, width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.width = width;
        self.mark = Some(mark);
        self
    }

    /// Set the spacing between box plots in the same group
    ///
    /// Controls the spacing between box plots within the same group as a ratio of box width.
    /// A value of 0.0 means no spacing, while 1.0 means spacing equal to box width.
    ///
    /// # Arguments
    ///
    /// * `spacing` - A f64 value representing spacing ratio (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_box_spacing(mut self, spacing: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.spacing = spacing;
        self.mark = Some(mark);
        self
    }

    /// Set the total span for each group of box plots
    ///
    /// Defines the total width allocated for each group of box plots in data units. This value
    /// determines how much space is reserved for all boxes within a group, including spacing.
    ///
    /// # Arguments
    ///
    /// * `span` - A f64 value representing the total span for each box group in data units
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    pub fn with_box_span(mut self, span: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.span = span;
        self.mark = Some(mark);
        self
    }

    // Group data by categorical values and calculate box statistics for each subgroup
    fn calculate_grouped_box_stats(
        &self,
    ) -> Result<Vec<(String, String, Option<BoxStats>)>, ChartonError> {
        let x_encoding = self.encoding.x.as_ref().unwrap();
        let y_encoding = self.encoding.y.as_ref().unwrap();

        // Determine grouping and value columns based on orientation
        let (group_col, value_col) = (&x_encoding.field, &y_encoding.field);

        // Get color column for grouping within categories or use a temporary one to avoid duplication code
        let color_field = self
            .encoding
            .color
            .as_ref()
            .map(|enc| enc.field.clone())
            .unwrap_or_else(|| format!("__charton_temp_color_{}", Uuid::now_v7().hyphenated()));

        // If no color encoding exists, create a temporary column with default values
        let working_df = if self.encoding.color.is_none() {
            let temp_color_series =
                Series::new((&color_field).into(), vec!["temp"; self.data.df.height()]);
            self.data
                .df
                .clone()
                .lazy()
                .with_column(lit(temp_color_series).alias(&color_field))
                .collect()?
        } else {
            self.data.df.clone()
        };

        // Get all unique group values and clean them of quotes
        let unique_groups = working_df.column(group_col)?.unique_stable()?;
        let group_values: Vec<String> = unique_groups
            .str()?
            .into_no_null_iter()
            .map(|s| {
                let s_str = s.to_string();
                // Remove quotes if they exist
                if s_str.starts_with('"') && s_str.ends_with('"') && s_str.len() >= 2 {
                    s_str[1..s_str.len() - 1].to_string()
                } else {
                    s_str
                }
            })
            .collect();

        // Get all unique color values if color encoding is used and clean them of quotes
        let color_values = {
            let unique = working_df.column(&color_field)?.unique_stable()?;
            let values: Vec<String> = unique
                .str()?
                .into_no_null_iter()
                .map(|s| {
                    let s_str = s.to_string();
                    // Remove quotes if they exist
                    if s_str.starts_with('"') && s_str.ends_with('"') && s_str.len() >= 2 {
                        s_str[1..s_str.len() - 1].to_string()
                    } else {
                        s_str
                    }
                })
                .collect();
            values
        };

        // Create all expected combinations
        let expected_combinations: Vec<(String, String)> =
            itertools::iproduct!(group_values.iter().cloned(), color_values.iter().cloned())
                .collect();

        // Group by both group and color columns (since we always have a color column now)
        let grouped_stats = working_df
            .clone()
            .lazy()
            .group_by_stable([col(group_col), col(&color_field)])
            .agg([
                col(value_col).min().alias("min"),
                col(value_col)
                    .quantile(lit(0.25), QuantileMethod::Linear)
                    .alias("q1"),
                col(value_col).median().alias("median"),
                col(value_col)
                    .quantile(lit(0.75), QuantileMethod::Linear)
                    .alias("q3"),
                col(value_col).max().alias("max"),
                col(value_col).mean().alias("mean"),
                col(value_col).count().alias("count"),
            ])
            .collect()?;

        // Calculate IQR and outlier bounds
        let stats_with_bounds = grouped_stats
            .lazy()
            .with_columns([
                (col("q3") - col("q1")).alias("iqr"),
                (col("q1") - lit(1.5) * (col("q3") - col("q1"))).alias("lower_bound"),
                (col("q3") + lit(1.5) * (col("q3") - col("q1"))).alias("upper_bound"),
            ])
            .collect()?;

        // Create a mapping of group values to their bounds for outlier detection
        let mut bounds_map = std::collections::HashMap::new();
        let group_series = stats_with_bounds.column(group_col)?;
        let lower_bound_series = stats_with_bounds.column("lower_bound")?;
        let upper_bound_series = stats_with_bounds.column("upper_bound")?;
        let count_series = stats_with_bounds.column("count")?;

        // We always have a color column now, so we always track bounds by both group and color
        let color_series = stats_with_bounds.column(&color_field)?;
        for i in 0..group_series.len() {
            let group_value = group_series.get(i)?.to_string();
            // Clean quotes from group value
            let clean_group_value = if group_value.starts_with('"')
                && group_value.ends_with('"')
                && group_value.len() >= 2
            {
                group_value[1..group_value.len() - 1].to_string()
            } else {
                group_value
            };

            let color_value = color_series.get(i)?.to_string();
            // Clean quotes from color value
            let clean_color_value = if color_value.starts_with('"')
                && color_value.ends_with('"')
                && color_value.len() >= 2
            {
                color_value[1..color_value.len() - 1].to_string()
            } else {
                color_value
            };

            let key = format!("{}_{}", clean_group_value, clean_color_value);
            let lower_bound = lower_bound_series.f64()?.get(i).unwrap();
            let upper_bound = upper_bound_series.f64()?.get(i).unwrap();
            let count = count_series.u32()?.get(i).unwrap();
            bounds_map.insert(key, (lower_bound, upper_bound, count));
        }

        // Identify outliers by joining original data with bounds
        let mut outliers_map = std::collections::HashMap::new();

        // Get original data series
        let orig_group_series = working_df.column(group_col)?;
        let orig_value_series = working_df.column(value_col)?;

        // We always have a color column now, so we always check for outliers by both group and color
        let orig_color_series = working_df.column(&color_field)?;
        for i in 0..orig_group_series.len() {
            let group_value = orig_group_series.get(i)?.to_string();
            // Clean quotes from group value
            let clean_group_value = if group_value.starts_with('"')
                && group_value.ends_with('"')
                && group_value.len() >= 2
            {
                group_value[1..group_value.len() - 1].to_string()
            } else {
                group_value
            };

            let color_value = orig_color_series.get(i)?.to_string();
            // Clean quotes from color value
            let clean_color_value = if color_value.starts_with('"')
                && color_value.ends_with('"')
                && color_value.len() >= 2
            {
                color_value[1..color_value.len() - 1].to_string()
            } else {
                color_value
            };

            let key = format!("{}_{}", clean_group_value, clean_color_value);

            if let Some(&(lower_bound, upper_bound, _)) = bounds_map.get(&key) {
                let value = orig_value_series.f64()?.get(i).unwrap_or(0.0);
                if value < lower_bound || value > upper_bound {
                    outliers_map.entry(key).or_insert_with(Vec::new).push(value);
                }
            }
        }

        // Convert to BoxStats structs, ensuring all expected combinations are represented
        let mut result = Vec::new();
        let group_series = stats_with_bounds.column(group_col)?;
        let min_series = stats_with_bounds.column("min")?;
        let q1_series = stats_with_bounds.column("q1")?;
        let median_series = stats_with_bounds.column("median")?;
        let q3_series = stats_with_bounds.column("q3")?;
        let max_series = stats_with_bounds.column("max")?;
        let mean_series = stats_with_bounds.column("mean")?;
        let _count_series = stats_with_bounds.column("count")?;

        // Create a map of existing results for quick lookup
        let mut existing_results = std::collections::HashMap::new();

        // We always have a color column now, so we always process results with both group and color
        let color_series = stats_with_bounds.column(&color_field)?;
        for i in 0..group_series.len() {
            let group_value = group_series.get(i)?.to_string();
            // Clean quotes from group value
            let clean_group_value = if group_value.starts_with('"')
                && group_value.ends_with('"')
                && group_value.len() >= 2
            {
                group_value[1..group_value.len() - 1].to_string()
            } else {
                group_value
            };

            let color_value = color_series.get(i)?.to_string();
            // Clean quotes from color value
            let clean_color_value = if color_value.starts_with('"')
                && color_value.ends_with('"')
                && color_value.len() >= 2
            {
                color_value[1..color_value.len() - 1].to_string()
            } else {
                color_value
            };

            let min = min_series.f64()?.get(i).unwrap_or(0.0);
            let q1 = q1_series.f64()?.get(i).unwrap_or(0.0);
            let median = median_series.f64()?.get(i).unwrap_or(0.0);
            let q3 = q3_series.f64()?.get(i).unwrap_or(0.0);
            let max = max_series.f64()?.get(i).unwrap_or(0.0);
            let _mean = mean_series.f64()?.get(i).unwrap_or(0.0);

            let key = format!("{}_{}", clean_group_value, clean_color_value);
            let outliers = outliers_map.get(&key).cloned().unwrap_or_else(Vec::new);

            existing_results.insert(
                key,
                (
                    clean_group_value,
                    clean_color_value,
                    BoxStats {
                        min,
                        q1,
                        median,
                        q3,
                        max,
                        outliers,
                        _mean,
                    },
                ),
            );
        }

        // Now iterate through all expected combinations and add results
        for (group_value, color_value) in expected_combinations {
            // This is the key fix - make sure the key format matches between storage and lookup
            let key = format!("{}_{}", group_value, color_value);

            if let Some((group, color, stats)) = existing_results.get(&key) {
                result.push((group.clone(), color.clone(), Some(stats.clone())));
            } else {
                // Add None for missing combinations
                result.push((
                    group_value,
                    color_value,
                    None, // No data for this combination
                ));
            }
        }

        Ok(result)
    }

    // Render box whisker elements with grouping support
    fn render_box_whiskers(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        let box_stats = self.calculate_grouped_box_stats()?;

        // Get unique group labels for positioning while preserving order of appearance
        let unique_groups: Vec<String> = box_stats
            .iter()
            .map(|(group, _, _)| group.clone())
            .collect::<IndexSet<_>>()
            .into_iter()
            .collect();

        // NOTE: We no longer sort to preserve the original data order
        let sorted_groups = unique_groups;

        // Extract the mark from Option
        let mark = self.mark.as_ref().ok_or_else(|| {
            ChartonError::Internal("Mark should exist when rendering rules".to_string())
        })?;

        // Get box plot configuration
        let user_width = mark.width;
        let spacing = mark.spacing; // Spacing between boxes in a group
        let span = mark.span;

        // Render each vertical box
        for (group_idx, group_label) in sorted_groups.iter().enumerate() {
            // Get all subgroups for this group
            let group_boxes: Vec<_> = box_stats
                .iter()
                .filter(|(group, _, _)| group == group_label)
                .collect();

            // Calculate width using the provided formula
            let groups_count = group_boxes.len() as f64;
            // Calculate width using the provided formula (in data coordinates)
            let box_width_data =
                user_width.min(span / (groups_count + (groups_count - 1.0) * spacing));

            // Get positions in pixel coordinates
            let base_position_pixel = (context.x_mapper)(0.0);
            let box_width_position_pixel = (context.x_mapper)(box_width_data);
            let box_width_pixels = (box_width_position_pixel - base_position_pixel).abs();

            // Calculate spacing in pixels
            let spacing_pixels = box_width_pixels * spacing;

            // Use context.x_mapper to properly position boxes based on x values
            let x_position_pixel = (context.x_mapper)(group_idx as f64);

            // Render each box in this group
            for (sub_idx, (_, color_label, stats_opt)) in group_boxes.iter().enumerate() {
                // Skip rendering if there's no data for this combination
                let stats = match stats_opt {
                    Some(s) => s,
                    None => continue, // Skip this box if there's no data
                };

                // Calculate position for this specific box within the group
                let offset_pixel = (sub_idx as f64 - (groups_count - 1.0) / 2.0)
                    * (box_width_pixels + spacing_pixels);
                let x_center_pixel = x_position_pixel + offset_pixel;

                // Transform y values to pixel coordinates
                let min_y_pixel = (context.y_mapper)(stats.min);
                let q1_y_pixel = (context.y_mapper)(stats.q1);
                let median_y_pixel = (context.y_mapper)(stats.median);
                let q3_y_pixel = (context.y_mapper)(stats.q3);
                let max_y_pixel = (context.y_mapper)(stats.max);

                // Determine box properties using encoding values when available, otherwise use mark defaults
                let fill_color = if self.encoding.color.is_some() {
                    // Find the index of color_label in the unique color values
                    let color_index = box_stats
                        .iter()
                        .position(|(_, c, _)| c == color_label)
                        .ok_or_else(|| {
                            ChartonError::Internal("color not found in unique values".to_string())
                        })?;
                    Some(SingleColor::new(&self.mark_palette.get_color(color_index)))
                } else {
                    mark.color.clone()
                };

                // Transform outlier y-values to pixel coordinates
                let outlier_y_coords: Vec<f64> = stats
                    .outliers
                    .iter()
                    .map(|&outlier_val| (context.y_mapper)(outlier_val))
                    .collect();

                if !self.swapped_axes {
                    // Render the box using the box renderer
                    crate::render::box_renderer::render_vertical_box(
                        svg,
                        crate::render::box_renderer::VerticalBoxConfig {
                            x_center: x_center_pixel,
                            min_y: min_y_pixel,
                            q1_y: q1_y_pixel,
                            median_y: median_y_pixel,
                            q3_y: q3_y_pixel,
                            max_y: max_y_pixel,
                            box_width: box_width_pixels,
                            fill_color: fill_color.clone(),
                            stroke_color: mark.stroke.clone(),
                            stroke_width: mark.stroke_width,
                            opacity: mark.opacity,
                            outliers: outlier_y_coords.clone(),
                            outlier_color: mark.outlier_color.clone(),
                            outlier_size: mark.outlier_size,
                        },
                    )?;
                } else {
                    // Render the box using the box renderer
                    crate::render::box_renderer::render_horizontal_box(
                        svg,
                        crate::render::box_renderer::HorizontalBoxConfig {
                            y_center: x_center_pixel,
                            min_x: min_y_pixel,
                            q1_x: q1_y_pixel,
                            median_x: median_y_pixel,
                            q3_x: q3_y_pixel,
                            max_x: max_y_pixel,
                            box_height: box_width_pixels,
                            fill_color: fill_color.clone(),
                            stroke_color: mark.stroke.clone(),
                            stroke_width: mark.stroke_width,
                            opacity: mark.opacity,
                            outliers: outlier_y_coords.clone(),
                            outlier_color: mark.outlier_color.clone(),
                            outlier_size: mark.outlier_size,
                        },
                    )?;
                }
            }
        }

        Ok(())
    }
}

// Implementation of MarkRenderer for Chart<MarkBoxWhisker>
impl crate::chart::common::MarkRenderer for Chart<MarkBoxplot> {
    fn render_marks(
        &self,
        svg: &mut String,
        context: &SharedRenderingContext,
    ) -> Result<(), ChartonError> {
        self.render_box_whiskers(svg, context)
    }
}

impl crate::chart::common::LegendRenderer for Chart<MarkBoxplot> {
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
