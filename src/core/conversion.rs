use crate::chart::Chart;
use crate::coordinate::CoordSystem;
use crate::core::composite::LayeredChart;
use crate::core::layer::Layer;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::scale::{Expansion, IntoExplicitTicks, ScaleDomain};
use crate::theme::Theme;

/// A unified interface for configuring and rendering visualizations and API.
///
/// This trait enables "Type Promotion": calling these methods on a single [Chart]
/// automatically converts it into a [LayeredChart] container. Because [LayeredChart]
/// also implements this trait, you can chain these methods regardless of the
/// underlying structure.
pub trait IntoLayered: Into<LayeredChart> + Clone {
    /// Combines this visual with another one to create a multi-layer specification.
    ///
    /// This is the core of Charton's "Layering Grammar." It accepts anything
    /// that can be converted into a [LayeredChart] (e.g., another [Chart] or
    /// an existing [LayeredChart]).
    fn and<L: Into<LayeredChart>>(self, other: L) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        let mut other_lc: LayeredChart = other.into();

        // 1. Merge layers
        lc.layers.append(&mut other_lc.layers);

        // 2. Resolve metadata (Optional: Left-side priority)
        // If the left side doesn't have a title/label, take it from the right side.
        if lc.title.is_none() {
            lc.title = other_lc.title;
        }
        if lc.x_label.is_none() {
            lc.x_label = other_lc.x_label;
        }
        if lc.y_label.is_none() {
            lc.y_label = other_lc.y_label;
        }

        lc
    }

    // --- Physical Dimensions ---

    /// Sets the target dimensions of the chart in pixels.
    fn with_size(self, width: u32, height: u32) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.width = width;
        lc.height = height;
        lc
    }

    // --- Layout Margins ---

    fn with_top_margin(self, margin: f64) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.top_margin = Some(margin);
        lc
    }

    fn with_right_margin(self, margin: f64) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.right_margin = Some(margin);
        lc
    }

    fn with_bottom_margin(self, margin: f64) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.bottom_margin = Some(margin);
        lc
    }

    fn with_left_margin(self, margin: f64) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.left_margin = Some(margin);
        lc
    }

    /// Configures the chart margins [top, right, bottom, left].
    fn with_margins(self, top: f64, right: f64, bottom: f64, left: f64) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.top_margin = Some(top);
        lc.right_margin = Some(right);
        lc.bottom_margin = Some(bottom);
        lc.left_margin = Some(left);
        lc
    }

    // --- Aesthetic Styling ---

    fn with_theme(self, theme: Theme) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.theme = theme;
        lc
    }

    /// Provides a closure to modify the existing theme fluently.
    fn configure_theme<F>(self, f: F) -> LayeredChart
    where
        F: FnOnce(Theme) -> Theme,
    {
        let mut lc: LayeredChart = self.into();
        lc.theme = f(lc.theme);
        lc
    }

    /// Sets the global chart title.
    fn with_title<S: Into<String>>(self, title: S) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.title = Some(title.into());
        lc
    }

    // --- Axis Data & Scale Configuration ---

    /// Set the global X-axis domain, overriding automatic data range calculation.
    fn with_x_domain(self, min: f64, max: f64) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.x_domain = Some(ScaleDomain::Continuous(min, max));
        lc
    }

    /// Set the X-axis expansion (padding).
    fn with_x_expand(self, expand: Expansion) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.x_expand = Some(expand);
        lc
    }

    fn with_x_label<S: Into<String>>(self, label: S) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.x_label = Some(label.into());
        lc
    }

    fn with_x_ticks<T: IntoExplicitTicks>(self, ticks: T) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.x_ticks = Some(ticks.into_explicit_ticks());
        lc
    }

    /// Set the global Y-axis domain.
    fn with_y_domain(self, min: f64, max: f64) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.y_domain = Some(ScaleDomain::Continuous(min, max));
        lc
    }

    /// Set the Y-axis expansion (padding).
    fn with_y_expand(self, expand: Expansion) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.y_expand = Some(expand);
        lc
    }

    fn with_y_label<S: Into<String>>(self, label: S) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.y_label = Some(label.into());
        lc
    }

    fn with_y_ticks<T: IntoExplicitTicks>(self, ticks: T) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.y_ticks = Some(ticks.into_explicit_ticks());
        lc
    }

    fn with_shape_label<S: Into<String>>(self, label: S) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.shape_label = Some(label.into());
        lc
    }

    fn with_size_label<S: Into<String>>(self, label: S) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.size_label = Some(label.into());
        lc
    }

    // --- Coordinate System ---

    /// Swaps the X and Y axes (common for horizontal charts).
    fn coord_flip(self) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.flipped = true;
        lc
    }

    /// Sets the coordinate system for the chart (e.g., Cartesian, Polar).
    fn with_coord(self, coord: CoordSystem) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.coord_system = coord;
        lc
    }

    // --- Polar Context Overrides ---

    /// Sets the starting angle for polar coordinates (in radians).
    fn with_start_angle(self, angle: f64) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.polar_start_angle = Some(angle);
        lc
    }

    /// Sets the total angular span for polar coordinates (in radians).
    fn with_end_angle(self, angle: f64) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.polar_end_angle = Some(angle);
        lc
    }

    /// Sets the inner radius ratio (0.0 to 1.0) for donut charts.
    fn with_inner_radius(self, radius: f64) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.polar_inner_radius = Some(radius);
        lc
    }

    // --- Terminal Actions ---

    /// Generates and returns the SVG representation of the chart.
    ///
    /// This method renders the entire chart as an SVG (Scalable Vector Graphics) string,
    /// including all layers, axes, labels, legends, and other visual elements.
    ///
    /// # Returns
    /// A [Result] containing the complete SVG markup or a [ChartonError].
    fn to_svg(&self) -> Result<String, ChartonError> {
        let lc: LayeredChart = self.clone().into();
        lc.to_svg()
    }

    fn show(&self) -> Result<(), ChartonError> {
        let lc: LayeredChart = self.clone().into();
        lc.show()
    }

    fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ChartonError> {
        let lc: LayeredChart = self.clone().into();
        lc.save(path)
    }
}

// Enable the trait for Chart
impl<T: crate::mark::Mark + 'static> IntoLayered for crate::chart::Chart<T> where
    crate::chart::Chart<T>: crate::core::layer::Layer + Clone
{
}

// Enable the trait for LayeredChart (Identical conversion)
impl IntoLayered for LayeredChart {}

impl<T: Mark + 'static> From<Chart<T>> for LayeredChart
where
    Chart<T>: Layer + Clone,
{
    /// Converts a single-layer chart into a layered chart by adding it as the first layer.
    ///
    /// This method creates a new [LayeredChart] with default settings and adds the input
    /// chart as its first layer. The resulting [LayeredChart] can then accept additional
    /// layers via the [LayeredChart::add_layer] method.
    ///
    /// # Arguments
    /// * `val` - The source [Chart] to convert into a [LayeredChart]
    ///
    /// # Returns
    /// A new [LayeredChart] instance with the input chart as its first layer
    fn from(val: Chart<T>) -> Self {
        LayeredChart::new().add_layer(val)
    }
}
