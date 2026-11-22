use crate::chart::Chart;
use crate::coordinate::CoordSystem;
use crate::core::composite::LayeredChart;
use crate::core::layer::Layer;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::scale::{Expansion, IntoExplicitTicks, ScaleDomain, formatter::LabelFormat};
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
        if lc.description.is_none() {
            lc.description = other_lc.description;
        }
        if lc.x_label.is_none() {
            lc.x_label = other_lc.x_label;
        }
        if lc.y_label.is_none() {
            lc.y_label = other_lc.y_label;
        }

        lc
    }

    /// Applies faceting to the chart, splitting it into multiple sub-plots.
    ///
    /// This automatically promotes a single Chart into a LayeredChart container
    /// and applies the facet specification.
    fn facet<F>(self, spec: F) -> LayeredChart
    where
        F: Into<crate::facets::FacetSpec>,
    {
        let mut lc: LayeredChart = self.into();
        lc.facet = Some(spec.into());
        lc
    }

    /// Provides a closure to modify the existing facet specification fluently.
    ///
    /// If faceting has already been configured on the chart (e.g. via `.facet(...)`),
    /// this method takes the current [`FacetSpec`](crate::facets::FacetSpec), applies the closure to mutate it,
    /// and saves it back.
    fn configure_facet<F>(self, f: F) -> LayeredChart
    where
        F: FnOnce(crate::facets::FacetSpec) -> crate::facets::FacetSpec,
    {
        let mut lc: LayeredChart = self.into();
        if let Some(spec) = lc.facet.take() {
            lc.facet = Some(f(spec));
        }
        lc
    }

    // --- Physical Dimensions ---

    /// Sets the layout size of the chart in pixels.
    ///
    /// This is also the exact pixel size of raster output under the default
    /// settings: an image built with `with_size(600, 400)` is 600 × 400 pixels.
    /// Small canvases are supersampled internally (2x) and downsampled for
    /// anti-aliasing, so what you set is what you get.
    ///
    /// The one exception is [`with_scale_factor`](Self::with_scale_factor),
    /// which deliberately multiplies the output pixel density.
    fn with_size(self, width: u32, height: u32) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.width = width;
        lc.height = height;
        lc
    }

    /// Sets how much width this view takes when it is a cell of a composition.
    ///
    /// Outside a composition this is ignored: a standalone chart is sized by
    /// [`with_size`](Self::with_size). Inside one, `TrackSize::Fixed(60.0)` pins a
    /// 60px strip (a marginal histogram) and `TrackSize::Fill(1.0)` (the default)
    /// takes a share of whatever is left over.
    ///
    /// ```rust,ignore
    /// let top = hist.with_height(TrackSize::Fixed(60.0));
    /// let right = hist.with_width(TrackSize::Fixed(60.0));
    /// ```
    fn with_width(self, size: impl Into<crate::composition::TrackSize>) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.width_spec = Some(size.into());
        lc
    }

    /// Sets how much height this view takes when it is a cell of a composition.
    ///
    /// See [`with_width`](Self::with_width); a standalone chart is unaffected.
    fn with_height(self, size: impl Into<crate::composition::TrackSize>) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.height_spec = Some(size.into());
        lc
    }

    // --- View Chrome (what this view draws around its data) ---

    /// Hides the x axis: no spine, ticks, labels or title, and no space reserved
    /// for them.
    ///
    /// This is a property of the *view*, not of the mark, because it answers
    /// "does this picture repeat a scale its neighbour already shows?" -- the
    /// question every marginal strip asks. It is also what a shared-axis pair of
    /// panels needs.
    ///
    /// Like the facet strategy's own switch, this names the **edge**: the x axis
    /// is the bottom one. After [`coord_flip`](Self::coord_flip) the bottom edge
    /// carries the y channel, so a flipped marginal (a histogram running
    /// sideways) keeps its count axis with
    /// [`with_show_y_axis(false)`](Self::with_show_y_axis), which hides the left
    /// edge -- the binned variable that the main view already shows.
    ///
    /// ```rust,ignore
    /// // A histogram strip above a scatter: the x scale is the scatter's.
    /// let top = hist.with_show_x_axis(false).with_height(TrackSize::Fixed(60.0));
    /// ```
    fn with_show_x_axis(self, show: bool) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.show_x_axis = Some(show);
        lc
    }

    /// Hides the y axis (the left edge). See
    /// [`with_show_x_axis`](Self::with_show_x_axis) for the flip caveat.
    fn with_show_y_axis(self, show: bool) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.show_y_axis = Some(show);
        lc
    }

    /// Hides this view's own legend.
    ///
    /// The third of the three "view chrome" switches, next to
    /// [`with_show_x_axis`](Self::with_show_x_axis) and
    /// [`with_show_y_axis`](Self::with_show_y_axis), and it answers the same kind of
    /// question: *does this picture repeat something a neighbour already says?* A
    /// marginal strip drawn from the same colour scale as the view it annotates
    /// says it twice, so it turns its own guide off — and a page that hoists a
    /// legend ([`Composition::with_legend_position`](crate::composition::Composition::with_legend_position))
    /// supplies it once at the edge instead.
    ///
    /// It is the same setting as `configure_theme(|t| t.with_show_legend(false))`,
    /// spelled as a switch because that is what it is: a view either shows a legend
    /// or it does not.
    ///
    /// ```rust,ignore
    /// let top = hist.with_show_x_axis(false).with_show_legend(false);
    /// ```
    fn with_show_legend(self, show: bool) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.theme = lc.theme.with_show_legend(show);
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

    fn with_grid(self, show: bool) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.show_grid = Some(show);
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
    ///
    /// The title is drawn on the canvas and, when the SVG accessibility mode is
    /// on, becomes the document's accessible name (`<title>`).
    fn with_title<S: Into<String>>(self, title: S) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.title = Some(title.into());
        lc
    }

    /// Sets a description of the chart.
    ///
    /// The description is never drawn: it is exported as the SVG `<desc>` element
    /// (the accessible description, announced after the title) and appended to
    /// the PDF figure's alternative text. Use it to say what the chart shows and
    /// what its main takeaway is, e.g. "Revenue grows steadily from 2019 to
    /// 2023, then drops in 2024".
    fn with_description<S: Into<String>>(self, description: S) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.description = Some(description.into());
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

    /// Sets the title of the color legend.
    fn with_color_label<S: Into<String>>(self, label: S) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.color_label = Some(label.into());
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

    // --- Label Formatting ---

    /// Formats the X-axis tick labels (e.g. currency, compact notation, precision).
    fn with_x_label_format(self, format: LabelFormat) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.x_format = Some(format);
        lc
    }

    /// Formats the Y-axis tick labels (e.g. currency, compact notation, precision).
    fn with_y_label_format(self, format: LabelFormat) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.y_format = Some(format);
        lc
    }

    /// Formats every legend entry and colour-bar label.
    fn with_legend_label_format(self, format: LabelFormat) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.legend_format = Some(format);
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

    /// Sets the raster pixel-density factor (default `1.0`).
    ///
    /// The layout stays as configured by [`with_size`](Self::with_size); this
    /// factor only multiplies the pixel density of raster output. With the
    /// default `1.0`, `with_size(600, 400)` produces a 600 × 400 image. With
    /// `2.0`, it produces a 1200 × 800 image of the **same layout** — fonts,
    /// stroke widths and spacing scale together, so it is the same picture,
    /// just with more pixels (useful for Retina displays, print, or fitting an
    /// image to a terminal's pixel area).
    ///
    /// # This is not needed for anti-aliasing
    /// `to_png()` already renders internally at 2x and box-downsamples, so the
    /// default output is smooth. Use this method only when you explicitly want
    /// a higher-density *asset*.
    ///
    /// # Scope
    /// - Affects raster outputs (PNG via CPU or GPU).
    /// - Has **no effect on vector outputs** (SVG, PDF), which are
    ///   resolution-independent.
    /// - Values `< 2.0` still receive internal 2x supersampling; values
    ///   `>= 2.0` are rendered at the requested density directly.
    fn with_scale_factor(self, factor: f32) -> LayeredChart {
        let mut lc: LayeredChart = self.into();
        lc.scale_factor = factor;
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

    /// Generates and returns the PDF representation of the chart, with default
    /// [`PdfOptions`](crate::render::pdf::PdfOptions) (a tagged document).
    #[cfg(feature = "pdf")]
    fn to_pdf(&self) -> Result<Vec<u8>, ChartonError> {
        let lc: LayeredChart = self.clone().into();
        lc.to_pdf()
    }

    /// Generates the PDF with explicit [`PdfOptions`](crate::render::pdf::PdfOptions):
    /// a tagged or plain document, compressed or inspectable.
    #[cfg(feature = "pdf")]
    fn to_pdf_with(
        &self,
        options: &crate::render::pdf::PdfOptions,
    ) -> Result<Vec<u8>, ChartonError> {
        let lc: LayeredChart = self.clone().into();
        lc.to_pdf_with(options)
    }

    /// Generates the SVG with explicit [`SvgOptions`](crate::render::svg::SvgOptions):
    /// semantic classes, accessibility metadata, custom `<defs>` and the id
    /// namespace the document uses.
    ///
    /// The options never change the picture, only how the document is written.
    fn to_svg_with(
        &self,
        options: &crate::render::svg::SvgOptions,
    ) -> Result<String, ChartonError> {
        let lc: LayeredChart = self.clone().into();
        lc.to_svg_with(options)
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
    /// layers via the `LayeredChart::add_layer` method.
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
