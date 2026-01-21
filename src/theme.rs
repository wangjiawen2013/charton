use crate::prelude::SingleColor;
use crate::visual::color::{ColorMap, ColorPalette};
use crate::scale::Expansion;

/// A theme that defines the visual styling properties for plots.
/// 
/// This struct separates aesthetic concerns (colors, fonts, stroke widths) 
/// from the structural logic of the chart.
#[derive(Clone)]
pub struct Theme {
    // --- Global Canvas Properties ---
    pub(crate) background_color: SingleColor,
    pub(crate) show_axes: bool,

    // --- Title properties ---
    pub(crate) title_size: f32,
    pub(crate) title_family: String,
    pub(crate) title_color: SingleColor,

    // --- Axis label properties ---
    pub(crate) label_size: f32,
    pub(crate) label_family: String,
    pub(crate) label_color: SingleColor,
    pub(crate) label_padding: f32,

    // --- Tick label properties ---
    pub(crate) tick_label_size: f32,
    pub(crate) tick_label_family: String,
    pub(crate) tick_label_color: SingleColor,
    pub(crate) tick_label_padding: f32,
    pub(crate) x_tick_label_angle: f32,
    pub(crate) y_tick_label_angle: f32,

    // --- Geometry & Stroke properties ---
    pub(crate) axis_width: f32,
    pub(crate) tick_width: f32,
    pub(crate) tick_length: f32,

    // --- Legend styling ---
    pub(crate) legend_label_size: Option<f32>,
    pub(crate) legend_label_family: Option<String>,
    pub(crate) legend_label_color: SingleColor,
    pub(crate) legend_block_gap: f32,
    pub(crate) legend_item_v_gap: f32,
    pub(crate) legend_col_h_gap: f32,
    pub(crate) legend_title_gap: f32,
    pub(crate) legend_marker_text_gap: f32,

    // --- Layout Defense Thresholds ---
    pub(crate) min_panel_size: f32,
    pub(crate) panel_defense_ratio: f32,
    pub(crate) axis_reserve_buffer: f32,

    // --- Color & Scale Defaults ---
    pub(crate) color_map: ColorMap,
    pub(crate) palette: ColorPalette,
    pub(crate) x_expand: Expansion,
    pub(crate) y_expand: Expansion,
    pub(crate) color_expand: Expansion,
    pub(crate) shape_expand: Expansion,
    pub(crate) size_expand: Expansion,
}

impl Theme {
    // --- Global Configuration ---

    pub fn background_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.background_color = color.into();
        self
    }

    pub fn show_axes(mut self, show: bool) -> Self {
        self.show_axes = show;
        self
    }

    // --- Title ---

    pub fn title_size(mut self, size: f32) -> Self {
        self.title_size = size;
        self
    }

    pub fn title_family(mut self, family: impl Into<String>) -> Self {
        self.title_family = family.into();
        self
    }

    pub fn title_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.title_color = color.into();
        self
    }

    // --- Label ---

    pub fn label_size(mut self, size: f32) -> Self {
        self.label_size = size;
        self
    }

    pub fn label_family(mut self, family: impl Into<String>) -> Self {
        self.label_family = family.into();
        self
    }

    pub fn label_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.label_color = color.into();
        self
    }

    pub fn label_padding(mut self, padding: f32) -> Self {
        self.label_padding = padding;
        self
    }

    // --- Tick ---

    pub fn tick_label_size(mut self, size: f32) -> Self {
        self.tick_label_size = size;
        self
    }

    pub fn tick_label_family(mut self, family: impl Into<String>) -> Self {
        self.tick_label_family = family.into();
        self
    }

    pub fn tick_label_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.tick_label_color = color.into();
        self
    }

    pub fn tick_label_padding(mut self, padding: f32) -> Self {
        self.tick_label_padding = padding;
        self
    }

    pub fn x_tick_label_angle(mut self, angle: f32) -> Self {
        self.x_tick_label_angle = angle;
        self
    }

    pub fn y_tick_label_angle(mut self, angle: f32) -> Self {
        self.y_tick_label_angle = angle;
        self
    }

    // --- Stroke ---

    pub fn axis_width(mut self, width: f32) -> Self {
        self.axis_width = width;
        self
    }

    pub fn tick_width(mut self, width: f32) -> Self {
        self.tick_width = width;
        self
    }
    
    pub fn tick_length(mut self, length: f32) -> Self {
        self.tick_length = length;
        self
    }

    // --- Legend ---

    pub fn legend_label_size(mut self, size: Option<f32>) -> Self {
        self.legend_label_size = size;
        self
    }

    pub fn legend_label_family(mut self, family: Option<String>) -> Self {
        self.legend_label_family = family;
        self
    }

    pub fn legend_label_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.legend_label_color = color.into();
        self
    }

    pub fn legend_block_gap(mut self, gap: f32) -> Self {
        self.legend_block_gap = gap;
        self
    }

    pub fn legend_item_v_gap(mut self, gap: f32) -> Self {
        self.legend_item_v_gap = gap;
        self
    }

    pub fn legend_col_h_gap(mut self, gap: f32) -> Self {
        self.legend_col_h_gap = gap;
        self
    }

    pub fn legend_title_gap(mut self, gap: f32) -> Self {
        self.legend_title_gap = gap;
        self
    }

    pub fn legend_marker_text_gap(mut self, gap: f32) -> Self {
        self.legend_marker_text_gap = gap;
        self
    }

    // --- Layout Defense ---

    pub fn min_panel_size(mut self, size: f32) -> Self {
        self.min_panel_size = size;
        self
    }

    pub fn panel_defense_ratio(mut self, ratio: f32) -> Self {
        self.panel_defense_ratio = ratio;
        self
    }

    pub fn axis_reserve_buffer(mut self, buffer: f32) -> Self {
        self.axis_reserve_buffer = buffer;
        self
    }

    // --- Color & Scale ---

    pub fn color_map(mut self, map: ColorMap) -> Self {
        self.color_map = map;
        self
    }

    pub fn palette(mut self, palette: ColorPalette) -> Self {
        self.palette = palette;
        self
    }

    pub fn x_expand(mut self, expand: Expansion) -> Self {
        self.x_expand = expand;
        self
    }

    pub fn y_expand(mut self, expand: Expansion) -> Self {
        self.y_expand = expand;
        self
    }

    pub fn color_expand(mut self, expand: Expansion) -> Self {
        self.color_expand = expand;
        self
    }

    pub fn shape_expand(mut self, expand: Expansion) -> Self {
        self.shape_expand = expand;
        self
    }

    pub fn size_expand(mut self, expand: Expansion) -> Self {
        self.size_expand = expand;
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
            tick_label_family: font_stack,
            tick_label_color: "#333".into(),
            tick_label_padding: 3.0,

            x_tick_label_angle: 0.0,
            y_tick_label_angle: 0.0,

            axis_width: 1.0,
            tick_width: 1.0,
            tick_length: 6.0,

            legend_label_size: None,
            legend_label_family: None,
            legend_label_color: "#333".into(),
            legend_block_gap: 35.0,
            legend_item_v_gap: 3.0,
            legend_col_h_gap: 15.0,
            legend_title_gap: 7.0,
            legend_marker_text_gap: 8.0,

            min_panel_size: 100.0,
            panel_defense_ratio: 0.2,
            axis_reserve_buffer: 60.0,

            color_map: ColorMap::Viridis,
            palette: ColorPalette::Tab10,

            x_expand: Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) },
            y_expand: Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) },
            color_expand: Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) },
            shape_expand: Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) },
            size_expand: Expansion { mult: (0.05, 0.05), add: (0.0, 0.0) },
        }
    }
}