use crate::mark::Mark;

/// A placeholder mark used for charts that have not yet been assigned a specific
/// visual type (e.g., bar, line, point).
///
/// This allows the "Base Chart" pattern where encodings are defined once and
/// reused across multiple layers.
#[derive(Clone, Default)]
pub struct NoMark;

impl Mark for NoMark {
    fn mark_type(&self) -> &'static str {
        "none"
    }
}
