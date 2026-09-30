use crate::core::guide::LegendPosition;
use crate::prelude::SingleColor;
use crate::visual::color::{ColorMap, ColorPalette};

/// Where the chart title sits across the width it is aligned to.
///
/// The width itself is chosen by [`TitleFrame`]. This only says whether the
/// title hugs the left side, the middle, or the right side of that width.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TitleAnchor {
    /// Against the left side.
    Start,
    /// In the middle.
    #[default]
    Middle,
    /// Against the right side.
    End,
}

/// The width the chart title is aligned to.
///
/// A title that describes the data lines up with the plot panel. A title that
/// belongs to the whole picture lines up with the figure body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TitleFrame {
    /// The area the data is drawn in.
    #[default]
    Panel,
    /// The whole figure, i.e. the canvas inside the outer margins.
    Figure,
}

/// A ready-made light or dark color scheme for a [`Theme`].
///
/// A mode chooses the colors of the canvas, axes, grid, text and default
/// palette, and leaves layout, fonts and sizes untouched, so changing modes
/// never changes the structure of a chart.
///
/// There is no automatic mode: a library cannot know what background it will be
/// shown on. Callers that do know (a CLI, a GUI, the Nushell plugin) detect the
/// environment and pick [`Light`] or [`Dark`].
///
/// [`Light`]: ThemeMode::Light
/// [`Dark`]: ThemeMode::Dark
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeMode {
    /// Dark ink and chrome for light backgrounds. The library default.
    #[default]
    Light,
    /// Light ink and chrome for dark backgrounds (Catppuccin Mocha palette).
    Dark,
}

impl ThemeMode {
    /// Whether this mode targets a dark background.
    pub const fn is_dark(self) -> bool {
        matches!(self, ThemeMode::Dark)
    }
}

impl std::str::FromStr for ThemeMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "light" | "latte" => Ok(ThemeMode::Light),
            "dark" | "mocha" => Ok(ThemeMode::Dark),
            other => Err(format!("unknown theme '{other}'; expected light or dark")),
        }
    }
}

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

    /// Whether to render grid lines
    pub(crate) show_grid: bool,
    pub(crate) grid_color: SingleColor,
    pub(crate) grid_width: f64,

    // --- Main Title Styling ---
    /// Font size for the main chart title.
    pub(crate) title_size: f64,
    /// Font family stack for the main chart title.
    pub(crate) title_family: String,
    /// Text color for the main chart title.
    pub(crate) title_color: SingleColor,
    /// Space between the title and whatever comes next below it.
    pub(crate) title_padding: f64,
    /// Where the title sits across the width it is aligned to.
    pub(crate) title_anchor: TitleAnchor,
    /// The width the title is aligned to.
    pub(crate) title_frame: TitleFrame,

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
    pub(crate) axes_color: SingleColor,
    /// Width of the small tick marks.
    pub(crate) tick_width: f64,
    pub(crate) tick_color: SingleColor,
    /// The physical length of the tick marks.
    pub(crate) tick_length: f64,
    /// Minimum pixel spacing between ticks to ensure visual density.
    pub(crate) tick_min_spacing: f64,

    // --- Legend Styling ---
    pub(crate) show_legend: bool,
    /// Font size for the legend's title.
    pub(crate) legend_title_size: f64,
    pub(crate) legend_title_color: SingleColor,
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
}

impl Theme {
    // --- Presets ---

    /// The default light theme: dark text on a light background.
    pub fn light() -> Self {
        Self::default()
    }

    /// A dark theme tuned for dark terminals and dark-mode output.
    ///
    /// Starts from [`Theme::default`] and overrides only the colors that define
    /// the look against a dark background; layout, typography and sizing stay
    /// identical to the default theme.
    pub fn dark() -> Self {
        Self {
            background_color: "#1E1E2E".into(), // Catppuccin Mocha base
            axes_color: "#6C7086".into(),       // overlay0
            tick_color: "#6C7086".into(),
            grid_color: "#313244".into(),  // surface0
            title_color: "#CDD6F4".into(), // text
            label_color: "#CDD6F4".into(),
            tick_label_color: "#BAC2DE".into(), // subtext1
            legend_title_color: "#CDD6F4".into(),
            legend_label_color: "#BAC2DE".into(),
            facet_label_color: "#CDD6F4".into(),
            facet_strip_fill: "#313244".into(),
            palette: [
                "#89B4FA", // blue
                "#F38BA8", // red
                "#A6E3A1", // green
                "#F9E2AF", // yellow
                "#CBA6F7", // mauve
                "#94E2D5", // teal
                "#FAB387", // peach
                "#F5C2E7", // pink
            ]
            .into(),
            ..Self::default()
        }
    }

    /// Switch to the given preset, replacing every color of this theme.
    ///
    /// The presets differ only in color, so layout and typography are the same
    /// either way. Per-chart overrides (palette, background, ...) should be
    /// applied after this call so they win over the preset.
    pub fn with_mode(self, mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Theme::light(),
            ThemeMode::Dark => Theme::dark(),
        }
    }

    /// Whether this theme is intended for a dark background.
    ///
    /// Backends can use this to pick contrast-dependent details (for example
    /// antialiasing or a fallback color when the background is transparent).
    pub const fn is_dark(&self) -> bool {
        let [r, g, b, _] = self.background_color.rgba();
        (0.2126 * r + 0.7152 * g + 0.0722 * b) < 0.5
    }

    // --- Global Configuration ---

    pub fn with_background_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.background_color = color.into();
        self
    }

    pub const fn with_top_margin(mut self, margin: f64) -> Self {
        self.top_margin = margin;
        self
    }

    pub const fn with_right_margin(mut self, margin: f64) -> Self {
        self.right_margin = margin;
        self
    }

    pub const fn with_bottom_margin(mut self, margin: f64) -> Self {
        self.bottom_margin = margin;
        self
    }

    pub const fn with_left_margin(mut self, margin: f64) -> Self {
        self.left_margin = margin;
        self
    }

    pub const fn with_show_axes(mut self, show: bool) -> Self {
        self.show_axes = show;
        self
    }

    pub const fn with_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    pub fn with_grid_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.grid_color = color.into();
        self
    }

    pub const fn with_grid_width(mut self, width: f64) -> Self {
        self.grid_width = width;
        self
    }

    // --- Title ---

    pub const fn with_title_size(mut self, size: f64) -> Self {
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

    pub const fn with_title_padding(mut self, padding: f64) -> Self {
        self.title_padding = padding;
        self
    }

    /// Places the title against the left side, the middle, or the right side of
    /// the width it is aligned to.
    pub const fn with_title_anchor(mut self, anchor: TitleAnchor) -> Self {
        self.title_anchor = anchor;
        self
    }

    /// Chooses whether the title lines up with the data area or with the whole
    /// figure.
    pub const fn with_title_frame(mut self, frame: TitleFrame) -> Self {
        self.title_frame = frame;
        self
    }

    // --- Axis Label ---

    pub const fn with_label_size(mut self, size: f64) -> Self {
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

    pub const fn with_label_padding(mut self, padding: f64) -> Self {
        self.label_padding = padding;
        self
    }

    // --- Tick Label ---

    pub const fn with_tick_label_size(mut self, size: f64) -> Self {
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

    pub const fn with_tick_label_padding(mut self, padding: f64) -> Self {
        self.tick_label_padding = padding;
        self
    }

    pub const fn with_x_tick_label_angle(mut self, angle: f64) -> Self {
        self.x_tick_label_angle = angle;
        self
    }

    pub const fn with_y_tick_label_angle(mut self, angle: f64) -> Self {
        self.y_tick_label_angle = angle;
        self
    }

    // --- Geometry Strokes ---

    pub const fn with_axis_width(mut self, width: f64) -> Self {
        self.axis_width = width;
        self
    }

    pub fn with_axes_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.axes_color = color.into();
        self
    }

    pub const fn with_tick_width(mut self, width: f64) -> Self {
        self.tick_width = width;
        self
    }

    pub fn with_tick_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.tick_color = color.into();
        self
    }

    pub const fn with_tick_length(mut self, length: f64) -> Self {
        self.tick_length = length;
        self
    }

    pub const fn with_tick_min_spacing(mut self, spacing: f64) -> Self {
        self.tick_min_spacing = spacing;
        self
    }

    // --- Legend Styling ---

    pub const fn with_show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    pub const fn with_legend_title_size(mut self, size: f64) -> Self {
        self.legend_title_size = size;
        self
    }

    pub fn with_legend_title_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.legend_title_color = color.into();
        self
    }

    pub const fn with_legend_label_size(mut self, size: f64) -> Self {
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

    pub const fn with_legend_block_gap(mut self, gap: f64) -> Self {
        self.legend_block_gap = gap;
        self
    }

    pub const fn with_legend_item_v_gap(mut self, gap: f64) -> Self {
        self.legend_item_v_gap = gap;
        self
    }

    pub const fn with_legend_col_h_gap(mut self, gap: f64) -> Self {
        self.legend_col_h_gap = gap;
        self
    }

    pub const fn with_legend_title_gap(mut self, gap: f64) -> Self {
        self.legend_title_gap = gap;
        self
    }

    pub const fn with_legend_marker_text_gap(mut self, gap: f64) -> Self {
        self.legend_marker_text_gap = gap;
        self
    }

    // --- Legend Logic ---

    pub const fn with_legend_position(mut self, position: LegendPosition) -> Self {
        self.legend_position = position;
        self
    }

    pub const fn with_legend_margin(mut self, margin: f64) -> Self {
        self.legend_margin = margin;
        self
    }

    // --- Layout Defense ---

    pub const fn with_min_panel_size(mut self, size: f64) -> Self {
        self.min_panel_size = size;
        self
    }

    pub const fn with_panel_defense_ratio(mut self, ratio: f64) -> Self {
        self.panel_defense_ratio = ratio;
        self
    }

    pub const fn with_axis_reserve_buffer(mut self, buffer: f64) -> Self {
        self.axis_reserve_buffer = buffer;
        self
    }

    // --- Color & Palette Defaults ---

    pub const fn with_color_map(mut self, map: ColorMap) -> Self {
        self.color_map = map;
        self
    }

    pub fn with_palette<P: Into<ColorPalette>>(mut self, palette: P) -> Self {
        self.palette = palette.into();
        self
    }

    // --- Facet Styling ---
    /// The font size for the facet labels (the text in the strip).
    pub const fn with_facet_label_size(mut self, size: f64) -> Self {
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
    pub const fn with_facet_spacing(mut self, spacing: f64) -> Self {
        self.facet_spacing = spacing;
        self
    }
    /// The padding inside the facet strip.
    pub const fn with_facet_strip_padding(mut self, padding: f64) -> Self {
        self.facet_strip_padding = padding;
        self
    }

    /// Calculates a suggested number of ticks based on the available
    /// physical space and the theme's density settings.
    pub fn suggest_tick_count(&self, available_pixels: f64) -> usize {
        // We ensure at least 2 ticks (start and end) are always present.
        ((available_pixels / self.tick_min_spacing).floor() as usize).max(2)
    }
}

impl Default for Theme {
    fn default() -> Self {
        let font_stack = "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, 'PingFang SC', 'Microsoft YaHei', Ubuntu, Cantarell, 'Noto Sans', sans-serif".to_string();

        Self {
            background_color: "white".into(),
            top_margin: 0.05,
            right_margin: 0.03,
            bottom_margin: 0.08,
            left_margin: 0.06,

            show_axes: true,

            show_grid: false,
            grid_color: " #BDBDBD".into(),
            grid_width: 1.0,

            title_size: 18.0,
            title_family: font_stack.clone(),
            title_color: "#333".into(),
            title_padding: 6.0,
            title_anchor: TitleAnchor::Middle,
            title_frame: TitleFrame::Panel,

            label_size: 15.0,
            label_family: font_stack.clone(),
            label_color: "#333".into(),
            label_padding: 5.0,

            tick_label_size: 13.0,
            tick_label_family: font_stack.clone(),
            tick_label_color: "#333".into(),
            tick_label_padding: 3.0,

            x_tick_label_angle: 0.0,
            y_tick_label_angle: 0.0,

            axis_width: 1.0,
            axes_color: "black".into(),
            tick_width: 1.0,
            tick_color: "black".into(),
            tick_length: 6.0,
            tick_min_spacing: 50.0,

            show_legend: true,
            legend_title_color: "#333".into(),
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_mode_parses_and_rejects() {
        assert_eq!("light".parse::<ThemeMode>(), Ok(ThemeMode::Light));
        assert_eq!("Dark".parse::<ThemeMode>(), Ok(ThemeMode::Dark));
        assert_eq!("mocha".parse::<ThemeMode>(), Ok(ThemeMode::Dark));
        assert_eq!("latte".parse::<ThemeMode>(), Ok(ThemeMode::Light));
        assert!("blue".parse::<ThemeMode>().is_err());
    }

    #[test]
    fn dark_preset_marks_itself_dark() {
        assert!(Theme::dark().is_dark());
        assert!(!Theme::light().is_dark());
        assert!(!Theme::default().is_dark());
    }

    fn rgba(c: SingleColor) -> [f32; 4] {
        c.rgba()
    }

    #[test]
    fn dark_preset_only_changes_colors() {
        let dark = Theme::dark();
        let light = Theme::default();
        // Layout / typography must be untouched.
        assert_eq!(dark.title_size, light.title_size);
        assert_eq!(dark.top_margin, light.top_margin);
        assert_eq!(dark.grid_width, light.grid_width);
        assert_eq!(dark.min_panel_size, light.min_panel_size);
        // But the ink and background do change.
        assert_ne!(rgba(dark.background_color), rgba(light.background_color));
        assert_ne!(rgba(dark.title_color), rgba(light.title_color));
    }

    #[test]
    fn with_mode_matches_presets() {
        let dark = Theme::light().with_mode(ThemeMode::Dark);
        assert!(dark.is_dark());
        let light = Theme::dark().with_mode(ThemeMode::Light);
        assert!(!light.is_dark());
    }

    #[test]
    fn dark_preset_overrides_palette() {
        let dark = Theme::dark();
        let ColorPalette::Custom(colors) = dark.palette else {
            panic!("dark theme should use a custom palette");
        };
        assert_eq!(colors.len(), 8);
        // First entry is Catppuccin Mocha blue #89B4FA.
        assert_eq!(rgba(colors[0])[0], 0x89 as f32 / 255.0);
    }
}
