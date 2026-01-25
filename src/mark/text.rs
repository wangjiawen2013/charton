use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for text annotations and labels.
///
/// The `MarkText` struct defines the visual properties of text elements.
/// It supports a fluent interface for configuring font size, alignment, 
/// color, and content.
#[derive(Debug, Clone)]
pub struct MarkText {
    pub(crate) color: SingleColor,
    pub(crate) size: f32,
    pub(crate) opacity: f32,
    pub(crate) text: String,
    pub(crate) anchor: TextAnchor,
    pub(crate) baseline: TextBaseline,
}

/// Horizontal alignment options for text elements.
#[derive(Debug, Clone, Default)]
pub enum TextAnchor {
    /// Left-align text.
    Start,
    /// Center text.
    #[default]
    Middle,
    /// Right-align text.
    End,
}

/// Vertical alignment options for text elements.
#[derive(Debug, Clone, Default)]
pub enum TextBaseline {
    /// Browser-determined default.
    #[default]
    Auto,
    /// Vertical center.
    Middle,
    /// Top-aligned.
    Hanging,
}

impl MarkText {
    /// Create a new text mark with default properties.
    pub(crate) fn new() -> Self {
        Self {
            // (1.000, 0.498, 0.153, 0.016), // #7f2704
            color: SingleColor::new("black"),
            size: 12.0,
            opacity: 1.0,
            text: String::new(),
            anchor: TextAnchor::default(),
            baseline: TextBaseline::default(),
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the text color. Accepts "red", "#hex", etc.
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the font size of the text.
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets the opacity of the text mark.
    /// 
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the static text content.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Sets the horizontal text anchor point.
    pub fn with_anchor(mut self, anchor: impl Into<TextAnchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Sets the vertical text baseline positioning.
    pub fn with_baseline(mut self, baseline: impl Into<TextBaseline>) -> Self {
        self.baseline = baseline.into();
        self
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