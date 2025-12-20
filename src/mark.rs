pub(crate) mod arc;
pub(crate) mod area;
pub(crate) mod bar;
pub(crate) mod boxplot;
pub(crate) mod errorbar;
pub(crate) mod histogram;
pub(crate) mod line;
pub(crate) mod point;
pub(crate) mod rect;
pub(crate) mod rule;
pub(crate) mod text;

/// A trait representing a visual mark in a plot.
///
/// This trait defines the common interface for all visual marks that can be used
/// in plots, such as points, lines, bars, etc. Each mark type must implement
/// this trait to provide information about its visual properties.
///
/// The trait requires implementing types to also implement `Clone` to allow
/// for easy duplication of mark configurations.
///
/// # Required Methods
///
/// Implementors must provide:
/// - `mark_type`: Returns a string identifier for the mark type
///
/// # Provided Methods
///
/// Default implementations are provided for:
/// - `stroke`: Returns the stroke color (defaults to None)
/// - `shape`: Returns the point shape (defaults to Circle)
/// - `opacity`: Returns the opacity value (defaults to 1.0)
pub trait Mark: Clone {
    /// Used to identify mark type
    fn mark_type(&self) -> &'static str;

    /// Returns the stroke color of the mark
    ///
    /// This method provides access to the stroke color setting of the mark.
    /// If no stroke color is set, it returns None.
    ///
    /// # Returns
    /// * `Option<&crate::visual::color::SingleColor>` - A reference to the stroke color, or None if not set
    fn stroke(&self) -> Option<&crate::visual::color::SingleColor> {
        None
    }

    /// Returns the shape of the point
    ///
    /// # Arguments
    /// * `self` - Reference to the instance
    ///
    /// # Returns
    /// Returns a PointShape enum value representing the visual shape of the point
    ///
    /// # Description
    /// This function returns a circular shape by default for point visualization
    fn shape(&self) -> crate::visual::shape::PointShape {
        crate::visual::shape::PointShape::Circle
    }

    /// Returns the opacity of the mark
    fn opacity(&self) -> f64 {
        1.0 // Default fully opaque
    }
}
