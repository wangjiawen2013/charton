use crate::core::layer::Layer;
/// LayeredChart structure - shared properties for all layers
///
/// This struct represents a multi-layer chart that can combine multiple chart layers
/// into a single visualization. It holds shared properties that apply to the entire
/// chart, such as dimensions, margins, theme settings, and axis configurations.
///
/// Each layer in the chart can be a different chart type (e.g., a line chart overlaid
/// on a bar chart) and they all share the same coordinate system and styling properties
/// defined at the LayeredChart level.
///
/// # Fields
///
/// * `width` - Width of the chart in pixels
/// * `height` - Height of the chart in pixels
/// * `left_margin` - Left margin as a proportion of total width (0.0 to 1.0)
/// * `right_margin` - Right margin as a proportion of total width (0.0 to 1.0)
/// * `top_margin` - Top margin as a proportion of total height (0.0 to 1.0)
/// * `bottom_margin` - Bottom margin as a proportion of total height (0.0 to 1.0)
/// * `theme` - Theme settings for colors, fonts, and other styling properties
/// * `title` - Optional chart title
/// * `layers` - Vector of boxed Layer trait objects representing individual chart layers
/// * `x_domain_min` - Optional custom minimum value for x-axis domain
/// * `x_domain_max` - Optional custom maximum value for x-axis domain
/// * `x_label` - Optional label for x-axis
/// * `x_tick_values` - Optional custom tick values for continuous x-axis
/// * `x_tick_labels` - Optional custom tick labels for discrete x-axis
/// * `y_domain_min` - Optional custom minimum value for y-axis domain
/// * `y_domain_max` - Optional custom maximum value for y-axis domain
/// * `y_label` - Optional label for y-axis
/// * `y_tick_values` - Optional custom tick values for continuous y-axis
/// * `y_tick_labels` - Optional custom tick labels for discrete y-axis
/// * `flipped` - Flag indicating whether x and y axes should be swapped
/// * `legend` - Optional flag to show/hide legend
/// * `legend_title` - Optional title for the legend
/// * `background` - Optional background color
/// * `axes` - Optional flag to show/hide axes
pub struct LayeredChart {
    width: u32,
    height: u32,
    left_margin: f64,
    right_margin: f64,
    top_margin: f64,
    bottom_margin: f64,
    theme: Theme,

    title: Option<String>,

    layers: Vec<Box<dyn Layer>>,
    _coord_system: Option<Cartesian>, // Maybe used in the future in case different coordinate systems are needed

    x_domain_min: Option<f64>, // Functions when creating continuous axis
    x_domain_max: Option<f64>, // Functions when creating continuous axis
    x_label: Option<String>,   // x axis label
    x_tick_values: Option<Vec<f64>>, // Functions when rendering continuous axis
    x_tick_labels: Option<Vec<String>>, // Functions when rendering discrete axis

    y_domain_min: Option<f64>, // Functions when creating continuous axis
    y_domain_max: Option<f64>, // Functions when creating continuous axis
    y_label: Option<String>,   // y axis label
    y_tick_values: Option<Vec<f64>>, // Functions when rendering continuous axis
    y_tick_labels: Option<Vec<String>>, // Functions when rendering discrete axis

    flipped: bool,

    legend: Option<bool>,
    legend_title: Option<String>,

    background: Option<String>,
    axes: Option<bool>,
}

impl Default for LayeredChart {
    fn default() -> Self {
        Self::new()
    }
}

impl LayeredChart {
    /// Create a new LayeredChart with default settings
    ///
    /// Initializes a LayeredChart with the following default values:
    /// - Width: 500 pixels
    /// - Height: 400 pixels
    /// - Margins: 15% left, 10% right, 10% top, 15% bottom
    /// - Default theme
    /// - No title
    /// - No layers
    /// - Automatic axis scaling
    /// - White background
    /// - Axes enabled by default
    /// - Legend disabled by default
    ///
    /// # Returns
    ///
    /// Returns a new LayeredChart instance with default configuration
    ///
    /// # Example
    ///
    /// ```
    /// use charton::prelude::*;
    ///
    /// let chart = LayeredChart::new();
    /// ```
    pub fn new() -> Self {
        Self {
            width: 500,
            height: 400,
            left_margin: 0.15,
            right_margin: 0.10,
            top_margin: 0.10,
            bottom_margin: 0.15,
            theme: Theme::default(),

            title: None,

            layers: Vec::new(),
            _coord_system: None,

            x_domain_min: None,
            x_domain_max: None,
            x_label: None,
            x_tick_values: None,
            x_tick_labels: None,
            y_domain_min: None,
            y_domain_max: None,
            y_label: None,
            y_tick_values: None,
            y_tick_labels: None,

            flipped: false,

            legend: None,
            legend_title: None,

            background: Some("white".to_string()),
            axes: None,
        }
    }

    /// Set the size of the chart
    ///
    /// Configures the overall dimensions of the chart in pixels. This affects the
    /// total area available for the plot, including margins, axes, labels, and legend.
    ///
    /// # Arguments
    ///
    /// * `width` - The width of the chart in pixels
    /// * `height` - The height of the chart in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set the left margin of the chart
    ///
    /// Configures the left margin as a proportion of the total chart width (0.0 to 1.0).
    /// This margin provides space for the y-axis labels, ticks, and title. Larger values
    /// create more space for these elements.
    ///
    /// # Arguments
    ///
    /// * `margin` - The left margin as a proportion of total width (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_left_margin(mut self, margin: f64) -> Self {
        self.left_margin = margin;
        self
    }

    /// Set the right margin of the chart
    ///
    /// Configures the right margin as a proportion of the total chart width (0.0 to 1.0).
    /// This margin provides space for the legend and right-side labels. Larger values
    /// create more space for these elements.
    ///
    /// # Arguments
    ///
    /// * `margin` - The right margin as a proportion of total width (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_right_margin(mut self, margin: f64) -> Self {
        self.right_margin = margin;
        self
    }

    /// Set the top margin of the chart
    ///
    /// Configures the top margin as a proportion of the total chart height (0.0 to 1.0).
    /// This margin provides space for the chart title and top labels. Larger values
    /// create more space for these elements.
    ///
    /// # Arguments
    ///
    /// * `margin` - The top margin as a proportion of total height (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_top_margin(mut self, margin: f64) -> Self {
        self.top_margin = margin;
        self
    }

    /// Set the bottom margin of the chart
    ///
    /// Configures the bottom margin as a proportion of the total chart height (0.0 to 1.0).
    /// This margin provides space for the x-axis labels, ticks, and title. Larger values
    /// create more space for these elements.
    ///
    /// # Arguments
    ///
    /// * `margin` - The bottom margin as a proportion of total height (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_bottom_margin(mut self, margin: f64) -> Self {
        self.bottom_margin = margin;
        self
    }

    /// Set the theme for the chart
    ///
    /// Applies a complete theme to the chart, which controls colors, fonts, stroke widths,
    /// and other visual styling properties. The theme affects all aspects of the chart
    /// including axes, labels, ticks, and legend.
    ///
    /// # Arguments
    ///
    /// * `theme` - The Theme to apply to the chart
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the title of the chart
    ///
    /// Adds a title to the chart which will be displayed at the top. The title is
    /// rendered using the theme's title font settings.
    ///
    /// # Arguments
    ///
    /// * `title` - The title text for the chart
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the font size for the chart title
    ///
    /// Configures the font size used for rendering the chart title in pixels.
    ///
    /// # Arguments
    ///
    /// * `size` - The font size in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_title_font_size(mut self, size: u32) -> Self {
        self.theme.title_font_size = size;
        self
    }

    /// Set the font family for the chart title
    ///
    /// Configures the font family used for rendering the chart title.
    ///
    /// # Arguments
    ///
    /// * `family` - The font family name
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_title_font_family(mut self, family: impl Into<String>) -> Self {
        self.theme.title_font_family = family.into();
        self
    }

    /// Set the color for the chart title
    ///
    /// Configures the color used for rendering the chart title.
    ///
    /// # Arguments
    ///
    /// * `color` - The color for the title (can be a named color like "red" or hex value like "#FF0000")
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_title_color(mut self, color: impl Into<String>) -> Self {
        self.theme.title_color = color.into();
        self
    }

    /// Set the minimum padding for x axis
    ///
    /// Configures the minimum padding used for axis scaling. This padding ensures
    /// that data points near the edges of the chart are not cut off.
    ///
    /// # Arguments
    ///
    /// * `padding` - The minimum padding value
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_x_axis_padding_min(mut self, padding: f64) -> Self {
        self.theme.x_axis_padding_min = padding;
        self
    }

    /// Set the maximum padding for x axis
    ///
    /// Configures the maximum padding used for axis scaling. This padding ensures
    /// that data points near the edges of the chart are not cut off.
    ///
    /// # Arguments
    ///
    /// * `padding` - The maximum padding value
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_x_axis_padding_max(mut self, padding: f64) -> Self {
        self.theme.x_axis_padding_max = padding;
        self
    }

    /// Set the minimum padding for y axis
    ///
    /// Configures the minimum padding used for axis scaling. This padding ensures
    /// that data points near the edges of the chart are not cut off.
    ///
    /// # Arguments
    ///
    /// * `padding` - The minimum padding value
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_y_axis_padding_min(mut self, padding: f64) -> Self {
        self.theme.y_axis_padding_min = padding;
        self
    }

    /// Set the maximum padding for y axis
    ///
    /// Configures the maximum padding used for axis scaling. This padding ensures
    /// that data points near the edges of the chart are not cut off.
    ///
    /// # Arguments
    ///
    /// * `padding` - The maximum padding value
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_y_axis_padding_max(mut self, padding: f64) -> Self {
        self.theme.y_axis_padding_max = padding;
        self
    }

    /// Set the stroke width for the axes
    ///
    /// Configures the stroke width (line thickness) used for drawing the chart axes.
    ///
    /// # Arguments
    ///
    /// * `width` - The stroke width in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_axis_stroke_width(mut self, width: f64) -> Self {
        self.theme.axis_stroke_width = width;
        self
    }

    /// Set the minimum value for the x-axis domain
    ///
    /// Configures the minimum value for the x-axis domain. This overrides the
    /// automatic domain calculation based on the data.
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum value for the x-axis domain
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_x_domain_min(mut self, min: f64) -> Self {
        self.x_domain_min = Some(min);
        self
    }

    /// Set the maximum value for the x-axis domain
    ///
    /// Configures the maximum value for the x-axis domain. This overrides the
    /// automatic domain calculation based on the data.
    ///
    /// # Arguments
    ///
    /// * `max` - The maximum value for the x-axis domain
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_x_domain_max(mut self, max: f64) -> Self {
        self.x_domain_max = Some(max);
        self
    }

    /// Set both minimum and maximum values for the x-axis domain
    ///
    /// Configures both the minimum and maximum values for the x-axis domain.
    /// This overrides the automatic domain calculation based on the data.
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum value for the x-axis domain
    /// * `max` - The maximum value for the x-axis domain
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_x_domain(mut self, min: f64, max: f64) -> Self {
        self.x_domain_min = Some(min);
        self.x_domain_max = Some(max);
        self
    }

    /// Set the label for the x-axis
    ///
    /// Configures the label text displayed alongside the x-axis. If not set,
    /// the field name from the x encoding will be used as the default label.
    ///
    /// # Arguments
    ///
    /// * `label` - The label text for the x-axis
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_x_label(mut self, label: impl Into<String>) -> Self {
        self.x_label = Some(label.into());
        self
    }

    /// Set the padding for the x-axis label
    ///
    /// Configures the spacing between the x-axis label and the axis line.
    ///
    /// # Arguments
    ///
    /// * `padding` - The padding value in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_x_label_padding(mut self, padding: f64) -> Self {
        self.theme.x_label_padding = padding;
        self
    }

    /// Set the angle for the x-axis label
    ///
    /// Configures the rotation angle of the x-axis label text.
    ///
    /// # Arguments
    ///
    /// * `angle` - The rotation angle in degrees
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_x_label_angle(mut self, angle: f64) -> Self {
        self.theme.label_angle = angle;
        self
    }

    /// Set custom tick values for continuous x-axis
    ///
    /// Configures specific tick positions for continuous x-axis scales.
    /// This overrides the automatic tick generation.
    ///
    /// # Arguments
    ///
    /// * `values` - A vector of f64 values specifying tick positions
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_x_tick_values(mut self, values: Vec<f64>) -> Self {
        self.x_tick_values = Some(values);
        self
    }

    /// Set custom tick labels for discrete x-axis
    ///
    /// Configures specific labels for discrete x-axis scales. This overrides the
    /// automatic label generation and allows you to specify custom text labels
    /// for each tick position.
    ///
    /// # Arguments
    ///
    /// * `labels` - A vector of strings or values that can be converted to strings,
    ///   specifying the labels for each tick position
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_x_tick_labels(mut self, labels: Vec<impl Into<String>>) -> Self {
        self.x_tick_labels = Some(labels.into_iter().map(Into::into).collect());
        self
    }

    /// Set the rotation angle for x-axis tick labels
    ///
    /// Configures the rotation angle of the x-axis tick label text. This is useful
    /// when labels are long and would otherwise overlap.
    ///
    /// # Arguments
    ///
    /// * `angle` - The rotation angle in degrees (e.g., 45.0 for a 45-degree rotation)
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_x_tick_label_angle(mut self, angle: f64) -> Self {
        self.theme.x_tick_label_angle = angle;
        self
    }

    /// Set the minimum value for the y-axis domain
    ///
    /// Configures the minimum value for the y-axis domain. This overrides the
    /// automatic domain calculation based on the data.
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum value for the y-axis domain
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_y_domain_min(mut self, min: f64) -> Self {
        self.y_domain_min = Some(min);
        self
    }

    /// Set the maximum value for the y-axis domain
    ///
    /// Configures the maximum value for the y-axis domain. This overrides the
    /// automatic domain calculation based on the data.
    ///
    /// # Arguments
    ///
    /// * `max` - The maximum value for the y-axis domain
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_y_domain_max(mut self, max: f64) -> Self {
        self.y_domain_max = Some(max);
        self
    }

    /// Set both minimum and maximum values for the y-axis domain
    ///
    /// Configures both the minimum and maximum values for the y-axis domain.
    /// This overrides the automatic domain calculation based on the data.
    ///
    /// # Arguments
    ///
    /// * `min` - The minimum value for the y-axis domain
    /// * `max` - The maximum value for the y-axis domain
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_y_domain(mut self, min: f64, max: f64) -> Self {
        self.y_domain_min = Some(min);
        self.y_domain_max = Some(max);
        self
    }

    /// Set the label for the y-axis
    ///
    /// Configures the label text displayed alongside the y-axis. If not set,
    /// the field name from the y encoding will be used as the default label.
    ///
    /// # Arguments
    ///
    /// * `label` - The label text for the y-axis
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_y_label(mut self, label: impl Into<String>) -> Self {
        self.y_label = Some(label.into());
        self
    }

    /// Set the padding for the y-axis label
    ///
    /// Configures the spacing between the y-axis label and the axis line.
    ///
    /// # Arguments
    ///
    /// * `padding` - The padding value in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_y_label_padding(mut self, padding: f64) -> Self {
        self.theme.y_label_padding = padding;
        self
    }

    /// Set the angle for the y-axis label
    ///
    /// Configures the rotation angle of the y-axis label text.
    ///
    /// # Arguments
    ///
    /// * `angle` - The rotation angle in degrees
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_y_label_angle(mut self, angle: f64) -> Self {
        self.theme.label_angle = angle;
        self
    }

    /// Set custom tick values for continuous y-axis
    ///
    /// Configures specific tick positions for continuous y-axis scales.
    /// This overrides the automatic tick generation.
    ///
    /// # Arguments
    ///
    /// * `values` - A vector of f64 values specifying tick positions
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_y_tick_values(mut self, values: Vec<f64>) -> Self {
        self.y_tick_values = Some(values);
        self
    }

    /// Set custom tick labels for discrete y-axis
    ///
    /// Configures specific labels for discrete y-axis scales. This overrides the
    /// automatic label generation and allows you to specify custom text labels
    /// for each tick position.
    ///
    /// # Arguments
    ///
    /// * `labels` - A vector of strings or values that can be converted to strings,
    ///   specifying the labels for each tick position
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_y_tick_labels(mut self, labels: Vec<impl Into<String>>) -> Self {
        self.y_tick_labels = Some(labels.into_iter().map(Into::into).collect());
        self
    }

    /// Set the rotation angle for y-axis tick labels
    ///
    /// Configures the rotation angle of the y-axis tick label text.
    ///
    /// # Arguments
    ///
    /// * `angle` - The rotation angle in degrees
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_y_tick_label_angle(mut self, angle: f64) -> Self {
        self.theme.y_tick_label_angle = angle;
        self
    }

    /// Set the stroke width for axis ticks
    ///
    /// Configures the stroke width (line thickness) used for drawing axis ticks.
    ///
    /// # Arguments
    ///
    /// * `width` - The stroke width in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_tick_stroke_width(mut self, width: f64) -> Self {
        self.theme.tick_stroke_width = width;
        self
    }

    /// Set the padding for tick labels
    ///
    /// Configures the spacing between tick labels and their corresponding ticks.
    ///
    /// # Arguments
    ///
    /// * `padding` - The padding value in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_tick_label_padding(mut self, padding: f64) -> Self {
        self.theme.tick_label_padding = padding;
        self
    }

    /// Set the font size for tick labels
    ///
    /// Configures the font size used for rendering tick labels in pixels.
    ///
    /// # Arguments
    ///
    /// * `size` - The font size in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_tick_label_font_size(mut self, size: u32) -> Self {
        self.theme.tick_label_font_size = size;
        self
    }

    /// Set the font family for tick labels
    ///
    /// Configures the font family used for rendering tick labels.
    ///
    /// # Arguments
    ///
    /// * `family` - The font family name
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_tick_label_font_family(mut self, family: impl Into<String>) -> Self {
        self.theme.tick_label_font_family = family.into();
        self
    }

    /// Set the color for tick labels
    ///
    /// Configures the color used for rendering tick labels.
    ///
    /// # Arguments
    ///
    /// * `color` - The color for tick labels (can be a named color like "red" or hex value like "#FF0000")
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_tick_label_color(mut self, color: impl Into<String>) -> Self {
        self.theme.tick_label_color = color.into();
        self
    }

    /// Set the font size for axis labels
    ///
    /// Configures the font size used for rendering axis labels in pixels.
    ///
    /// # Arguments
    ///
    /// * `size` - The font size in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_label_font_size(mut self, size: u32) -> Self {
        self.theme.label_font_size = size;
        self
    }

    /// Set the font family for axis labels
    ///
    /// Configures the font family used for rendering axis labels.
    ///
    /// # Arguments
    ///
    /// * `family` - The font family name
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_label_font_family(mut self, family: impl Into<String>) -> Self {
        self.theme.label_font_family = family.into();
        self
    }

    /// Set the color for axis labels
    ///
    /// Configures the color used for rendering axis labels.
    ///
    /// # Arguments
    ///
    /// * `color` - The color for axis labels (can be a named color like "red" or hex value like "#FF0000")
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_label_color(mut self, color: impl Into<String>) -> Self {
        self.theme.label_color = color.into();
        self
    }

    /// Swap the x and y axes of the chart
    ///
    /// Transposes the chart so that the x-axis becomes the y-axis and vice versa. This
    /// is useful for creating horizontal bar charts, horizontal box plots, or any other
    /// chart where you want to flip the orientation.
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn swap_axes(mut self) -> Self {
        self.swapped_axes = true;
        self
    }

    /// Show or hide the legend
    ///
    /// Controls whether the legend is displayed on the chart.
    ///
    /// # Arguments
    ///
    /// * `show` - True to show the legend, false to hide it
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_legend(mut self, show: bool) -> Self {
        self.legend = Some(show);
        self
    }

    /// Set the title for the legend
    ///
    /// Configures the title text displayed above the legend. If not set,
    /// no title will be shown for the legend.
    ///
    /// # Arguments
    ///
    /// * `title` - The title text for the legend
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_legend_title(mut self, title: impl Into<String>) -> Self {
        self.legend_title = Some(title.into());
        self
    }

    /// Set the font size for legend titles and labels
    ///
    /// Configures the font size used for rendering legend titles and labels in pixels.
    ///
    /// # Arguments
    ///
    /// * `size` - The font size in pixels
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_legend_font_size(mut self, size: u32) -> Self {
        self.theme.legend_font_size = Some(size);
        self
    }

    /// Set the font family for legend titles and labels
    ///
    /// Configures the font family used for rendering legend titles and labels.
    ///
    /// # Arguments
    ///
    /// * `family` - The font family name
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_legend_font_family(mut self, family: impl Into<String>) -> Self {
        self.theme.legend_font_family = Some(family.into());
        self
    }

    /// Set the background color of the chart
    ///
    /// Configures the background color of the entire chart area.
    ///
    /// # Arguments
    ///
    /// * `background` - The background color (can be a named color like "red" or hex value like "#FF0000")
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_background(mut self, background: impl Into<String>) -> Self {
        self.background = Some(background.into());
        self
    }

    /// Show or hide the axes
    ///
    /// Controls whether the chart axes are displayed.
    ///
    /// # Arguments
    ///
    /// * `show` - True to show the axes, false to hide them
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining
    ///
    pub fn with_axes(mut self, show: bool) -> Self {
        self.axes = Some(show);
        self
    }

    // Get the x-axis continuous bounds from all layers
    fn get_x_continuous_bounds_from_layers(&self) -> Result<(f64, f64), ChartonError> {
        if self.layers.is_empty() {
            return Ok((0.0, 1.0));
        }

        let mut global_x_min = f64::INFINITY;
        let mut global_x_max = f64::NEG_INFINITY;

        // Iterate through all layers
        for layer in &self.layers {
            let (x_min, x_max) = layer.get_x_continuous_bounds()?;
            global_x_min = global_x_min.min(x_min);
            global_x_max = global_x_max.max(x_max);
        }

        // Handle edge case where min and max are the same
        if (global_x_max - global_x_min).abs() < 1e-12 {
            let offset = 0.5;
            global_x_min -= offset;
            global_x_max += offset;
        }

        Ok((global_x_min, global_x_max))
    }

    // Get the y-axis continuous bounds from all layers
    fn get_y_continuous_bounds_from_layers(&self) -> Result<(f64, f64), ChartonError> {
        if self.layers.is_empty() {
            return Ok((0.0, 1.0));
        }

        let mut global_y_min = f64::INFINITY;
        let mut global_y_max = f64::NEG_INFINITY;

        // Iterate through all layers
        for layer in &self.layers {
            let (y_min, y_max) = layer.get_y_continuous_bounds()?;
            global_y_min = global_y_min.min(y_min);
            global_y_max = global_y_max.max(y_max);
        }

        // Handle edge case where all values are the same
        if (global_y_max - global_y_min).abs() < 1e-12 {
            let offset = 0.5;
            global_y_min -= offset;
            global_y_max += offset;
        }

        Ok((global_y_min, global_y_max))
    }

    // Get discrete x labels from all layers
    fn get_x_discrete_tick_labels_from_layers(&self) -> Result<Option<Vec<String>>, ChartonError> {
        if self.layers.is_empty() {
            return Ok(None);
        }

        // Collect all unique labels for discrete x-axis, preserving insertion order
        let mut all_x_labels: IndexSet<String> = IndexSet::new();

        // Iterate through all layers
        for layer in &self.layers {
            // Use if let in case charts that don't have x encoding (like pie charts)
            if let Some(labels) = layer.get_x_discrete_tick_labels()? {
                for label in labels {
                    all_x_labels.insert(label);
                }
            }
        }

        if !all_x_labels.is_empty() {
            Ok(Some(all_x_labels.into_iter().collect()))
        } else {
            Ok(None)
        }
    }

    // Get discrete y labels from all layers
    fn get_y_discrete_tick_labels_from_layers(&self) -> Result<Option<Vec<String>>, ChartonError> {
        if self.layers.is_empty() {
            return Ok(None);
        }

        // Collect all unique labels for discrete y-axis, preserving insertion order
        let mut all_y_labels: IndexSet<String> = IndexSet::new();

        // Iterate through all layers
        for layer in &self.layers {
            // Use if let in case charts that don't have y encoding (like pie charts)
            if let Some(labels) = layer.get_y_discrete_tick_labels()? {
                for label in labels {
                    all_y_labels.insert(label);
                }
            }
        }

        if !all_y_labels.is_empty() {
            Ok(Some(all_y_labels.into_iter().collect()))
        } else {
            Ok(None)
        }
    }

    // Get the x-axis label from layers
    fn get_x_axis_label_from_layers(&self) -> String {
        // First check if we have an explicit label set on the chart
        if let Some(ref label) = self.x_label {
            return label.clone();
        }

        // Try to take the label from the first layer that has a label or field name defined
        for layer in &self.layers {
            // Use if let in case charts that don't have x encoding (like pie charts)
            if let Some(field) = layer.get_x_encoding_field() {
                return field;
            }
        }

        // Default fallback
        "X".to_string()
    }

    // Get the y-axis label from layers
    fn get_y_axis_label_from_layers(&self) -> String {
        // First check if we have an explicit label set on the chart
        if let Some(ref label) = self.y_label {
            return label.clone();
        }

        // Try to get label from the first layers
        for layer in &self.layers {
            // Use if let in case charts that don't have y encoding (like pie charts)
            if let Some(field) = layer.get_y_encoding_field() {
                return field;
            }
        }

        // Default fallback
        "Y".to_string()
    }

    // Get the x-axis data type from all layers
    fn get_x_data_type_from_layers(&self) -> Option<polars::datatypes::DataType> {
        if self.layers.is_empty() {
            return None;
        }

        // Iterate through all layers to find the first one with x encoding
        for layer in &self.layers {
            if let Some(data_type) = layer.get_x_data_type() {
                return Some(data_type);
            }
        }

        None
    }

    // Get the y-axis data type from all layers
    fn get_y_data_type_from_layers(&self) -> Option<polars::datatypes::DataType> {
        if self.layers.is_empty() {
            return None;
        }

        // Iterate through all layers to find the first one with y encoding
        for layer in &self.layers {
            if let Some(data_type) = layer.get_y_data_type() {
                return Some(data_type);
            }
        }

        None
    }

    /// Add a layer to the chart
    ///
    /// Adds a new chart layer to create a multi-layered visualization. Each layer can represent
    /// a different data series or chart type, allowing for complex composite visualizations like
    /// line charts overlaid on bar charts.
    ///
    /// Layers are rendered in the order they are added, with the first layer at the bottom
    /// and subsequent layers stacked on top.
    ///
    /// # Arguments
    ///
    /// * `layer` - A Chart instance representing the layer to be added
    ///
    /// # Returns
    ///
    /// Returns the LayeredChart instance for method chaining
    ///
    /// # Example
    ///
    /// ```
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df1 = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
    /// let df2 = df!["x" => [1, 2, 3], "y" => [5, 15, 25]]?;
    ///
    /// let base_layer = Chart::<MarkBar>::build(&df1)?
    ///     .mark_bar()
    ///     .encode(x("x"), y("y"))?;
    ///     
    /// let overlay_layer = Chart::<MarkLine>::build(&df2)?
    ///     .mark_line()
    ///     .encode(x("x"), y("y"))?;
    ///
    /// let chart = LayeredChart::new()
    ///     .add_layer(base_layer)
    ///     .add_layer(overlay_layer);
    /// ```
    pub fn add_layer<T: crate::mark::Mark + 'static>(mut self, layer: Chart<T>) -> Self
    where
        Chart<T>: Layer,
    {
        // Check if the layer has data before adding it
        if layer.data.df.height() > 0 {
            self.layers.push(Box::new(layer));
        }
        // If layer is empty, silently ignore it
        self
    }

    // Calculate the effective right margin based on legend width requirements
    fn effective_right_margin(&self) -> f64 {
        let draw_x0 = self.left_margin * self.width as f64;
        let min_plot_width = 200.0;

        // Calculate required legend width
        let mut total_required_legend_width: f64 = 0.0;
        for layer in &self.layers {
            let layer_legend_width = layer.calculate_legend_width(
                &self.theme,
                self.height as f64,
                self.top_margin,
                self.bottom_margin,
            );
            total_required_legend_width = total_required_legend_width.max(layer_legend_width);
            // Add 10 pixels padding
            total_required_legend_width += 10.0;
        }

        let base_right_margin_width = self.right_margin * self.width as f64;
        let initial_plot_w = self.width as f64 - draw_x0 - base_right_margin_width;

        if initial_plot_w < min_plot_width {
            let required_right_margin_width = self.width as f64 - draw_x0 - min_plot_width;
            required_right_margin_width / self.width as f64
        } else if total_required_legend_width > base_right_margin_width {
            let additional_width_needed = total_required_legend_width - base_right_margin_width;
            let new_plot_w = initial_plot_w - additional_width_needed;

            if new_plot_w >= min_plot_width {
                (total_required_legend_width / self.width as f64).max(self.right_margin)
            } else {
                let max_compression = initial_plot_w - min_plot_width;
                let actual_compression = additional_width_needed.min(max_compression);
                let final_right_margin =
                    base_right_margin_width + (additional_width_needed - actual_compression);
                final_right_margin / self.width as f64
            }
        } else {
            self.right_margin
        }
    }

    // Create shared coordinate system
    fn create_shared_coord_system(&self) -> Result<Cartesian, ChartonError> {
        // Determine plot dimensions
        let plot_w = (1.0 - self.left_margin - self.effective_right_margin()) * self.width as f64;
        let plot_h = (1.0 - self.top_margin - self.bottom_margin) * self.height as f64;

        // Get axis labels
        let x_label = self.get_x_axis_label_from_layers();
        let y_label = self.get_y_axis_label_from_layers();

        // Determine scales - derive from layers or fallback to linear for charts without x, y encoding (like pie charts)
        let x_scale = self.get_x_scale_from_layers()?.unwrap_or(Scale::Linear);
        let y_scale = self.get_y_scale_from_layers()?.unwrap_or(Scale::Linear);

        // Create axes with appropriate labels and scales
        let mut x_axis = crate::axis::Axis::new(x_label, x_scale.clone());
        let mut y_axis = crate::axis::Axis::new(y_label, y_scale.clone());

        // For discrete axes, set tick values and labels in order as automatically computed ticks
        if matches!(x_scale, Scale::Discrete) {
            if let Some(x_tick_labels) = self.get_x_discrete_tick_labels_from_layers()? {
                x_axis.compute_discrete_ticks(x_tick_labels)?;
            }
            // If explicit tick labels are provided for discrete axis, use them to find indices
            if let Some(ref explicit_labels) = self.x_tick_labels {
                // Find indices of explicit labels in the automatic ticks
                let explicit_ticks: Vec<(f64, String)> = explicit_labels
                    .iter()
                    .filter_map(|label| {
                        // Find the position of this label in the automatic ticks
                        x_axis
                            .automatic_ticks
                            .ticks
                            .iter()
                            .position(|tick| tick.label == *label)
                            .map(|index| (index as f64, label.clone()))
                    })
                    .collect();

                // Set explicit ticks using the Axis method
                x_axis = x_axis.with_explicit_ticks(explicit_ticks);
            }
        } else {
            // For continuous x-axis, get bounds
            let (x_min, x_max) = self.get_x_continuous_bounds_from_layers()?;
            // Use custom domain if specified at LayeredChart level, otherwise use data bounds
            let effective_x_min = self.x_domain_min.unwrap_or(x_min);
            let effective_x_max = self.x_domain_max.unwrap_or(x_max);
            x_axis.compute_continuous_ticks(effective_x_min, effective_x_max, plot_w as u32)?;

            // If explicit tick values are provided for continuous axis, use them with string representation as labels
            if let Some(ref explicit_values) = self.x_tick_values {
                let explicit_ticks: Vec<(f64, String)> = explicit_values
                    .iter()
                    .map(|&value| {
                        let value_str = format!("{:?}", value); // This preserves the decimal format
                        let parts: Vec<&str> = value_str.split('.').collect();

                        let formatted_str = if parts.len() == 2 {
                            // Has decimal point
                            format!("{}.{}", parts[0], parts[1])
                        } else {
                            // No decimal point
                            parts[0].to_string()
                        };

                        (value, formatted_str)
                    })
                    .collect();
                x_axis = x_axis.with_explicit_ticks(explicit_ticks);
            }
        }

        // For discrete axes, set tick values and labels in order as automatically computed ticks
        if matches!(y_scale, Scale::Discrete) {
            if let Some(y_tick_labels) = self.get_y_discrete_tick_labels_from_layers()? {
                y_axis.compute_discrete_ticks(y_tick_labels)?;
            }

            // If explicit tick labels are provided for discrete axis, use them to find indices
            if let Some(ref explicit_labels) = self.y_tick_labels {
                // Find indices of explicit labels in the automatic ticks
                let explicit_ticks: Vec<(f64, String)> = explicit_labels
                    .iter()
                    .filter_map(|label| {
                        // Find the position of this label in the automatic ticks
                        y_axis
                            .automatic_ticks
                            .ticks
                            .iter()
                            .position(|tick| tick.label == *label)
                            .map(|index| (index as f64, label.clone()))
                    })
                    .collect();

                // Set explicit ticks using the Axis method
                y_axis = y_axis.with_explicit_ticks(explicit_ticks);
            }
        } else {
            // For continuous y-axis, get bounds
            let (y_min, y_max) = self.get_y_continuous_bounds_from_layers()?;
            // Use custom domain if specified at LayeredChart level, otherwise use data bounds
            let effective_y_min = self.y_domain_min.unwrap_or(y_min);
            let effective_y_max = self.y_domain_max.unwrap_or(y_max);
            y_axis.compute_continuous_ticks(effective_y_min, effective_y_max, plot_h as u32)?;

            // If explicit tick values are provided for continuous axis, use them with string representation as labels
            if let Some(ref explicit_values) = self.y_tick_values {
                let explicit_ticks: Vec<(f64, String)> = explicit_values
                    .iter()
                    .map(|&value| {
                        let value_str = format!("{:?}", value); // This preserves the decimal format
                        let parts: Vec<&str> = value_str.split('.').collect();

                        let formatted_str = if parts.len() == 2 {
                            // Has decimal point
                            format!("{}.{}", parts[0], parts[1])
                        } else {
                            // No decimal point
                            parts[0].to_string()
                        };

                        (value, formatted_str)
                    })
                    .collect();
                y_axis = y_axis.with_explicit_ticks(explicit_ticks);
            }
        }

        // Determine x axis padding - use preferred padding from layers if available
        let x_axis_padding_min = self
            .layers
            .iter()
            .filter_map(|layer| layer.preferred_x_axis_padding_min())
            .next()
            .unwrap_or(self.theme.x_axis_padding_min);

        let x_axis_padding_max = self
            .layers
            .iter()
            .filter_map(|layer| layer.preferred_x_axis_padding_max())
            .next()
            .unwrap_or(self.theme.x_axis_padding_max);

        // Determine y axis padding - use preferred padding from layers if available
        let y_axis_padding_min = self
            .layers
            .iter()
            .filter_map(|layer| layer.preferred_y_axis_padding_min())
            .next()
            .unwrap_or(self.theme.y_axis_padding_min);

        let y_axis_padding_max = self
            .layers
            .iter()
            .filter_map(|layer| layer.preferred_y_axis_padding_max())
            .next()
            .unwrap_or(self.theme.y_axis_padding_max);

        // Create coordinate system using theme.axis_padding - use appropriate constructor based on swapped flag
        let coord_system = Cartesian::new(
            x_axis,
            y_axis,
            x_axis_padding_min,
            x_axis_padding_max,
            y_axis_padding_min,
            y_axis_padding_max,
        );

        Ok(coord_system)
    }

    fn render(&self, svg: &mut String) -> Result<(), ChartonError> {
        // If there are no layers, create a blank chart with the specified dimensions
        if self.layers.is_empty() {
            return Ok(());
        }

        // Create the coordinate system first
        let coord_system = self.create_shared_coord_system()?;
        // Calculate draw offsets based on margins
        let draw_x0 = self.left_margin * self.width as f64;
        let draw_y0 = self.top_margin * self.height as f64;
        let plot_w = (1.0 - self.left_margin - self.effective_right_margin()) * self.width as f64;
        let plot_h = (1.0 - self.bottom_margin - self.top_margin) * self.height as f64;

        // Render title if it exists
        if let Some(ref title) = self.title {
            let title_x = self.width as f64 / 2.0; // Center horizontally
            let title_y = self.theme.title_font_size as f64; // Position at top with padding based on font size
            let font_size = self.theme.title_font_size;
            let font_family = &self.theme.title_font_family;
            let title_color = &self.theme.title_color;

            writeln!(
                svg,
                r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}" text-anchor="middle">{}</text>"#,
                title_x, title_y, font_size, font_family, title_color, title
            )?;
        }

        // Create proper mappers based on the coordinate system and chart properties
        let (x_mapper, y_mapper) = if self.swapped_axes {
            // When axes are swapped, x maps to vertical pixels and y maps to horizontal pixels
            let x_mapper: Box<dyn Fn(f64) -> f64> =
                Box::new(coord_system.x_data_to_vertical_pixels(draw_y0, plot_h));
            let y_mapper: Box<dyn Fn(f64) -> f64> =
                Box::new(coord_system.y_data_to_horizontal_pixels(draw_x0, plot_w));
            (x_mapper, y_mapper)
        } else {
            // Normal orientation: x maps to horizontal pixels and y maps to vertical pixels
            let x_mapper: Box<dyn Fn(f64) -> f64> =
                Box::new(coord_system.x_data_to_horizontal_pixels(draw_x0, plot_w));
            let y_mapper: Box<dyn Fn(f64) -> f64> =
                Box::new(coord_system.y_data_to_vertical_pixels(draw_y0, plot_h));
            (x_mapper, y_mapper)
        };

        // Create context for rendering with proper mappers
        let context = SharedRenderingContext {
            x_mapper: &*x_mapper,
            y_mapper: &*y_mapper,
            coord_system: &coord_system,
            draw_x0,
            draw_y0,
            plot_width: plot_w,
            plot_height: plot_h,
            swapped_axes: self.swapped_axes,
            legend: self.legend,
        };

        // Render each layer's marks
        for layer in &self.layers {
            layer.render_marks(svg, &context)?;
        }

        // Render axes only if:
        // 1. axes setting is explicitly true, OR
        // 2. axes setting is not explicitly false AND at least one layer requires axes
        let should_render_axes = match self.axes {
            Some(true) => true,
            Some(false) => false,
            None => self.layers.iter().any(|layer| layer.requires_axes()),
        };
        if should_render_axes {
            render_axes(svg, &self.theme, &context)?;
        }

        // Render each layer's legends if legend is enabled
        for layer in &self.layers {
            layer.render_legends(svg, &self.theme, &context)?;
        }

        Ok(())
    }

    // Generate the SVG content for the chart
    fn generate_svg(&self) -> Result<String, ChartonError> {
        let mut svg_content = String::new();
        // Add SVG header with viewBox for better scaling
        svg_content.push_str(&format!(
            r#"<svg width="{}" height="{}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">"#,
            self.width, self.height, self.width, self.height
        ));

        // Add background rectangle if background is specified
        if let Some(ref bg_color) = self.background {
            svg_content.push_str(&format!(
                r#"<rect width="100%" height="100%" fill="{}" />"#,
                bg_color
            ));
        }

        // Render the chart content
        self.render(&mut svg_content)?;

        // Close SVG tag
        svg_content.push_str("</svg>");

        Ok(svg_content)
    }

    /// Generates and returns the SVG representation of the chart.
    ///
    /// This method renders the entire chart as an SVG (Scalable Vector Graphics) string,
    /// including all layers, axes, labels, legends, and other visual elements. The
    /// generated SVG can be embedded directly in HTML documents.
    ///
    /// # Returns
    /// A Result containing either:
    /// - Ok(String) with the complete SVG markup of the chart
    /// - Err(ChartonError) if there was an error during rendering
    ///
    /// # Example
    /// ```
    /// let svg_string = chart.to_svg()?;
    /// std::fs::write("chart.svg", svg_string)?;
    /// ```
    pub fn to_svg(&self) -> Result<String, ChartonError> {
        self.generate_svg()
    }

    /// Generate the chart and display in Jupyter
    ///
    /// Renders the chart as an SVG and displays it directly in a Jupyter notebook
    /// environment using the EVCXR kernel. This method is specifically designed
    /// for interactive data exploration in Jupyter notebooks.
    ///
    /// The method automatically detects if it's running in an EVCXR environment
    /// and will only display the chart in that context. In other environments,
    /// this method will successfully execute but won't produce any visible output.
    ///
    /// # Returns
    ///
    /// Returns a Result indicating success or a ChartonError if SVG generation fails
    ///
    /// # Example
    ///
    /// ```rust
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
    /// let chart = Chart::build(&df)?
    ///     .mark_point()
    ///     .encode(X::new("x"), Y::new("y"))?;
    ///
    /// chart.show()?; // Displays in Jupyter notebook
    /// ```
    pub fn show(&self) -> Result<(), ChartonError> {
        let svg_content = self.generate_svg()?;

        // Check if we're in EVCXR Jupyter environment
        if std::env::var("EVCXR_IS_RUNTIME").is_ok() {
            println!(
                "EVCXR_BEGIN_CONTENT text/html\n{}\nEVCXR_END_CONTENT",
                svg_content
            );
        }

        Ok(())
    }

    /// Generate the chart and save to file
    ///
    /// Renders the chart and saves it to the specified file path. The format is determined
    /// by the file extension in the path. Currently, only SVG and PNG format are supported.
    ///
    /// # Arguments
    ///
    /// * `path` - A path-like object specifying where to save the chart file
    ///
    /// # Returns
    ///
    /// Returns a Result indicating success or a ChartonError if SVG/PNG generation or file writing fails
    ///
    /// # Example
    ///
    /// ```rust
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
    /// let chart = Chart::build(&df)?
    ///     .mark_point()
    ///     .encode(x("x"), y("y"))?;
    ///
    /// chart.save("my_chart.svg")?; // Save as SVG file
    /// ```
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ChartonError> {
        let svg_content = self.generate_svg()?;

        // Convert to Path for file operations
        let path_obj = path.as_ref();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path_obj.parent().filter(|p| !p.exists()) {
            std::fs::create_dir_all(parent).map_err(|e| {
                ChartonError::Io(std::io::Error::other(format!(
                    "Failed to create directory: {}",
                    e
                )))
            })?;
        }

        let ext = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        match ext.as_deref() {
            Some("svg") => {
                std::fs::write(path_obj, svg_content).map_err(ChartonError::Io)?;
            }
            Some("png") => {
                // Load system fonts
                let mut opts = resvg::usvg::Options::default();

                // 1. Create a new fontdb instead of cloning the default one
                let mut fontdb = resvg::usvg::fontdb::Database::new();

                // 2. Load system fonts (utilizing resources from various OS)
                fontdb.load_system_fonts();

                // 3. Load built-in "emergency" font to ensure display even in extreme environments
                let default_font_data = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
                fontdb.load_font_data(default_font_data.to_vec());

                // 4. Set explicit family mappings (Fallback logic)
                // When users specify "sans-serif" but the system doesn't have mappings configured,
                // resvg will try this font as a fallback.
                fontdb.set_sans_serif_family("Inter");

                opts.fontdb = std::sync::Arc::new(fontdb);

                // Parse svg string
                let tree = resvg::usvg::Tree::from_str(&svg_content, &opts)
                    .map_err(|e| ChartonError::Render(format!("SVG parsing error: {:?}", e)))?;

                // Scale the image size to higher resolution
                let pixmap_size = tree.size();
                let scale = 2.0;
                let width = (pixmap_size.width() * scale) as u32;
                let height = (pixmap_size.height() * scale) as u32;

                // Create pixmap
                let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
                    .ok_or(ChartonError::Render("Failed to create pixmap".into()))?;

                // Render and save
                let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
                resvg::render(&tree, transform, &mut pixmap.as_mut());
                pixmap
                    .save_png(path_obj)
                    .map_err(|e| ChartonError::Render(format!("PNG saving error: {:?}", e)))?;
            }
            Some(format) => {
                return Err(ChartonError::Unimplemented(format!(
                    "Output format '{}' is not supported",
                    format
                )));
            }
            None => {
                return Err(ChartonError::Unimplemented(
                    "Output format could not be determined from file extension".to_string(),
                ));
            }
        }

        Ok(())
    }
}

// Convert single layer chart into layered chart
impl<T: Mark + 'static> From<Chart<T>> for LayeredChart
where
    Chart<T>: Layer,
{
    fn from(val: Chart<T>) -> Self {
        LayeredChart::new().add_layer(val)
    }
}

impl<T: Mark + 'static> Chart<T>
where
    Chart<T>: Layer,
{
    /// Convert a single-layer chart into a layered chart
    ///
    /// This method transforms a single Chart<T> instance into a LayeredChart by adding
    /// the chart as the first (and only) layer of the new LayeredChart. This allows
    /// for easy conversion when you want to start with a simple chart and later add
    /// additional layers to create a more complex visualization.
    ///
    /// This is equivalent to calling `LayeredChart::new().add_layer(self)`, but provides
    /// a more convenient and readable syntax for the conversion.
    ///
    /// # Returns
    ///
    /// Returns a new LayeredChart instance with this chart as its first layer
    ///
    /// # Example
    ///
    /// ```rust
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
    /// let chart = Chart::build(&df)?
    ///     .mark_point()
    ///     .encode(x("x"), y("y"))?;
    ///
    /// // Convert to layered chart for further composition
    /// let layered_chart = chart.into_layered();
    /// ```
    pub fn into_layered(self) -> LayeredChart {
        self.into()
    }
}