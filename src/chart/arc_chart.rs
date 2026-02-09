use crate::chart::Chart;
use crate::mark::arc::MarkArc;
use crate::encode::x::X;
use crate::encode::y::Y;

/// Extension implementation for `Chart` to support Arc Charts (MarkArc).
impl Chart<MarkArc> {
    /// Initializes a new `MarkArc` layer.
    pub fn mark_arc(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkArc::default());
        }
        self
    }

    /// Configures the visual properties of the arc mark (e.g., inner_radius, pad_angle).
    pub fn configure_arc<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(MarkArc) -> MarkArc 
    {
        let mark = self.mark.take().unwrap_or_default();
        self.mark = Some(f(mark));
        self
    }
}

// --- SEMANTIC ALIASES ---

/// Maps a data field to the angular axis (Theta/Angle).
/// Internally maps to the Y encoding channel.
pub fn theta(field: &str) -> Y {
    Y::new(field)
}

/// Maps a data field to the radial axis (Radius/Length).
/// Internally maps to the X encoding channel.
pub fn radius(field: &str) -> X {
    X::new(field)
}
