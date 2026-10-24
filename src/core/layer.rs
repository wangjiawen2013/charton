use super::aesthetics::GlobalAesthetics;
use super::data::{Dataset, FacetPartition};
use crate::Precision;
use crate::coordinate::CoordinateTrait;
use crate::core::context::PanelContext;
use crate::core::semantic::{Element, Group};
use crate::encode::Channel;
use crate::error::ChartonError;
use crate::scale::{Expansion, Scale, ScaleDomain};
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;
use std::sync::Arc;

/// Geometry hint for a [`PathConfig`].
///
/// The hint lets a backend choose the cheapest correct pipeline without
/// inspecting the vertex data. It is also the contract for how subpaths are
/// interpreted:
///
/// * [`Simple`](PathTopology::Simple): open polylines. Every subpath is
///   stroked independently and nothing is closed or filled.
/// * [`Convex`](PathTopology::Convex): a single closed, convex ring. Backends
///   may use a triangle fan or another convex fast path.
/// * [`Complex`](PathTopology::Complex): one or more closed rings filled with
///   the **even-odd** rule. This is the form that supports concave shapes,
///   self-intersections and holes: an outer ring and its inner rings are just
///   separate subpaths of the same path, and the parity of their cover turns
///   the interior ring into a hole.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub enum PathTopology {
    /// Open polylines, stroked only.
    #[default]
    Simple,

    /// A single closed convex ring, filled.
    Convex,

    /// One or more closed rings, filled with the even-odd rule.
    Complex,
}

/// PointElementConfig is a high-level representation of a point-based mark.
/// It acts as a bridge between the Layer logic and the specific geometric
/// draw calls of the RenderBackend.
#[derive(Clone, Copy, Debug)]
pub struct PointElementConfig {
    pub x: f64,
    pub y: f64,
    pub shape: PointShape,
    pub size: f64,
    pub fill: SingleColor,
    pub stroke: SingleColor,
    pub stroke_width: f64,
    pub opacity: f64,
}

pub struct CircleConfig {
    pub x: Precision,
    pub y: Precision,
    pub radius: Precision,
    pub fill: SingleColor,
    pub stroke: SingleColor,
    pub stroke_width: Precision,
    pub opacity: Precision,
}

pub struct RectConfig {
    pub x: Precision,
    pub y: Precision,
    pub width: Precision,
    pub height: Precision,
    pub fill: SingleColor,
    pub stroke: SingleColor,
    pub stroke_width: Precision,
    pub opacity: Precision,
}

pub struct LineConfig {
    pub x1: Precision,
    pub y1: Precision,
    pub x2: Precision,
    pub y2: Precision,
    pub color: SingleColor,
    pub width: Precision,
    pub opacity: Precision,
    pub dash: Vec<Precision>,
}

/// A vector path made of one or more independent subpaths.
///
/// A line has one open subpath; a polygon with holes has one closed subpath for
/// the outer ring and one for each hole. All subpaths share the same paint and
/// topology. Closed paths ([`PathTopology::Convex`] and
/// [`PathTopology::Complex`]) are closed independently per subpath; when filled,
/// they use the even-odd rule, so inner rings punch holes without relying on
/// their winding direction.
#[derive(Clone, Debug)]
pub struct PathConfig {
    /// Independent vertex sequences. Empty subpaths are ignored by backends.
    pub subpaths: Vec<Vec<(Precision, Precision)>>,
    pub fill: SingleColor,
    pub stroke: SingleColor,
    pub stroke_width: Precision,
    pub opacity: Precision,
    pub dash: Vec<Precision>,

    /// Lightweight structural hint to steer the backend's geometry pipeline.
    pub topology: PathTopology,
}

impl PathConfig {
    /// A single-subpath path with neutral paint defaults.
    pub fn new(points: Vec<(Precision, Precision)>) -> Self {
        Self::from_subpaths(vec![points])
    }

    /// A path with one or more subpaths and neutral paint defaults.
    pub const fn from_subpaths(subpaths: Vec<Vec<(Precision, Precision)>>) -> Self {
        Self {
            subpaths,
            fill: SingleColor::none(),
            stroke: SingleColor::none(),
            stroke_width: 0.0,
            opacity: 1.0,
            dash: vec![],
            topology: PathTopology::Simple,
        }
    }

    /// Sets the topology hint.
    pub const fn with_topology(mut self, topology: PathTopology) -> Self {
        self.topology = topology;
        self
    }

    /// Whether this path is a closed, filled region.
    pub const fn is_closed(&self) -> bool {
        matches!(self.topology, PathTopology::Convex | PathTopology::Complex)
    }

    /// Whether a stroking backend must append the first point of `points` to
    /// close that particular subpath.
    ///
    /// The SVG/PDF/raster backends close a path with `Z`/`close()`. The WGPU
    /// stroke shader extrudes one quad per consecutive pair and never wraps, so
    /// it must re-append the first point itself.
    #[cfg_attr(not(any(feature = "wgpu", test)), allow(dead_code))]
    pub(crate) fn stroke_closes(topology: PathTopology, points: &[(Precision, Precision)]) -> bool {
        matches!(topology, PathTopology::Convex | PathTopology::Complex)
            && points.len() >= 2
            && points.first() != points.last()
    }

    /// The outline dash pattern, flattened for a GPU shader.
    ///
    /// It follows the SVG `stroke-dasharray` rules: negative lengths are
    /// clamped to zero, an odd number of entries is repeated once so the
    /// pattern always alternates dash/gap, and a pattern with no positive entry
    /// is returned empty (a solid stroke). SVG and the raster backend apply
    /// these rules themselves, so this exists for the WGPU path shader.
    #[cfg_attr(not(any(feature = "wgpu", test)), allow(dead_code))]
    pub(crate) fn normalized_dash(&self) -> Vec<f32> {
        let values: Vec<f32> = self.dash.iter().map(|&d| d.max(0.0)).collect();
        if values.iter().all(|&v| v <= 0.0) {
            return Vec::new();
        }
        if values.len() % 2 == 1 {
            let mut doubled = values.clone();
            doubled.extend_from_slice(&values);
            return doubled;
        }
        values
    }
}

pub struct TextConfig {
    pub x: Precision,
    pub y: Precision,
    pub text: String,
    pub font_size: Precision,
    pub font_family: String,
    pub color: SingleColor,
    pub text_anchor: String,       // "start", "middle", "end"
    pub dominant_baseline: String, // "alphabetic", "hanging", "central", etc.
    pub font_weight: String,       // "normal", "bold", or numeric "400"
    pub font_style: String,        // "normal", "italic", or "oblique"
    pub opacity: Precision,
    pub angle: Precision, // Rotation angle in degrees
}

/// A colour ramp drawn as one rectangle, used by colour bars.
///
/// `stops` are `(offset, colour)` pairs with the offset in `0.0..=1.0`. The ramp
/// becomes a gradient *definition* in formats that have one (SVG `<linearGradient>`,
/// PDF shading); the rectangle merely references it. `field` names the data
/// field the ramp stands for, so the definition can be told apart in the output.
pub struct GradientRectConfig {
    pub x: Precision,
    pub y: Precision,
    pub width: Precision,
    pub height: Precision,
    pub stops: Vec<(Precision, SingleColor)>,
    pub is_vertical: bool,
    /// The data field whose scale this ramp visualises.
    pub field: String,
}

/// Abstract backend for rendering shapes.
/// Implementations could be SvgBackend (String), RasterBackend or WgpuBackend (GPU Buffers).
pub trait RenderBackend {
    /// Draws a circle with optional fill and stroke.
    fn draw_circle(&mut self, config: CircleConfig);

    /// Draws a rectangle with optional fill and stroke.
    fn draw_rect(&mut self, config: RectConfig);

    /// Draws a vector path.
    ///
    /// A path has one or more subpaths. Open paths ([`PathTopology::Simple`])
    /// are stroked; closed paths ([`PathTopology::Convex`] and
    /// [`PathTopology::Complex`]) are filled with the even-odd rule, so inner
    /// subpaths become holes.
    fn draw_path(&mut self, config: PathConfig);

    /// Renders text with specific alignment and weight.
    ///
    /// Fallible because backends that rasterise glyphs themselves (PNG, PDF)
    /// must resolve the font family and can fail on a stack that names no
    /// available font. Markup/GPU backends defer font selection to the renderer
    /// or host and normally return `Ok`.
    fn draw_text(&mut self, config: TextConfig) -> Result<(), ChartonError>;

    /// Draws a simple straight line between two points.
    ///
    /// Commonly used for rendering axis ticks or custom markers within guides.
    fn draw_line(&mut self, config: LineConfig);

    /// Draws a rectangle filled with a linear gradient.
    fn draw_gradient_rect(&mut self, config: GradientRectConfig);

    /// Opens a semantic scope around the primitives that follow.
    ///
    /// A backend turns the role and identity into whatever its output format can
    /// express: the SVG backend writes `<g class="charton-…">` (plus `data-*`,
    /// `aria-*` and `clip-path`), the raster and PDF backends apply the clip
    /// rectangle and ignore the rest, and the GPU backend only narrows its
    /// scissor rectangle. Every group must be closed by [`RenderBackend::end_group`].
    ///
    /// Groups nest; a group without a clip is a pure semantic scope and costs a
    /// backend nothing.
    fn begin_group(&mut self, group: &Group<'_>) {
        let _ = group;
    }

    /// Closes the innermost scope opened by [`RenderBackend::begin_group`].
    fn end_group(&mut self) {}

    /// Marks the primitives that follow as representing one data value.
    ///
    /// This is the per-element counterpart of [`RenderBackend::begin_group`]:
    /// the group says *which series* a run of primitives belongs to, this says
    /// *which observation* a single primitive stands for. The SVG backend writes
    /// it as `data-value` **on the primitive itself**, not on a wrapper, because
    /// a class or attribute on a parent group cannot override the `fill` or
    /// `font-size` presentation attributes the primitive already carries — a
    /// stylesheet must match the element it wants to restyle.
    ///
    /// Element scopes never nest; the identity applies to every primitive drawn
    /// until [`RenderBackend::end_element`].
    fn begin_element(&mut self, element: Element<'_>) {
        let _ = element;
    }

    /// Clears the value set by [`RenderBackend::begin_element`].
    fn end_element(&mut self) {}
}

/// `MarkRenderer` defines the contract for drawing geometric primitives.
///
/// Implementations of this trait (like PointRenderer or LineRenderer) focus
/// purely on the visual output using the provided coordinate tools.
pub trait MarkRenderer {
    /// Executes the drawing logic for this layer's marks.
    ///
    /// # Arguments
    /// * `backend`: The drawing engine (e.g., SVG, Canvas) that provides low-level primitives.
    /// * `context`: The localized `PanelContext` containing the coordinate system
    ///   and the physical area for this specific rendering pass.
    ///
    /// # Faceting Logic:
    /// In a faceted chart, this method may be called multiple times for a single layer,
    /// each time receiving a different `PanelContext` with a new `Rect` and potentially
    /// different `Scale` bounds.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError>;
}

/// `Layer` is the core trait for the layered grammar of graphics in Charton.
///
/// A layer represents a single component of a chart (e.g., a Scatter plot, a
/// Regression line, or a Bar set). It integrates metadata discovery
/// with the actual rendering logic.
///
/// The lifecycle of a Layer during the rendering pipeline is:
/// 1. **Statistics**: the mark's own summary (binning, stacking, density) is
///    applied to the dataset, per group. A faceted chart re-runs it per panel.
/// 2. **Discovery**: `LayeredChart` queries `get_data_bounds` to understand data ranges.
/// 3. **Training**: the engine aggregates those bounds into a scale. By default
///    one scale is shared by every layer and panel (*fixed*); a faceted chart
///    may instead train one per panel (`Free` / `FreeX` / `FreeY`).
/// 4. **Injection**: the engine calls `inject_resolved_scales` to "back-fill" the final scales into the layer.
/// 5. **Rendering**: the engine calls `render_marks` to produce the final geometry.
pub trait Layer: MarkRenderer + Send + Sync {
    // --- Metadata Discovery Phase ---

    /// The mark type this layer draws (`"line"`, `"point"`, `"bar"`, ...).
    ///
    /// Used to give the layer's marks a `charton-mark-<type>` class and a
    /// `data-mark` attribute, so a stylesheet can treat a line differently from
    /// a band without counting elements. Layers with no mark return `""`.
    fn mark_type(&self) -> &'static str {
        ""
    }

    /// Returns true if this layer requires coordinate axes (X/Y) to be drawn.
    /// Some layers, like annotations or background fills, might not need them.
    fn requires_axes(&self) -> bool;

    /// Returns the data field name mapped to a specific visual channel (e.g., "horsepower" -> Color).
    fn get_field(&self, channel: Channel) -> Option<String>;

    /// Returns a display label for a channel, when the encoding set one.
    ///
    /// Used for axis and legend titles; layers without labels return `None`.
    fn get_label(&self, channel: Channel) -> Option<String> {
        let _ = channel;
        None
    }

    /// Returns a reference to the underlying Dataset.
    ///
    /// For faceting this is the data from *before* the mark's statistic, because
    /// a statistic may drop or reshape the very columns the facet is keyed on.
    /// See [`Layer::facet_partition`] and [`Layer::subset_rows`].
    fn get_dataset(&self) -> &Dataset;

    /// Returns the preferred scale type (e.g., Linear, Log, Discrete) for a channel.
    fn get_scale(&self, channel: Channel) -> Option<Scale>;

    /// Returns the expansion rules (padding/margins) requested by this layer for a channel.
    fn get_expand(&self, channel: Channel) -> Option<Expansion>;

    /// Whether this layer asked for a reversed positional axis.
    ///
    /// Defaults to `false` so non-positional layers ignore it.
    fn get_reverse(&self, channel: Channel) -> bool {
        let _ = channel;
        false
    }

    /// Calculates the raw data boundaries (Min/Max for continuous, unique labels for discrete)
    /// contained within this specific layer's dataset.
    ///
    /// A discrete position axis may return [`ScaleDomain::DiscretePositions`]
    /// to report both its category labels and the numeric positions its marks
    /// actually occupy.
    ///
    /// This is the primary input for the "Training" phase, where scales are
    /// resolved — one shared scale for the chart, or one per panel under free
    /// scales.
    fn get_data_bounds(&self, channel: Channel) -> Result<ScaleDomain, ChartonError>;

    // --- State Resolution (The "Back-filling" Phase) ---

    /// Injects the resolved state (Coordinate system and Aesthetic mappings) into the layer.
    ///
    /// This ensures the layer has access to the final scales before rendering
    /// starts. The coordinate system is shared for a fixed chart and rebuilt per
    /// panel under free scales; the aesthetic mappings are always shared, so the
    /// legend stays unified.
    ///
    /// We use `&self` here; implementations typically use interior mutability (e.g., `OnceLock`
    /// or `RwLock`) to cache these values safely.
    ///
    /// # Arguments
    /// * `coord`: The coordinate system for this layer's panel (containing X and Y scales).
    /// * `aesthetics`: The shared global aesthetics (containing Color, Shape, and Size scales).
    fn inject_resolved_scales(
        &self,
        coord: Arc<dyn CoordinateTrait>,
        aesthetics: &GlobalAesthetics,
    );

    // --- Faceted Data Subsetting ---

    /// Partitions this layer's rows by the given facet `fields`.
    ///
    /// The result maps each combination of field values (in `fields` order) to
    /// the rows that carry it, so a panel can turn its `(field, value)` filter
    /// into a concrete set of rows via [`FacetPartition::row_indices`].
    ///
    /// Returns `Ok(None)` when the layer holds no facetable data (e.g. a pure
    /// annotation), in which case it is drawn unchanged on every panel.
    ///
    /// Implementations fail fast when a requested field is missing from their
    /// dataset, so a mistyped facet field is reported instead of silently
    /// leaving panels unfiltered.
    fn facet_partition(&self, fields: &[&str]) -> Result<Option<FacetPartition>, ChartonError> {
        let _ = fields;
        Ok(None)
    }

    /// Returns a copy of this layer containing only `rows`.
    ///
    /// This materializes one panel: the caller looks the panel's rows up in the
    /// partition returned by [`Layer::facet_partition`] and passes them here.
    ///
    /// Returns `Ok(None)` when the layer does not hold subsettable data.
    fn subset_rows(&self, rows: &[usize]) -> Result<Option<Arc<dyn Layer>>, ChartonError> {
        let _ = rows;
        Ok(None)
    }

    /// Returns a copy of this layer with its pre-statistic snapshot released.
    ///
    /// A statistic may keep the rows from before it ran so a faceted chart can
    /// re-run it per panel (see [`Layer::subset_rows`]). A chart with no facets
    /// never does that, so the snapshot is pure memory. The engine drops it from
    /// the mutable render path once it knows there are no facets.
    ///
    /// Layers that keep no snapshot may return `self` unchanged; they cannot be
    /// cloned through this trait, so `Chart<T>` is the only implementor and
    /// builds its copy directly.
    fn without_source_data(&self) -> Arc<dyn Layer>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(points: Vec<(Precision, Precision)>, topology: PathTopology) -> PathConfig {
        PathConfig {
            fill: SingleColor::none(),
            stroke: SingleColor::new("black"),
            stroke_width: 1.0,
            ..PathConfig::new(points)
        }
        .with_topology(topology)
    }

    /// The GPU dash pattern follows the SVG rules: an odd pattern is repeated
    /// to alternate, non-positive entries clamp to zero, and a pattern with no
    /// positive entry collapses to a solid stroke (empty).
    #[test]
    fn normalized_dash_follows_svg_rules() {
        assert_eq!(
            path(vec![], PathTopology::Simple).normalized_dash(),
            Vec::<f32>::new()
        );

        let mut even = path(vec![], PathTopology::Simple);
        even.dash = vec![6.0, 4.0];
        assert_eq!(even.normalized_dash(), vec![6.0, 4.0]);

        let mut odd = path(vec![], PathTopology::Simple);
        odd.dash = vec![6.0, 4.0, 2.0];
        assert_eq!(odd.normalized_dash(), vec![6.0, 4.0, 2.0, 6.0, 4.0, 2.0]);

        let mut zero = path(vec![], PathTopology::Simple);
        zero.dash = vec![0.0, 0.0];
        assert!(zero.normalized_dash().is_empty());
    }

    /// A closed (`Complex` or `Convex`) path must have its first point
    /// re-appended for a segment-extruding backend, an open path must not, and
    /// an already-closed vertex loop must not get a duplicate.
    #[test]
    fn closed_path_stroke_closes_each_loop() {
        let open = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)];
        assert!(PathConfig::stroke_closes(PathTopology::Complex, &open));
        assert!(PathConfig::stroke_closes(PathTopology::Convex, &open));
        assert!(!PathConfig::stroke_closes(PathTopology::Simple, &open));

        let closed = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0), (0.0, 0.0)];
        assert!(!PathConfig::stroke_closes(PathTopology::Complex, &closed));

        assert!(!PathConfig::stroke_closes(
            PathTopology::Complex,
            &[(0.0, 0.0)]
        ));

        // Every ring is closed independently.
        let mut config = PathConfig::from_subpaths(vec![
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 0.0)],
            vec![(2.0, 2.0), (4.0, 2.0), (4.0, 4.0)],
        ]);
        config.topology = PathTopology::Complex;
        assert!(PathConfig::stroke_closes(
            PathTopology::Complex,
            &config.subpaths[1]
        ));
    }
}
