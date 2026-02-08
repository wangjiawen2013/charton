use crate::chart::Chart;
use crate::mark::arc::MarkArc;
use crate::encode::x::X;
use crate::encode::y::Y;
use crate::encode::color::Color;

impl<T: crate::mark::Mark> Chart<T> {
    /// Entry point for circular charts.
    pub fn mark_arc(self) -> Chart<MarkArc> {
        Chart::<MarkArc> {
            data: self.data,
            encoding: self.encoding,
            mark: Some(MarkArc::default()),
        }
    }
}

impl Chart<MarkArc> {
    pub fn configure_arc<F>(mut self, f: F) -> Self 
    where F: FnOnce(MarkArc) -> MarkArc {
        if let Some(m) = self.mark.take() {
            self.mark = Some(f(m));
        }
        self
    }
}

// --- Semantic Aliases ---

pub fn theta(field: &str) -> Y {
    let mut ax = Y::new(field);
    ax.stack = true; // Implicitly enable stacking for Pie behavior
    ax
}

pub fn radius(field: &str) -> X {
    X::new(field)
}