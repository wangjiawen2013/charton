use crate::prelude::SingleColor;
use crate::visual::color::{ColorMap, ColorPalette};
use crate::core::guide::LegendPosition;

/// A `Theme` defines the visual "look and feel" of a chart.
/// 
/// It stores constants for aesthetics (colors, fonts) and layout preferences (margins, spacing).
/// It does NOT store data-specific content like titles or domain limits.
#[derive(Clone)]
pub struct Theme {
    // --- Global Canvas & Layout ---
    /// The fill color of the entire chart background.
    pub(crate) background_color: SingleColor,
    /// Default relative margins for the chart (top, right, bottom, left).
    /// Expressed as a ratio [0.0, 1.0] of the canvas size.
    pub(crate) top_margin: f64,
    pub(crate) right_margin: f64,
    pub(crate) bottom_margin: f64,
    pub(crate) left_margin: f64,
    /// Whether to render axes by default.
    pub(crate) show_axes: bool,

    // --- Main Title Styling ---
    /// Font size for the main chart title.
    pub(crate) title_size: f64,
    /// Font family stack for the main chart title.
    pub(crate) title_family: String,
    /// Text color for the main chart title.
    pub(crate) title_color: SingleColor,

    // --- Axis Title (Label) Styling ---
    /// Font size for axis titles (e.g., "Price").
    pub(crate) label_size: f64,
    /// Font family for axis titles.
    pub(crate) label_family: String,
    /// Text color for axis titles.
    pub(crate) label_color: SingleColor,
    /// Spacing between the axis title and the tick labels.
    pub(crate) label_padding: f64,

    // --- Tick Label Styling (The numbers/categories on axes) ---
    /// Font size for the text next to axis ticks.
    pub(crate) tick_label_size: f64,
    /// Font family for tick labels.
    pub(crate) tick_label_family: String,
    /// Text color for tick labels.
    pub(crate) tick_label_color: SingleColor,
    /// Distance between the tick mark and the tick text.
    pub(crate) tick_label_padding: f64,
    /// Default rotation angle in degrees for X-axis tick labels.
    pub(crate) x_tick_label_angle: f64,
    /// Default rotation angle in degrees for Y-axis tick labels.
    pub(crate) y_tick_label_angle: f64,

    // --- Geometry & Stroke Properties ---
    /// Width of the main axis lines.
    pub(crate) axis_width: f64,
    /// Width of the small tick marks.
    pub(crate) tick_width: f64,
    /// The physical length of the tick marks.
    pub(crate) tick_length: f64,
    /// Minimum pixel spacing between ticks to ensure visual density.
    pub(crate) tick_min_spacing: f64,

    // --- Legend Styling ---
    /// Font size for the legend's title.
    pub(crate) legend_title_size: f64,
    /// Font size for legend item labels.
    pub(crate) legend_label_size: f64,
    /// Font family for all legend text.
    pub(crate) legend_label_family: String,
    /// Text color for all legend text.
    pub(crate) legend_label_color: SingleColor,
    /// Default position of the legend relative to the plot.
    pub(crate) legend_position: LegendPosition,
    /// Spacing between the plot area and the legend.
    pub(crate) legend_margin: f64,
    /// Gap between separate legend blocks (e.g., Color vs Size).
    pub(crate) legend_block_gap: f64,
    /// Vertical gap between items within a legend.
    pub(crate) legend_item_v_gap: f64,
    /// Horizontal gap between columns in a multi-column legend.
    pub(crate) legend_col_h_gap: f64,
    /// Spacing between the legend title and its items.
    pub(crate) legend_title_gap: f64,
    /// Spacing between the legend marker (e.g., circle) and its label text.
    pub(crate) legend_marker_text_gap: f64,

    // --- Layout Defense & Auto-Sizing ---
    /// The minimum size (pixels) the data panel must maintain.
    pub(crate) min_panel_size: f64,
    /// Max ratio of the canvas that axes and margins can occupy.
    pub(crate) panel_defense_ratio: f64,
    /// Reserved pixel buffer for axis labels to prevent cropping.
    pub(crate) axis_reserve_buffer: f64,

    // --- Aesthetic Defaults ---
    /// Default color map for continuous data mapping.
    pub(crate) color_map: ColorMap,
    /// Default categorical palette for discrete data mapping.
    pub(crate) palette: ColorPalette,

    // --- Facet (Subplot) Styling ---
    /// Font size for facet strip labels.
    pub(crate) facet_label_size: f64,
    /// Color for facet label text.
    pub(crate) facet_label_color: SingleColor,
    /// Background fill for the facet strip header.
    pub(crate) facet_strip_fill: SingleColor,
    /// Spacing between individual facet panels.
    pub(crate) facet_spacing: f64,
    /// Padding inside the facet strip.
    pub(crate) facet_strip_padding: f64,

    // --- Polar Chart Defaults ---
    /// Default starting angle for polar charts (e.g., 12 o'clock).
    pub(crate) polar_start_angle: f64,
    /// Default angular span (e.g., 360 degrees).
    pub(crate) polar_end_angle: f64,
    /// Default inner radius ratio (e.g., 0.5 for a donut chart).
    pub(crate) polar_inner_radius: f64,

    /// Color for the grid lines. 
    /// Typically a faint gray like #BDBDBD or a semi-transparent version of label_color.
    pub(crate) grid_color: SingleColor,
    /// Width of the grid lines (usually thinner than axis_width).
    pub(crate) grid_width: f64,
}

impl Theme {
    // --- Global Configuration ---

    pub fn with_background_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.background_color = color.into();
        self
    }

    pub fn with_top_margin(mut self, margin: f64) -> Self {
        self.top_margin = margin;
        self
    }

    pub fn with_right_margin(mut self, margin: f64) -> Self {
        self.right_margin = margin;
        self
    }

    pub fn with_bottom_margin(mut self, margin: f64) -> Self {
        self.bottom_margin = margin;
        self
    }

    pub fn with_left_margin(mut self, margin: f64) -> Self {
        self.top_margin = margin;
        self
    }

    pub fn with_show_axes(mut self, show: bool) -> Self {
        self.show_axes = show;
        self
    }

    // --- Title ---

    pub fn with_title_size(mut self, size: f64) -> Self {
        self.title_size = size;
        self
    }

    pub fn with_title_family(mut self, family: impl Into<String>) -> Self {
        self.title_family = family.into();
        self
    }

    pub fn with_title_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.title_color = color.into();
        self
    }

    // --- Axis Label ---

    pub fn with_label_size(mut self, size: f64) -> Self {
        self.label_size = size;
        self
    }

    pub fn with_label_family(mut self, family: impl Into<String>) -> Self {
        self.label_family = family.into();
        self
    }

    pub fn with_label_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.label_color = color.into();
        self
    }

    pub fn with_label_padding(mut self, padding: f64) -> Self {
        self.label_padding = padding;
        self
    }

    // --- Tick Label ---

    pub fn with_tick_label_size(mut self, size: f64) -> Self {
        self.tick_label_size = size;
        self
    }

    pub fn with_tick_label_family(mut self, family: impl Into<String>) -> Self {
        self.tick_label_family = family.into();
        self
    }

    pub fn with_tick_label_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.tick_label_color = color.into();
        self
    }

    pub fn with_tick_label_padding(mut self, padding: f64) -> Self {
        self.tick_label_padding = padding;
        self
    }

    pub fn with_x_tick_label_angle(mut self, angle: f64) -> Self {
        self.x_tick_label_angle = angle;
        self
    }

    pub fn with_y_tick_label_angle(mut self, angle: f64) -> Self {
        self.y_tick_label_angle = angle;
        self
    }

    // --- Geometry Strokes ---

    pub fn with_axis_width(mut self, width: f64) -> Self {
        self.axis_width = width;
        self
    }

    pub fn with_tick_width(mut self, width: f64) -> Self {
        self.tick_width = width;
        self
    }
    
    pub fn with_tick_length(mut self, length: f64) -> Self {
        self.tick_length = length;
        self
    }

    pub fn with_tick_min_spacing(mut self, spacing: f64) -> Self {
        self.tick_min_spacing = spacing;
        self
    }

    // --- Legend Styling ---

    pub fn with_legend_title_size(mut self, size: f64) -> Self {
        self.legend_title_size = size;
        self
    }

    pub fn with_legend_label_size(mut self, size: f64) -> Self {
        self.legend_label_size = size;
        self
    }

    pub fn with_legend_label_family(mut self, family: impl Into<String>) -> Self {
        self.legend_label_family = family.into();
        self
    }

    pub fn with_legend_label_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.legend_label_color = color.into();
        self
    }

    pub fn with_legend_block_gap(mut self, gap: f64) -> Self {
        self.legend_block_gap = gap;
        self
    }

    pub fn with_legend_item_v_gap(mut self, gap: f64) -> Self {
        self.legend_item_v_gap = gap;
        self
    }

    pub fn with_legend_col_h_gap(mut self, gap: f64) -> Self {
        self.legend_col_h_gap = gap;
        self
    }

    pub fn with_legend_title_gap(mut self, gap: f64) -> Self {
        self.legend_title_gap = gap;
        self
    }

    pub fn with_legend_marker_text_gap(mut self, gap: f64) -> Self {
        self.legend_marker_text_gap = gap;
        self
    }

    // --- Legend Logic ---

    pub fn with_legend_position(mut self, position: LegendPosition) -> Self {
        self.legend_position = position;
        self
    }

    pub fn with_legend_margin(mut self, margin: f64) -> Self {
        self.legend_margin = margin;
        self
    }

    // --- Layout Defense ---

    pub fn with_min_panel_size(mut self, size: f64) -> Self {
        self.min_panel_size = size;
        self
    }

    pub fn with_panel_defense_ratio(mut self, ratio: f64) -> Self {
        self.panel_defense_ratio = ratio;
        self
    }

    pub fn with_axis_reserve_buffer(mut self, buffer: f64) -> Self {
        self.axis_reserve_buffer = buffer;
        self
    }

    // --- Color & Palette Defaults ---

    pub fn with_color_map(mut self, map: ColorMap) -> Self {
        self.color_map = map;
        self
    }

    pub fn with_palette(mut self, palette: ColorPalette) -> Self {
        self.palette = palette;
        self
    }

    // --- Facet Styling ---
    /// The font size for the facet labels (the text in the strip).
    pub fn with_facet_label_size(mut self, size: f64) -> Self {
        self.facet_label_size = size;
        self
    }

    /// The color of the facet label text.
    pub fn with_facet_label_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.facet_label_color = color.into();
        self
    }

    /// The background color of the facet strip (the header box).
    pub fn with_facet_strip_fill(mut self, color: impl Into<SingleColor>) -> Self {
        self.facet_strip_fill = color.into();
        self
    }

    /// The spacing between individual facet panels (both horizontal and vertical).
    pub fn with_facet_spacing(mut self, spacing: f64) -> Self {
        self.facet_spacing = spacing;
        self
    }
    /// The padding inside the facet strip.
    pub fn with_facet_strip_padding(mut self, padding: f64) -> Self {
        self.facet_strip_padding = padding;
        self
    }

    /// Calculates a suggested number of ticks based on the available 
    /// physical space and the theme's density settings.
    pub fn suggest_tick_count(&self, available_pixels: f64) -> usize {
        // We ensure at least 2 ticks (start and end) are always present.
        ((available_pixels / self.tick_min_spacing).floor() as usize).max(2)
    }

    pub fn with_grid_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.grid_color = color.into();
        self
    }

    pub fn with_grid_width(mut self, width: f64) -> Self {
        self.grid_width = width;
        self
    }
}

impl Default for Theme {
    fn default() -> Self {
        let font_stack = "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, 'PingFang SC', 'Microsoft YaHei', Ubuntu, Cantarell, 'Noto Sans', sans-serif".to_string();

        Self {
            background_color: "white".into(),
            top_margin: 0.10,
            right_margin: 0.03,
            bottom_margin: 0.08,
            left_margin: 0.06,
            show_axes: true,

            title_size: 18.0,
            title_family: font_stack.clone(),
            title_color: "#333".into(),

            label_size: 15.0,
            label_family: font_stack.clone(),
            label_color: "#333".into(),
            label_padding: 0.0,

            tick_label_size: 13.0,
            tick_label_family: font_stack.clone(),
            tick_label_color: "#333".into(),
            tick_label_padding: 3.0,

            x_tick_label_angle: 0.0,
            y_tick_label_angle: 0.0,

            axis_width: 1.0,
            tick_width: 1.0,
            tick_length: 6.0,
            tick_min_spacing: 50.0,

            legend_title_size: 14.0,
            legend_label_size: 12.0,
            legend_label_family: font_stack,
            legend_label_color: "#333".into(),
            legend_block_gap: 35.0,
            legend_item_v_gap: 3.0,
            legend_col_h_gap: 15.0,
            legend_title_gap: 7.0,
            legend_marker_text_gap: 8.0,

            legend_position: LegendPosition::Right,
            legend_margin: 15.0,

            min_panel_size: 100.0,
            panel_defense_ratio: 0.2,
            axis_reserve_buffer: 60.0,

            color_map: ColorMap::Viridis,
            palette: ColorPalette::Tab10,

            facet_label_size: 11.0,
            facet_label_color: "#333".into(),
            facet_strip_fill: "lightgray".into(),
            facet_spacing: 10.0,
            facet_strip_padding: 5.0,

            polar_start_angle: -std::f64::consts::FRAC_PI_2, 
            polar_end_angle: 3.0 * std::f64::consts::FRAC_PI_2, // start + 2*PI
            polar_inner_radius: 0.0,

            grid_color: " #BDBDBD".into(),
            grid_width: 1.0,
        }
    }
}