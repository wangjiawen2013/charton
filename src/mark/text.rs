use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for text annotations and labels.
///
/// The `MarkText` struct defines the visual properties of text elements.
/// It supports a fluent interface for configuring font size, alignment, 
/// color, and content.
#[derive(Debug, Clone)]
pub struct MarkText {
    pub(crate) text: String,
    pub(crate) color: SingleColor,
    pub(crate) font_size: f64,
    pub(crate) font_family: String,
    pub(crate) font_weight: FontWeight,
    pub(crate) text_anchor: TextAnchor,
    pub(crate) opacity: f64,
}

/// Font weight options for text elements.
#[derive(Debug, Clone, Default)]
pub enum FontWeight {
    /// Normal font weight (equivalent to 400).
    #[default]
    Normal,
    /// Bold font weight (equivalent to 700).
    Bold,
    /// Specific numeric weight (e.g., 100, 300, 900).
    Weight(u16),
}

impl From<&str> for FontWeight {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "bold" => FontWeight::Bold,
            "normal" => FontWeight::Normal,
            _ => {
                // Try to parse numeric strings like "300"
                s.parse::<u16>().map(FontWeight::Weight).unwrap_or(FontWeight::Normal)
            }
        }
    }
}

// Helper for backend conversion.
impl std::fmt::Display for FontWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontWeight::Normal => write!(f, "normal"),
            FontWeight::Bold => write!(f, "bold"),
            FontWeight::Weight(w) => write!(f, "{}", w),
        }
    }
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

impl From<&str> for TextAnchor {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "start" | "left" => TextAnchor::Start,
            "end" | "right"  => TextAnchor::End,
            _ => TextAnchor::Middle, // Default
        }
    }
}

// Facilitates conversion for the rendering backend.
impl std::fmt::Display for TextAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextAnchor::Start => write!(f, "start"),
            TextAnchor::Middle => write!(f, "middle"),
            TextAnchor::End => write!(f, "end"),
        }
    }
}

impl MarkText {
    /// Create a new text mark with default properties.
    pub(crate) fn new() -> Self {
        let font_stack = "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, 'PingFang SC', 'Microsoft YaHei', Ubuntu, Cantarell, 'Noto Sans', sans-serif".to_string();
        Self {
            text: String::new(),
            color: SingleColor::new("black"),
            font_size: 12.0,
            font_family: font_stack,
            font_weight: "normal".into(),
            text_anchor: TextAnchor::default(),
            opacity: 1.0,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the text color. Accepts "red", "#hex", etc.
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the font size of the text.
    pub fn with_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    /// Sets the static text content.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Sets the font weight of the text.
    pub fn with_weight(mut self, anchor: impl Into<FontWeight>) -> Self {
        self.font_weight = anchor.into();
        self
    }

    /// Sets the horizontal text anchor point.
    pub fn with_anchor(mut self, anchor: impl Into<TextAnchor>) -> Self {
        self.text_anchor = anchor.into();
        self
    }

    /// Sets the opacity of the text mark.
    /// 
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
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