use crate::coordinate::{CoordinateTrait, CoordSystem, Rect, cartesian::Cartesian2D};
use crate::chart::Chart;
use crate::core::layer::Layer;
use crate::core::legend::{LegendSpec, LegendPosition};
use crate::core::context::SharedRenderingContext;
use crate::scale::{Scale, ScaleDomain, create_scale, mapper::VisualMapper};
use crate::encode::aesthetics::GlobalAesthetics;
use crate::theme::Theme;
use crate::error::ChartonError;

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
/// * `coord_system` - The type of coordinate system to use for this chart
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
    coord_system: CoordSystem,

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
    /// The strategic placement of the legend block.
    /// This property triggers dynamic margin adjustments during the layout phase.
    pub(crate) legend_position: LegendPosition,

    /// The spacing (in pixels) between the plot area and the legend.
    /// Acts as a buffer to prevent visual crowding.
    pub(crate) legend_margin: f64,

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
            coord_system: CoordSystem::default(),

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
            legend_position: LegendPosition::Right,
            legend_margin: 15.0,

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
    pub fn with_title_font_size(mut self, size: f64) -> Self {
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
    pub fn with_tick_label_font_size(mut self, size: f64) -> Self {
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
    pub fn with_label_font_size(mut self, size: f64) -> Self {
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
    pub fn coord_flip(mut self) -> Self {
        self.flipped = true;
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
    pub fn with_legend_font_size(mut self, size: f64) -> Self {
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

    /// Set the position of the chart legend.
    ///
    /// Configures where the legend is placed relative to the plot area. 
    /// Changing the position automatically adjusts the internal margins:
    /// - `Left` / `Right`: Legends are stacked vertically, reducing the plot width.
    /// - `Top` / `Bottom`: Legends are arranged horizontally (flow layout), reducing the plot height.
    /// - `None`: Disables the legend rendering entirely.
    ///
    /// # Arguments
    ///
    /// * `position` - The `LegendPosition` variant (Top, Bottom, Left, Right, or None).
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining.
    ///
    /// # Example
    ///
    /// ```
    /// let chart = LayeredChart::new()
    ///     .with_legend_position(LegendPosition::Bottom);
    /// ```
    pub fn with_legend_position(mut self, position: LegendPosition) -> Self {
        self.legend_position = position;
        self
    }

    /// Set the spacing between the plot area and the legend.
    ///
    /// This defines the buffer zone (in pixels) that prevents the legend from 
    /// overlapping with axis titles or labels.
    ///
    /// # Arguments
    ///
    /// * `margin_px` - The buffer space in pixels.
    ///
    /// # Returns
    ///
    /// Returns the chart instance for method chaining.
    pub fn with_legend_margin(mut self, margin_px: f64) -> Self {
        self.legend_margin = margin_px;
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

    /// Consolidates X-axis data domains across all layers.
    /// 
    /// Ensures all layers use a compatible Scale type and merges their data 
    /// into a single global domain (Continuous or Categorical).
    fn get_x_domain_from_layers(&self) -> Result<Option<(Scale, ScaleDomain)>, ChartonError> {
        let mut resolved_type: Option<Scale> = None;
        
        // Variables to track continuous bounds
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;
        
        // Vector to track unique categorical labels
        let mut all_labels: Vec<String> = Vec::new();

        for (i, layer) in self.layers.iter().enumerate() {
            // Step 1: Identify scale type for X in this layer
            let current_type = match layer.get_x_scale_type_from_layer() {
                Some(t) => t,
                None => continue, // Skip layers without X encoding
            };

            // Step 2: Validate type consistency
            if let Some(ref existing_type) = resolved_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "X-axis scale type conflict at layer {}: Expected {:?}, found {:?}",
                        i, existing_type, current_type
                    )));
                }
            } else {
                resolved_type = Some(current_type);
            }

            // Step 3: Extract and merge domain data
            match resolved_type.as_ref().unwrap() {
                Scale::Discrete => {
                    if let Some(labels) = layer.get_x_discrete_tick_labels()? {
                        for label in labels {
                            if !all_labels.contains(&label) {
                                all_labels.push(label);
                            }
                        }
                    }
                }
                _ => {
                    let (min, max) = layer.get_x_continuous_bounds()?;
                    global_min = global_min.min(min);
                    global_max = global_max.max(max);
                }
            }
        }

        // Step 4: Finalize the domain
        match resolved_type {
            Some(stype) => {
                let domain = match stype {
                    Scale::Discrete => {
                        if all_labels.is_empty() { return Ok(None); }
                        ScaleDomain::Categorical(all_labels)
                    },
                    _ => {
                        if global_min.is_infinite() {
                            ScaleDomain::Continuous(0.0, 1.0)
                        } else {
                            // Handle edge case where min == max (e.g., single data point)
                            if (global_max - global_min).abs() < 1e-12 {
                                global_min -= 0.5;
                                global_max += 0.5;
                            }
                            ScaleDomain::Continuous(global_min, global_max)
                        }
                    }
                };
                Ok(Some((stype, domain)))
            },
            None => Ok(None),
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

    // Get the x-axis scale from all layers
    fn get_x_scale_type_from_layers(&self) -> Option<Scale> {
        if self.layers.is_empty() {
            return None;
        }

        // Iterate through all layers to find the first non-None scale
        for layer in &self.layers {
            // Use if let in case charts that don't have x encoding (like pie charts)
            if let Some(scale) = layer.get_x_scale_type_from_layer() {
                return Some(scale);
            }
        }

        None
    }

    /// Consolidates Y-axis data domains across all layers.
    /// 
    /// Follows the same consolidation logic as X and Color channels:
    /// 1. Validates scale type consistency (Linear vs Discrete).
    /// 2. Aggregates min/max for continuous scales or unique labels for discrete ones.
    fn get_y_domain_from_layers(&self) -> Result<Option<(Scale, ScaleDomain)>, ChartonError> {
        let mut resolved_type: Option<Scale> = None;
        
        // Variables to track continuous bounds
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;
        
        // Vector to track unique categorical labels
        let mut all_labels: Vec<String> = Vec::new();

        for (i, layer) in self.layers.iter().enumerate() {
            // Step 1: Check Y encoding and scale type
            let current_type = match layer.get_y_scale_type_from_layer() {
                Some(t) => t,
                None => continue, // Skip layers without Y encoding
            };

            // Step 2: Ensure Y scale consistency across layers
            if let Some(ref existing_type) = resolved_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "Y-axis scale type conflict at layer {}: Expected {:?}, found {:?}",
                        i, existing_type, current_type
                    )));
                }
            } else {
                resolved_type = Some(current_type);
            }

            // Step 3: Domain aggregation
            match resolved_type.as_ref().unwrap() {
                Scale::Discrete => {
                    if let Some(labels) = layer.get_y_discrete_tick_labels()? {
                        for label in labels {
                            if !all_labels.contains(&label) {
                                all_labels.push(label);
                            }
                        }
                    }
                }
                _ => {
                    let (min, max) = layer.get_y_continuous_bounds()?;
                    global_min = global_min.min(min);
                    global_max = global_max.max(max);
                }
            }
        }

        // Step 4: Construct final Y domain
        match resolved_type {
            Some(stype) => {
                let domain = match stype {
                    Scale::Discrete => {
                        if all_labels.is_empty() { return Ok(None); }
                        ScaleDomain::Categorical(all_labels)
                    },
                    _ => {
                        if global_min.is_infinite() {
                            ScaleDomain::Continuous(0.0, 1.0)
                        } else {
                            // Apply offset if min/max are identical to ensure a valid range
                            if (global_max - global_min).abs() < 1e-12 {
                                global_min -= 0.5;
                                global_max += 0.5;
                            }
                            ScaleDomain::Continuous(global_min, global_max)
                        }
                    }
                };
                Ok(Some((stype, domain)))
            },
            None => Ok(None),
        }
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

    // Get the y-axis scale from all layers
    fn get_y_scale_type_from_layers(&self) -> Option<Scale> {
        if self.layers.is_empty() {
            return None;
        }

        // Iterate through all layers to find the first non-None scale
        for layer in &self.layers {
            // Use if let in case charts that don't have y encoding (like pie charts)
            if let Some(scale) = layer.get_y_scale_type_from_layer() {
                return Some(scale);
            }
        }

        None
    }

    /// Consolidates color data domains across all layers to ensure visual consistency.
    /// 
    /// This method performs two critical tasks:
    /// 1. **Type Validation**: It ensures that all layers using the color channel share the 
    ///    same Scale type (e.g., you cannot mix a Continuous 'Linear' scale with a 
    ///    Discrete 'Ordinal' scale in the same chart).
    /// 2. **Domain Aggregation**: 
    ///    - For Continuous scales: It finds the global minimum and maximum across all layers.
    ///    - For Categorical scales: It collects all unique labels while preserving 
    ///      insertion order across layers.
    ///
    /// # Returns
    /// - `Ok(Some((Scale, ScaleDomain)))`: A unified Scale type and domain ready for Scale initialization.
    /// - `Ok(None)`: If no layers have color encodings defined.
    /// - `Err(ChartonError)`: If a type conflict is detected between layers.
    fn get_color_domain_from_layers(&self) -> Result<Option<(Scale, ScaleDomain)>, ChartonError> {
        let mut resolved_type: Option<Scale> = None;
        
        // Variables to track continuous bounds
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;
        
        // Vector to track unique categorical labels
        let mut all_labels: Vec<String> = Vec::new();

        for (i, layer) in self.layers.iter().enumerate() {
            // Step 1: Check if this layer has a color encoding and what its scale type is
            let current_type = match layer.get_color_scale_type_from_layer() {
                Some(t) => t,
                None => continue, // Skip layers that don't encode color
            };

            // Step 2: Ensure type consistency across the entire layered chart
            if let Some(ref existing_type) = resolved_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "Color scale type conflict at layer {}: Layer 0 is {:?}, but layer {} is {:?}",
                        i, existing_type, i, current_type
                    )));
                }
            } else {
                // This is the first layer with color; it sets the 'source of truth' for the chart
                resolved_type = Some(current_type);
            }

            // Step 3: Extract and merge domain data based on the resolved type
            match resolved_type.as_ref().unwrap() {
                Scale::Discrete => {
                    // Collect unique strings for categorical mapping
                    if let Some(labels) = layer.get_color_discrete_labels()? {
                        for label in labels {
                            if !all_labels.contains(&label) {
                                all_labels.push(label);
                            }
                        }
                    }
                }
                _ => {
                    // Update global min/max for continuous mapping (Linear, Log, etc.)
                    if let Some((min, max)) = layer.get_color_continuous_bounds()? {
                        global_min = global_min.min(min);
                        global_max = global_max.max(max);
                    }
                }
            }
        }

        // Step 4: Construct the final ScaleDomain based on the accumulated data
        match resolved_type {
            Some(stype) => {
                let domain = match stype {
                    Scale::Discrete => {
                        if all_labels.is_empty() { return Ok(None); }
                        ScaleDomain::Categorical(all_labels)
                    },
                    _ => {
                        // Handle cases where no valid numeric data was found despite having a scale type
                        if global_min.is_infinite() {
                            // Fallback to a unit range [0, 1] if data is missing or empty
                            ScaleDomain::Continuous(0.0, 1.0)
                        } else {
                            // Or apply optional user-defined overrides if they exist at the chart level
                            // (Assuming self.color_domain_min/max exist similar to x_domain_min)
                            ScaleDomain::Continuous(global_min, global_max)
                        }
                    }
                };
                Ok(Some((stype, domain)))
            },
            None => Ok(None),
        }
    }

    /// Consolidates shape data domains across all layers.
    ///
    /// # Returns
    /// - `Ok(Some(ScaleDomain::Categorical))`: Unified unique shape labels.
    /// - `Ok(None)`: If no shape encodings are defined.
    fn get_shape_domain_from_layers(&self) -> Result<Option<ScaleDomain>, ChartonError> {
        let mut resolved_type: Option<Scale> = None;
        let mut all_labels: Vec<String> = Vec::new();

        for (i, layer) in self.layers.iter().enumerate() {
            let current_type = match layer.get_shape_scale_type_from_layer() {
                Some(t) => t,
                None => continue,
            };

            // Type Validation (Ensuring Shape remains Discrete)
            if let Some(ref existing_type) = resolved_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "Shape scale type conflict at layer {}: expected {:?}, found {:?}",
                        i, existing_type, current_type
                    )));
                }
            } else {
                resolved_type = Some(current_type);
            }

            // Domain Aggregation
            if let Some(labels) = layer.get_shape_discrete_labels()? {
                for label in labels {
                    if !all_labels.contains(&label) {
                        all_labels.push(label);
                    }
                }
            }
        }

        match resolved_type {
            Some(_) => {
                if all_labels.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(ScaleDomain::Categorical(all_labels)))
                }
            },
            None => Ok(None),
        }
    }

    /// Consolidates size data domains across all layers.
    ///
    /// # Returns
    /// - `Ok(Some(ScaleDomain::Continuous))`: Unified numeric range for size mapping.
    /// - `Ok(None)`: If no size encodings are defined.
    fn get_size_domain_from_layers(&self) -> Result<Option<ScaleDomain>, ChartonError> {
        let mut resolved_type: Option<Scale> = None;
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;

        for (i, layer) in self.layers.iter().enumerate() {
            let current_type = match layer.get_size_scale_type_from_layer() {
                Some(t) => t,
                None => continue,
            };

            if let Some(ref existing_type) = resolved_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "Size scale type conflict at layer {}: expected {:?}, found {:?}",
                        i, existing_type, current_type
                    )));
                }
            } else {
                resolved_type = Some(current_type);
            }

            if let Some((min, max)) = layer.get_size_continuous_bounds()? {
                global_min = global_min.min(min);
                global_max = global_max.max(max);
            }
        }

        match resolved_type {
            Some(_) => {
                if global_min.is_infinite() {
                    Ok(Some(ScaleDomain::Continuous(0.0, 1.0)))
                } else {
                    // Ensure the range is not zero
                    if (global_max - global_min).abs() < 1e-12 {
                        global_min -= 0.5;
                        global_max += 0.5;
                    }
                    Ok(Some(ScaleDomain::Continuous(global_min, global_max)))
                }
            },
            None => Ok(None),
        }
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

    /// Resolves the final rendering layout and global aesthetic scales by consolidating metadata.
    /// 
    /// This function performs the "Training Phase" of the chart:
    /// 1. **Geometry**: Calculates the plot area (Panel) by calling the LayoutEngine.
    /// 2. **Coordinates**: Resolves X and Y scales and constructs the Coordinate System.
    /// 3. **Aesthetics**: Aggregates Color, Size, and Shape domains to create global Mappers.
    ///
    /// # Arguments
    /// * `legend_specs` - The unified legend specifications used to calculate dynamic margins.
    fn resolve_rendering_layout(&self, legend_specs: &[LegendSpec]) -> Result<(Box<dyn CoordinateTrait>, Rect, GlobalAesthetics), ChartonError> {
        // --- 1. Geometry: Calculate the physical drawing panel ---
        // We delegate the complex estimation of legend sizes to the LayoutEngine.
        let legend_box = crate::core::layout::LayoutEngine::calculate_legend_constraints(
            legend_specs,
            self.legend_position,
            self.legend_margin,
            &self.theme
        );
        
        // Add legend constraints (pixels) to the user-defined proportional margins.
        // This ensures the plot panel "shrinks" to make room for titles and legends.
        let left_margin_px = (self.left_margin * self.width as f64) + legend_box.left;
        let right_margin_px = (self.right_margin * self.width as f64) + legend_box.right;
        let top_margin_px = (self.top_margin * self.height as f64) + legend_box.top;
        let bottom_margin_px = (self.bottom_margin * self.height as f64) + legend_box.bottom;
        
        // Calculate final plot dimensions
        let plot_w = (self.width as f64 - left_margin_px - right_margin_px).max(0.0);
        let plot_h = (self.height as f64 - top_margin_px - bottom_margin_px).max(0.0);

        // Define the target rectangle where all data marks will be rendered.
        let panel = Rect::new(left_margin_px, top_margin_px, plot_w, plot_h);

        // --- 2. Coordinate Scales: Resolve X and Y ---
        // The process involves retrieving raw domains from layers and applying user overrides.
        
        // Resolve X Scale
        let x_scale = if let Some((stype, mut domain)) = self.get_x_domain_from_layers()? {
            if let ScaleDomain::Continuous(ref mut min, ref mut max) = domain {
                if let Some(u_min) = self.x_domain_min { *min = u_min; }
                if let Some(u_max) = self.x_domain_max { *max = u_max; }
            }
            create_scale(&stype, domain, self.theme.x_expand)?
        } else {
            create_scale(&Scale::Linear, ScaleDomain::Continuous(0.0, 1.0), self.theme.x_expand)?
        };

        // Resolve Y Scale
        let y_scale = if let Some((stype, mut domain)) = self.get_y_domain_from_layers()? {
            if let ScaleDomain::Continuous(ref mut min, ref mut max) = domain {
                if let Some(u_min) = self.y_domain_min { *min = u_min; }
                if let Some(u_max) = self.y_domain_max { *max = u_max; }
            }
            create_scale(&stype, domain, self.theme.y_expand)?
        } else {
            create_scale(&Scale::Linear, ScaleDomain::Continuous(0.0, 1.0), self.theme.y_expand)?
        };

        // --- 3. Aesthetic Scales: Color, Size, and Shape ---
        // These scales ensure that shared visual properties (like color) are consistent across all layers.
        
        let color_bundle = if let Some((scale_type, domain)) = self.get_color_domain_from_layers()? {
            let scale = create_scale(&scale_type, domain, self.theme.color_expand)?;
            let mapper = VisualMapper::new_color_default(&scale_type, &self.theme);
            Some((scale, mapper))
        } else {
            None
        };

        let shape_bundle = if let Some(domain) = self.get_shape_domain_from_layers()? {
            let scale = create_scale(&Scale::Discrete, domain, self.theme.shape_expand)?;
            let mapper = VisualMapper::new_shape_default();
            Some((scale, mapper))
        } else {
            None
        };

        let size_bundle = if let Some(domain) = self.get_size_domain_from_layers()? {
            let scale = create_scale(&Scale::Linear, domain, self.theme.size_expand)?;
            let mapper = VisualMapper::new_size_default(2.0, 20.0);
            Some((scale, mapper))
        } else {
            None
        };

        // --- 4. Coordinate System Factory ---
        // Construct the final coordinate engine that maps data values to the resolved Panel.
        let coord_box: Box<dyn CoordinateTrait> = match self.coord_system {
            CoordSystem::Cartesian2D => {
                Box::new(Cartesian2D::new(x_scale, y_scale, self.flipped))
            },
            CoordSystem::Polar => {
                todo!("Implement Polar coordinates using the resolved scales")
            }
        };

        let aesthetics = GlobalAesthetics {
            color: color_bundle,
            shape: shape_bundle,
            size: size_bundle,
        };

        Ok((coord_box, panel, aesthetics))
    }

    /// Renders the chart title at the top-center of the SVG canvas.
    /// 
    /// The title's appearance (font size, family, and color) is determined by the 
    /// current theme. If no title is provided in the chart configuration, 
    /// this method returns early without modifying the SVG string.
    fn render_title(&self, svg: &mut String) -> Result<(), ChartonError> {
        // If no title is defined, there is nothing to render.
        let title_text = match &self.title {
            Some(t) => t,
            None => return Ok(()),
        };

        // Calculate the horizontal center of the SVG canvas.
        let center_x = self.width as f64 / 2.0;
        
        // Position the title slightly below the top edge. 
        // We use an offset of roughly 25 pixels, but this could be 
        // linked to top_margin in a more complex layout.
        let top_offset = 25.0;

        // Extract styling metadata from the theme.
        let font_size = self.theme.title_font_size;
        let font_family = &self.theme.title_font_family;
        let font_color = &self.theme.title_color;

        // Generate the SVG <text> element.
        // text-anchor="middle" ensures the center_x is the midpoint of the string.
        let title_tag = format!(
            r#"<text x="{}" y="{}" text-anchor="middle" font-family="{}" font-size="{}" fill="{}">{}</text>"#,
            center_x,
            top_offset,
            font_family,
            font_size,
            font_color,
            title_text
        );

        // Append the title element to the SVG buffer.
        svg.push_str(&title_tag);
        svg.push('\n');

        Ok(())
    }

    /// Renders the entire layered chart to the provided SVG string.
    ///
    /// This implementation follows the Grammar of Graphics pipeline:
    /// 1. Sync: Consolidate data domains across all layers to ensure visual consistency.
    /// 2. Back-fill: Update layers with unified scale/domain metadata.
    /// 3. Layout: Calculate plot area (Panel) using dynamic margins and legend dimensions.
    /// 4. Draw: Render axes, marks, and unified legends using the resolved context.
    /// 
    /// This method takes ownership of self (mut self) and returns it back to 
    /// the caller, supporting a one-time fluent generation pipeline.
    pub fn render(mut self, svg: &mut String) -> Result<Self, ChartonError> {
        // 0. Guard: If no layers exist, we render nothing.
        if self.layers.is_empty() { 
            return Ok(self); 
        }

        // --- STEP 1: SYNC & BACK-FILL PHASE ---
        // Calculate global domains from all layers to ensure visual consistency 
        // across the entire chart (e.g., "Red" always means the same category).
        let global_color = self.get_color_domain_from_layers()?;
        let global_shape = self.get_shape_domain_from_layers()?;
        let global_size = self.get_size_domain_from_layers()?;

        // Back-fill: Update each layer with the unified global metadata.
        // This synchronization is a core requirement of the Grammar of Graphics.
        for layer in self.layers.iter_mut() {
            if let Some((scale, domain)) = &global_color {
                layer.set_scale_type("color", scale.clone());
                layer.set_domain("color", domain.clone());
            }
            if let Some(domain) = &global_shape {
                layer.set_domain("shape", domain.clone());
            }
            if let Some(domain) = &global_size {
                layer.set_domain("size", domain.clone());
            }
        }

        // --- STEP 2: LEGEND COLLECTION ---
        // Collect unified legend specifications after the back-fill phase.
        // The LegendManager aggregates requirements from all layers into unique guides.
        let legend_specs = crate::core::legend::LegendManager::collect_legends(&self.layers);

        // --- STEP 3: LAYOUT RESOLUTION ---
        // Resolve the coordinate system and plot area (Panel).
        // This step accounts for chart margins and the space required by the legend position.
        let (coord_box, panel, aesthetics) = self.resolve_rendering_layout(&legend_specs)?; 

        // Construct the SharedRenderingContext.
        // Note: 'panel' here is the "squeezed" area reserved strictly for data marks,
        // while legend_position and legend_margin inform the renderers about the environment.
        let context = SharedRenderingContext {
            coord: &*coord_box, 
            panel,
            legend_position: self.legend_position,
            legend_margin: self.legend_margin,
            aesthetics,
        };

        // --- STEP 4: DRAWING PHASE ---
        
        // 5. Render Chart Title (if defined in the chart metadata)
        self.render_title(svg)?;

        // 6. Render Axes (X and Y) 
        // We determine if axes are needed by checking chart-level overrides or layer requirements.
        let should_render_axes = self.axes.unwrap_or_else(|| {
            self.layers.iter().any(|layer| layer.requires_axes())
        });
        if should_render_axes {
            crate::render::axis_renderer::render_axes(svg, &self.theme, &context)?;
        }

        // 7. Render Marks (Data Geometries)
        // Each layer draws its specific marks (points, lines, etc.) within the context's panel.
        let mut backend = crate::render::backend::svg::SvgBackend::new(svg);
        for layer in &self.layers {
            layer.render_marks(&mut backend, &context)?;
        }

        // 8. Render Unified Legends
        // The LegendRenderer uses the context to position itself relative to the panel.
        // We map the internal error to ChartonError to maintain the public API contract.
        crate::render::legend_renderer::LegendRenderer::render_legend(
            svg, 
            &legend_specs, 
            &self.theme, 
            &context
        ).map_err(|e| ChartonError::RenderError(e.to_string()))?;

        // Return the chart instance to allow for one-time fluent generation chains.
        Ok(self)
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
