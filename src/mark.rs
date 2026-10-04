pub(crate) mod area;
pub(crate) mod bar;
pub(crate) mod boxplot;
pub(crate) mod contour;
pub(crate) mod density;
pub(crate) mod density_2d;
pub(crate) mod errorbar;
pub(crate) mod geo_path;
pub(crate) mod histogram;
pub(crate) mod line;
pub(crate) mod no_mark;
pub(crate) mod point;
pub(crate) mod rect;
pub(crate) mod rule;
pub(crate) mod text;
pub(crate) mod tick;
pub(crate) mod violin;

use crate::prelude::SingleColor;
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
pub trait Mark: Clone + 'static {
    /// Used to identify mark type
    fn mark_type(&self) -> &'static str;

    /// Returns the stroke color of the mark
    ///
    /// This method provides access to the stroke color setting of the mark.
    /// If no stroke color is set, it returns None.
    ///
    /// # Returns
    /// * `crate::visual::color::SingleColor`
    fn stroke(&self) -> crate::visual::color::SingleColor {
        SingleColor::new("none")
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

    /// Returns this mark's violin configuration, when it is a violin.
    ///
    /// A composite mark exposes its configuration here so the shared transform
    /// dispatch can read it. Every other mark returns `None`.
    fn as_violin(&self) -> Option<&crate::mark::violin::MarkViolin> {
        None
    }

    /// Mutable counterpart of [`Mark::as_violin`].
    fn as_violin_mut(&mut self) -> Option<&mut crate::mark::violin::MarkViolin> {
        None
    }

    /// Returns this mark's contour configuration, when it is a contour.
    ///
    /// See [`Mark::as_violin`].
    fn as_contour(&self) -> Option<&crate::mark::contour::MarkContour> {
        None
    }

    /// Mutable counterpart of [`Mark::as_contour`].
    fn as_contour_mut(&mut self) -> Option<&mut crate::mark::contour::MarkContour> {
        None
    }

    /// Returns this mark's density configuration, when it is a density.
    ///
    /// See [`Mark::as_violin`].
    fn as_density(&self) -> Option<&crate::mark::density::MarkDensity> {
        None
    }

    /// Mutable counterpart of [`Mark::as_density`].
    fn as_density_mut(&mut self) -> Option<&mut crate::mark::density::MarkDensity> {
        None
    }

    /// Returns this mark's 2-D density configuration, when it is one.
    ///
    /// See [`Mark::as_violin`].
    fn as_density_2d(&self) -> Option<&crate::mark::density_2d::MarkDensity2D> {
        None
    }

    /// Mutable counterpart of [`Mark::as_density_2d`].
    fn as_density_2d_mut(&mut self) -> Option<&mut crate::mark::density_2d::MarkDensity2D> {
        None
    }
}
