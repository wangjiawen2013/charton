use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Represents a text mark with its properties
///
/// The `MarkText` struct defines the visual properties of text elements used in
/// chart annotations and labels. It controls the appearance of text including its
/// color, size, opacity, content, alignment, and baseline positioning.
///
/// Text marks are used to add descriptive labels, annotations, or data-driven text
/// elements to visualizations. They support various alignment options and can display
/// either static text or dynamic text content derived from data fields.
///
/// # Color Handling
///
/// In text charts, colors can be assigned based on data categories or groups.
/// When color encoding is used, each text element will be assigned a color from the
/// palette system to distinguish between different data series or categories.
/// When no explicit color encoding is provided, the `color` field in this struct
/// serves as the default fill color for all text elements. For multi-series text charts,
/// different series are automatically assigned different colors from the palette
/// to distinguish them.
#[derive(Debug, Clone)]
pub struct MarkText {
    // Text color
    pub(crate) color: Option<SingleColor>,
    // Text size
    pub(crate) size: f64,
    // Text opacity
    pub(crate) opacity: f64,
    // Text content (if static)
    pub(crate) text: String,
    // Text anchor (start, middle, end)
    pub(crate) anchor: TextAnchor,
    // Text baseline (auto, middle, etc.)
    pub(crate) baseline: TextBaseline,
}

/// Text anchor options
///
/// The `TextAnchor` enum defines the horizontal alignment of text elements relative
/// to their anchor point. This controls how text is positioned horizontally when rendered.
#[derive(Debug, Clone)]
#[derive(Default)]
pub enum TextAnchor {
    /// Left-align text to the anchor point
    Start,
    /// Center text on the anchor point
    #[default]
    Middle,
    /// Right-align text to the anchor point
    End,
}


/// Text baseline options
///
/// The `TextBaseline` enum defines the vertical alignment of text elements relative
/// to their anchor point. This controls how text is positioned vertically when rendered.
#[derive(Debug, Clone)]
#[derive(Default)]
pub enum TextBaseline {
    /// Use the default baseline determined by the browser
    #[default]
    Auto,
    /// Align the middle of the text with the anchor point
    Middle,
    /// Align the top of the text with the anchor point
    Hanging,
}


impl MarkText {
    // Create a new text mark with default properties
    pub(crate) fn new() -> Self {
        Self {
            color: Some(SingleColor::new("black")),
            size: 12.0,
            opacity: 1.0,
            text: "".to_string(),
            anchor: TextAnchor::default(),
            baseline: TextBaseline::default(),
        }
    }
}

impl Default for MarkText {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkText {
    fn mark_type(&self) -> &'static str {
        "text"
    }
}
