use crate::visual::color::{ColorMap, ColorPalette};
use crate::scale::Expansion;
/// A theme that defines the visual styling properties for plots.
///
/// The `Theme` struct contains all the styling parameters that control
/// the appearance of various plot elements including titles, axis labels,
/// tick marks, and legends. It provides a centralized way to manage
/// the visual design of plots.
///
/// # Fields
///
/// The theme is organized into several sections:
///
/// - **Title properties**: Control the appearance of plot titles
/// - **Axis label properties**: Define how axis labels are displayed
/// - **Tick label properties**: Configure the styling of tick mark labels
/// - **Stroke properties**: Set the width of axis and tick lines
/// - **Legend properties**: Control legend appearance
/// - **Padding properties**: Define spacing around various elements
///
/// # Examples
///
/// ```
/// use charton::theme::Theme;
///
/// let theme = Theme::default();
/// ```
#[derive(Clone)]
pub struct Theme {
    // Title properties
    pub(crate) title_font_size: f64,
    pub(crate) title_font_family: String,
    pub(crate) title_color: String,

    // Axis label properties
    pub(crate) label_font_size: f64,
    pub(crate) label_font_family: String,
    pub(crate) label_color: String,
    pub(crate) label_angle: f64,

    // Axis label specific padding
    pub(crate) x_label_padding: f64,
    pub(crate) y_label_padding: f64,

    // Tick label properties
    pub(crate) tick_label_font_size: f64,
    pub(crate) tick_label_font_family: String,
    pub(crate) tick_label_color: String,

    // Tick label specific rotation angles
    pub(crate) x_tick_label_angle: f64,
    pub(crate) y_tick_label_angle: f64,

    // Stroke properties
    pub(crate) axis_stroke_width: f64,
    pub(crate) tick_stroke_width: f64,

    // Legend properties
    pub(crate) legend_font_size: Option<f64>,
    pub(crate) legend_font_family: Option<String>,
    pub(crate) legent_label_color: String,

    pub(crate) color_map: ColorMap,
    pub(crate) color_palette: ColorPalette,

    // Expansion for x-axis or y-axis including both min and max expanding
    pub(crate) x_expand: Expansion,
    pub(crate) y_expand: Expansion,
    pub(crate) color_expand: Expansion,
    pub(crate) shape_expand: Expansion,
    pub(crate) size_expand: Expansion,

    pub(crate) tick_label_padding: f64,
}

impl Default for Theme {
    fn default() -> Self {
        let font_stack = "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, 'PingFang SC', 'Microsoft YaHei', Ubuntu, Cantarell, 'Noto Sans', sans-serif".to_string();

        Self {
            title_font_size: 18.0,
            title_font_family: font_stack.clone(),
            title_color: "#333".to_string(),

            label_font_size: 15.0,
            label_font_family: font_stack.clone(),
            label_color: "#333".to_string(),
            label_angle: 0.0,

            x_label_padding: 10.0,
            y_label_padding: 10.0,

            tick_label_font_size: 13.0,
            tick_label_font_family: font_stack,
            tick_label_color: "#333".to_string(),

            x_tick_label_angle: 0.0,
            y_tick_label_angle: 0.0,

            axis_stroke_width: 1.0,
            tick_stroke_width: 1.0,

            legend_font_size: None,
            legend_font_family: None,
            legent_label_color: "#333".to_string(),

            color_map: ColorMap::Viridis,
            color_palette: ColorPalette::Tab10,

            x_expand: Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) },
            y_expand: Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) },
            color_expand: Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) },
            shape_expand: Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) },
            size_expand: Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) },

            tick_label_padding: 3.0,
        }
    }
}
