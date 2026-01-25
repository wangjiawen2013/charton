use crate::prelude::SingleColor;
use crate::visual::color::{ColorMap, ColorPalette};

/// A `Theme` defines the visual "look and feel" of a chart, independent of the data mapping.
/// 
/// Following best practices from ggplot2 and Altair, this struct manages aesthetic properties 
/// like colors, fonts, and strokes. It delegates data-logic concerns (such as Scale Domains, 
/// Expansions, and Coordinate transformations) to the `LayeredChart` or `Scale` resolution logic.
#[derive(Clone)]
pub struct Theme {
    // --- Global Canvas Properties ---
    /// The fill color of the entire chart background.
    pub(crate) background_color: SingleColor,
    /// Whether to render the axis lines, ticks, and labels.
    pub(crate) show_axes: bool,

    // --- Title Properties ---
    /// Font size for the main chart title.
    pub(crate) title_size: f32,
    /// Font family stack for the main chart title.
    pub(crate) title_family: String,
    /// Text color for the main chart title.
    pub(crate) title_color: SingleColor,

    // --- Axis Title (Label) Properties ---
    /// Font size for axis titles (e.g., "Price", "Distance").
    pub(crate) label_size: f32,
    /// Font family for axis titles.
    pub(crate) label_family: String,
    /// Text color for axis titles.
    pub(crate) label_color: SingleColor,
    /// Additional spacing between the axis title and the tick labels.
    pub(crate) label_padding: f32,

    // --- Tick Label Properties (The numbers on the axes) ---
    /// Font size for the text next to axis ticks.
    pub(crate) tick_label_size: f32,
    /// Font family for the text next to axis ticks.
    pub(crate) tick_label_family: String,
    /// Text color for the text next to axis ticks.
    pub(crate) tick_label_color: SingleColor,
    /// Distance between the tick mark and the tick text.
    pub(crate) tick_label_padding: f32,
    /// Rotation angle in degrees for X-axis tick labels.
    pub(crate) x_tick_label_angle: f32,
    /// Rotation angle in degrees for Y-axis tick labels.
    pub(crate) y_tick_label_angle: f32,

    // --- Geometry & Stroke Properties ---
    /// Width of the main axis lines.
    pub(crate) axis_width: f32,
    /// Width of the small tick marks.
    pub(crate) tick_width: f32,
    /// The physical length of the tick marks extending from the axis.
    pub(crate) tick_length: f32,

    // --- Default Mark (Geometry) Properties ---
    /// The fallback size for geometries (e.g., point radius) when 'size' is not mapped to data.
    pub(crate) default_mark_size: f32,
    /// The fallback color for geometries when 'color' is not mapped to data.
    pub(crate) default_mark_color: SingleColor,

    // --- Legend Styling ---
    /// Font size for the title of the legend.
    pub(crate) legend_title_size: f32,
    /// Font size for legend item labels.
    pub(crate) legend_label_size: f32,
    /// Font family for all legend text.
    pub(crate) legend_label_family: String,
    /// Text color for all legend text.
    pub(crate) legend_label_color: SingleColor,
    /// Gap between separate legend blocks (e.g., between Color legend and Size legend).
    pub(crate) legend_block_gap: f32,
    /// Vertical gap between individual items within a legend.
    pub(crate) legend_item_v_gap: f32,
    /// Horizontal gap between columns in a multi-column legend.
    pub(crate) legend_col_h_gap: f32,
    /// Spacing between the legend title and the first legend item.
    pub(crate) legend_title_gap: f32,
    /// Spacing between the legend marker (symbol) and its text label.
    pub(crate) legend_marker_text_gap: f32,

    // --- Layout Defense Thresholds ---
    /// The minimum allowed size for the data panel before rendering fails or truncates.
    pub(crate) min_panel_size: f32,
    /// Maximum percentage of the total canvas that axes/margins can consume.
    pub(crate) panel_defense_ratio: f32,
    /// Pre-allocated pixel buffer for axis labels to prevent overlapping.
    pub(crate) axis_reserve_buffer: f32,

    // --- Aesthetic Defaults (Candidates for Scale Resolution) ---
    /// The default color map for continuous data if no specific scale is provided.
    pub(crate) default_color_map: ColorMap,
    /// The default categorical palette for discrete data if no specific scale is provided.
    pub(crate) default_palette: ColorPalette,
}

impl Theme {
    // --- Global Configuration ---

    pub fn with_background_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.background_color = color.into();
        self
    }

    pub fn with_show_axes(mut self, show: bool) -> Self {
        self.show_axes = show;
        self
    }

    // --- Title ---

    pub fn with_title_size(mut self, size: f32) -> Self {
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

    pub fn with_label_size(mut self, size: f32) -> Self {
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

    pub fn with_label_padding(mut self, padding: f32) -> Self {
        self.label_padding = padding;
        self
    }

    // --- Tick Label ---

    pub fn with_tick_label_size(mut self, size: f32) -> Self {
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

    pub fn with_tick_label_padding(mut self, padding: f32) -> Self {
        self.tick_label_padding = padding;
        self
    }

    pub fn with_x_tick_label_angle(mut self, angle: f32) -> Self {
        self.x_tick_label_angle = angle;
        self
    }

    pub fn with_y_tick_label_angle(mut self, angle: f32) -> Self {
        self.y_tick_label_angle = angle;
        self
    }

    // --- Geometry Strokes ---

    pub fn with_axis_width(mut self, width: f32) -> Self {
        self.axis_width = width;
        self
    }

    pub fn with_tick_width(mut self, width: f32) -> Self {
        self.tick_width = width;
        self
    }
    
    pub fn with_tick_length(mut self, length: f32) -> Self {
        self.tick_length = length;
        self
    }

    // --- Legend Styling ---

    pub fn with_legend_title_size(mut self, size: f32) -> Self {
        self.legend_title_size = size;
        self
    }

    pub fn with_legend_label_size(mut self, size: f32) -> Self {
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

    pub fn with_legend_block_gap(mut self, gap: f32) -> Self {
        self.legend_block_gap = gap;
        self
    }

    pub fn with_legend_item_v_gap(mut self, gap: f32) -> Self {
        self.legend_item_v_gap = gap;
        self
    }

    pub fn with_legend_col_h_gap(mut self, gap: f32) -> Self {
        self.legend_col_h_gap = gap;
        self
    }

    pub fn with_legend_title_gap(mut self, gap: f32) -> Self {
        self.legend_title_gap = gap;
        self
    }

    pub fn with_legend_marker_text_gap(mut self, gap: f32) -> Self {
        self.legend_marker_text_gap = gap;
        self
    }

    // --- Layout Defense ---

    pub fn with_min_panel_size(mut self, size: f32) -> Self {
        self.min_panel_size = size;
        self
    }

    pub fn with_panel_defense_ratio(mut self, ratio: f32) -> Self {
        self.panel_defense_ratio = ratio;
        self
    }

    pub fn with_axis_reserve_buffer(mut self, buffer: f32) -> Self {
        self.axis_reserve_buffer = buffer;
        self
    }

    // --- Color & Palette Defaults ---

    pub fn with_default_color_map(mut self, map: ColorMap) -> Self {
        self.default_color_map = map;
        self
    }

    pub fn with_default_palette(mut self, palette: ColorPalette) -> Self {
        self.default_palette = palette;
        self
    }
}

impl Default for Theme {
    fn default() -> Self {
        let font_stack = "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, 'PingFang SC', 'Microsoft YaHei', Ubuntu, Cantarell, 'Noto Sans', sans-serif".to_string();

        Self {
            background_color: "white".into(),
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

            default_mark_size: 4.0,
            default_mark_color: "#4682b4".into(), // SteelBlue

            legend_title_size: 14.0,
            legend_label_size: 12.0,
            legend_label_family: font_stack,
            legend_label_color: "#333".into(),
            legend_block_gap: 35.0,
            legend_item_v_gap: 3.0,
            legend_col_h_gap: 15.0,
            legend_title_gap: 7.0,
            legend_marker_text_gap: 8.0,

            min_panel_size: 100.0,
            panel_defense_ratio: 0.2,
            axis_reserve_buffer: 60.0,

            default_color_map: ColorMap::Viridis,
            default_palette: ColorPalette::Tab10,
        }
    }
}