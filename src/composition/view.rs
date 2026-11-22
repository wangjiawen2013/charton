//! What a cell can hold, and what the page asks it.
//!
//! A page never draws a chart itself. It gives each cell a box and asks two
//! questions — one before drawing, one while drawing:
//!
//! ```text
//!   measure(box) -> where does your data land?   what legend can you hand over?
//!                   what scale did you resolve?
//!   draw(backend, plan) -> draw yourself in this box, adopting what I decided
//! ```
//!
//! There are exactly two things that can answer: a finished chart
//! ([`LayeredChart`]) and another page ([`Composition`]). Both implement
//! [`CellView`], and [`View`] is the handle the public API takes. That is the whole
//! abstraction, and it buys three properties:
//!
//! * **The page adds nothing to a chart.** It never inspects marks, scales or
//!   guides; it moves a picture it did not make and writes down the decisions it
//!   took (a pinned rectangle, a shared scale, a legend drawn elsewhere).
//! * **A page is a view.** Nesting needs no special case: a cell holding a page is
//!   measured, aligned and drawn exactly like a cell holding a chart.
//! * **The questions are asked once.** Every answer comes out of the one scene the
//!   chart is going to draw, so measuring and drawing cannot disagree.
//!
//! # Coordinates
//!
//! A view answers in its **own** coordinates, origin at the top-left of the box it
//! was given, because that is how it draws. Putting it on the page is the page's
//! job, and it does it by wrapping the drawing target in an
//! [offset backend](crate::render::offset) — the picture is moved, never re-laid
//! out. A cell therefore never knows where on the page it is, and a chart never
//! knows it is in a composition at all.
//!
//! # What a nested page answers
//!
//! A page has no single data rectangle — it has one per column and per row. When it
//! stands in a cell, it answers for its **master cell**: the largest one by canvas,
//! which in a marginal layout is the main view and never a strip. That rectangle is
//! what the parent aligns and pins, and pinning it makes the inner page re-solve
//! around it, so alignment and nesting compose instead of excluding each other.

use std::sync::Arc;

use crate::coordinate::Rect;
use crate::core::composite::{LayeredChart, LegendMaterial, ResolvedScene};
use crate::core::layer::RenderBackend;
use crate::encode::Channel;
use crate::error::ChartonError;
use crate::render::offset::OffsetBackend;
use crate::scale::ScaleDomain;

use super::{Composition, SharedScale, TrackSize};

/// One direction across the **page**.
///
/// Physical, not logical: `coord_flip` swaps a view's channels, never the page's
/// directions. Everything a composition reasons about — a column, a row, a
/// shared axis — is physical, so this is the only direction type the layout uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    /// Both directions, in the order the page reads them.
    pub(crate) const ALL: [Self; 2] = [Self::Horizontal, Self::Vertical];

    /// The slot this direction occupies in a per-direction array.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Horizontal => 0,
            Self::Vertical => 1,
        }
    }
}

/// What a view reports once it has been measured in a box.
///
/// Everything a composition decides about a cell comes from here: which
/// rectangle the track shares, whether the shared top clears the title, which
/// cell owns the page legend, and which scale a neighbour may adopt.
pub(crate) struct ViewMetrics {
    /// The data rectangle, in the view's own coordinates. The only part a
    /// neighbouring view may share or pin.
    pub(crate) panel: Rect,
    /// The bottom edge of the view's own title band (0 when it has none).
    pub(crate) title_bottom: f64,
    /// The guides this view would draw, packaged so a container can draw them
    /// somewhere else. `None` when it has nothing to hand over.
    pub(crate) legend: Option<LegendMaterial>,
    /// The positional scale this view resolved for each page direction.
    pub(crate) scales: [Option<SharedScale>; 2],
    /// Whether the caller must leave that scale alone, per direction.
    ///
    /// A domain the caller pinned by hand is a promise, never something a layout
    /// may overwrite.
    pub(crate) pinned: [bool; 2],
}

/// Everything the page decided about one cell, handed to it when it draws.
pub(crate) struct CellPlan {
    /// The data rectangle the cell must adopt (`bounds: flush`).
    pub(crate) panel: Option<Rect>,
    /// Whether the cell draws its own legend.
    pub(crate) show_legend: bool,
    /// A scale a neighbour resolved, per page direction.
    ///
    /// Applied when the cell draws, which is safe because its panel is pinned at
    /// the same time: a wider domain can move tick labels, but not the rectangle
    /// they are drawn around, so the page does not have to solve twice.
    pub(crate) scales: [Option<SharedScale>; 2],
}

impl Default for CellPlan {
    /// Nothing imposed: draw everything the way the view asked to be drawn.
    fn default() -> Self {
        Self {
            panel: None,
            show_legend: true,
            scales: [None, None],
        }
    }
}

impl CellPlan {
    /// The scale imposed on `axis`, if any.
    pub(crate) const fn scale(&self, axis: Axis) -> Option<&SharedScale> {
        self.scales[axis.index()].as_ref()
    }
}

/// A child a composition can place, measure and draw.
///
/// Implemented by [`LayeredChart`] (a finished chart) and [`Composition`] (a
/// nested page). Closed on purpose — see [`View`].
pub(crate) trait CellView {
    /// Measures the view in a `width` × `height` box.
    ///
    /// `show_legend` says whether the view will draw its own legend, which
    /// decides whether its band is reserved: a view whose legend is drawn
    /// elsewhere must not leave a hole where it used to be.
    fn measure(
        &self,
        width: f64,
        height: f64,
        show_legend: bool,
    ) -> Result<ViewMetrics, ChartonError>;

    /// How much room this view asks for when the track it sits in is free.
    fn requested_width(&self) -> Option<TrackSize> {
        None
    }

    /// See [`CellView::requested_width`].
    fn requested_height(&self) -> Option<TrackSize> {
        None
    }

    /// Draws the view into its own `(0, 0)`, honouring the page's decisions.
    fn draw(
        &self,
        backend: &mut dyn RenderBackend,
        width: f64,
        height: f64,
        plan: &CellPlan,
    ) -> Result<(), ChartonError>;
}

// ----------------------------------------------------------------------------
// A chart, as a cell
// ----------------------------------------------------------------------------

impl CellView for LayeredChart {
    fn measure(
        &self,
        width: f64,
        height: f64,
        show_legend: bool,
    ) -> Result<ViewMetrics, ChartonError> {
        // One scene resolution answers all four questions, so a page never pays
        // for measuring what it is not going to use.
        let sized = self.for_cell(pixels(width), pixels(height), show_legend);
        let scene = sized.resolve_scene()?;
        let panel = scene.panel;
        let title_bottom = scene.title_rect.map_or(0.0, |rect| rect.y + rect.height);
        let legend = legend_of(&sized, scene);
        Ok(ViewMetrics {
            panel,
            title_bottom,
            legend,
            scales: [
                resolved_scale(self, Axis::Horizontal),
                resolved_scale(self, Axis::Vertical),
            ],
            pinned: [
                domain_pinned(self, Axis::Horizontal),
                domain_pinned(self, Axis::Vertical),
            ],
        })
    }

    fn requested_width(&self) -> Option<TrackSize> {
        self.width_spec
    }

    fn requested_height(&self) -> Option<TrackSize> {
        self.height_spec
    }

    fn draw(
        &self,
        backend: &mut dyn RenderBackend,
        width: f64,
        height: f64,
        plan: &CellPlan,
    ) -> Result<(), ChartonError> {
        let mut chart = self.for_cell(pixels(width), pixels(height), plan.show_legend);
        chart.panel_override = plan.panel;

        // A shared scale belongs to a *direction* on the page, so it is written
        // back to whichever channel governs that direction: after `coord_flip`
        // the horizontal direction is the y channel.
        for axis in Axis::ALL {
            let Some(shared) = plan.scale(axis) else {
                continue;
            };
            match channel_of(&chart, axis) {
                Channel::X => {
                    chart.x_domain = Some(shared.domain.clone());
                    chart.x_expand = Some(shared.expand);
                }
                Channel::Y => {
                    chart.y_domain = Some(shared.domain.clone());
                    chart.y_expand = Some(shared.expand);
                }
                _ => {}
            }
        }

        // The cell draws in its own `(0, 0)`; the zero offset keeps the trait
        // object boundary without teaching every renderer about `dyn`.
        chart.render(&mut OffsetBackend::new(backend, 0.0, 0.0))
    }
}

/// The guides a chart would draw, packaged for another container.
///
/// Taken from the very scene the renderer uses, so a legend drawn elsewhere
/// lists the entries this chart would have listed.
fn legend_of(chart: &LayeredChart, scene: ResolvedScene) -> Option<LegendMaterial> {
    if !chart.theme.show_legend || scene.guides.is_empty() {
        return None;
    }
    Some(LegendMaterial {
        guides: scene.guides,
        aesthetics: scene.aesthetics,
        coord: scene.coord,
        panel: scene.panel,
        theme: chart.theme.clone(),
    })
}

/// The channel that governs `axis` on the page.
///
/// `coord_flip` swaps them, so everything that reasons about the *page* (a
/// column, a row, a shared scale) goes through this instead of assuming x is
/// horizontal.
pub(super) const fn channel_of(chart: &LayeredChart, axis: Axis) -> Channel {
    match (chart.flipped, axis) {
        (false, Axis::Horizontal) | (true, Axis::Vertical) => Channel::X,
        _ => Channel::Y,
    }
}

/// The positional scale a chart trained for one page direction.
fn resolved_scale(chart: &LayeredChart, axis: Axis) -> Option<SharedScale> {
    let spec = chart
        .resolve_scale_spec(channel_of(chart, axis))
        .ok()
        .flatten()?;
    Some(SharedScale {
        domain: spec.domain,
        expand: spec.expand,
    })
}

/// Whether the caller pinned the domain that governs `axis`.
const fn domain_pinned(chart: &LayeredChart, axis: Axis) -> bool {
    match channel_of(chart, axis) {
        Channel::X => chart.x_domain.is_some(),
        Channel::Y => chart.y_domain.is_some(),
        _ => true,
    }
}

/// Whether two scales are of the same *kind*, so one may stand in for the other.
///
/// Only numeric domains are shared. A discrete domain is a list of labels, and
/// replacing it would rename the categories rather than widen a range; the kind
/// of scale (linear against log) belongs to the encoding, so a cell that
/// disagrees keeps its own scale instead of silently plotting one against the
/// other.
pub(super) const fn same_kind(own: &SharedScale, other: &SharedScale) -> bool {
    matches!(
        (&own.domain, &other.domain),
        (ScaleDomain::Continuous(..), ScaleDomain::Continuous(..))
            | (ScaleDomain::Temporal(..), ScaleDomain::Temporal(..))
    )
}

/// A box size as the pixel counts the chart layout works in.
const fn pixels(size: f64) -> u32 {
    size.round().max(1.0) as u32
}

// ----------------------------------------------------------------------------
// The public handle
// ----------------------------------------------------------------------------

/// A view that can be placed in a cell: a finished chart, or a nested
/// composition.
///
/// The set is closed — a chart or a composition, nothing else — which is why
/// composition needs no `into_view()` ceremony at the call site:
///
/// ```rust,ignore
/// Composition::grid(2, 2)
///     .with_cell(0, 0, top_hist)?     // a LayeredChart
///     .with_cell(1, 0, scatter)?      // a LayeredChart
///     .with_cell(1, 1, another_page)? // a nested Composition
/// ```
///
/// It is a handle, not a builder: the wrapped view is shared by reference
/// count, so cloning a composition (or its cells) never clones a chart's data.
/// A view handle is shared by reference count, so cloning a page — or moving one of
/// its cells — never clones a chart's data.
///
/// The count is atomic even though a chart is not `Send` today: the crate renders in
/// parallel behind a feature flag, and an atomic counter keeps a future `Send` chart
/// from having to change this type.
#[derive(Clone)]
pub struct View(pub(crate) Arc<dyn CellView>);

#[allow(clippy::arc_with_non_send_sync)]
impl From<LayeredChart> for View {
    fn from(chart: LayeredChart) -> Self {
        Self(Arc::new(chart))
    }
}

/// A single-layer chart — what `chart!(..)?.mark_point()?.encode(..)?` returns —
/// is a view too, so a cell can be given the pipeline directly instead of a
/// `LayeredChart`.
impl<T: crate::mark::Mark + 'static> From<crate::chart::Chart<T>> for View
where
    crate::chart::Chart<T>: crate::core::layer::Layer + Clone,
{
    fn from(chart: crate::chart::Chart<T>) -> Self {
        Self::from(LayeredChart::from(chart))
    }
}

#[allow(clippy::arc_with_non_send_sync)]
impl From<Composition> for View {
    fn from(composition: Composition) -> Self {
        Self(Arc::new(composition))
    }
}

impl std::fmt::Debug for View {
    /// The concrete kind, without trying to describe a whole chart: a view is
    /// either one chart or one page, and which of the two is what a reader of a
    /// layout needs to know.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("View(…)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scale(domain: ScaleDomain) -> SharedScale {
        SharedScale {
            domain,
            expand: Default::default(),
        }
    }

    #[test]
    fn only_scales_of_the_same_kind_are_interchangeable() {
        let numeric = scale(ScaleDomain::Continuous(0.0, 10.0));
        let wider = scale(ScaleDomain::Continuous(0.0, 100.0));
        let labels = scale(ScaleDomain::Discrete(vec!["a".into(), "b".into()]));
        assert!(
            same_kind(&numeric, &wider),
            "a numeric domain may be replaced by a wider one"
        );
        assert!(
            !same_kind(&numeric, &labels),
            "a list of labels would be renamed, not widened"
        );
    }

    #[test]
    fn the_two_directions_are_distinct_slots() {
        assert_eq!(Axis::Horizontal.index(), 0);
        assert_eq!(Axis::Vertical.index(), 1);
        assert_eq!(Axis::ALL, [Axis::Horizontal, Axis::Vertical]);
    }
}
