//! Several charts on one page.
//!
//! # What a composition decides
//!
//! A chart is a complete picture, laid out in its own canvas. Composition is the
//! step above it: it takes finished views and decides the three things a chart
//! cannot decide for itself — each one a rectangle, what is shared between those
//! rectangles, and which furniture is drawn around them.
//!
//! * **which rectangle each view gets** — a grid of columns and rows
//!   ([`TrackSize`], [`Composition::with_cell`]);
//! * **which data rectangle they share** — so that a marginal strip lines up with
//!   the chart it belongs to ([`Align`], [`Resolve`]);
//! * **where their axes and legends are drawn** — a view may hide its own, and the
//!   page may draw one legend on its edge ([`Composition::with_legend_position`]).
//!
//! It adds no statistic and no mark. A scatter with marginal histograms is three
//! ordinary charts (`mark_point` and two `mark_hist`) whose layout is decided
//! here — or, since that layout is the one everybody writes, one call to
//! [`Composition::marginal`].
//!
//! # The model
//!
//! A page is a **grid of tracks** — one slot per column (a width) and per row (a
//! height) — and a view sits on the intersection of one of each, so the layout
//! reads like a table:
//!
//! ```text
//!             column 0                   column 1
//!           +------------------------+-----------+
//!  row 0    |  cell(0, 0)            |  (empty)  |   Fixed
//!           +------------------------+-----------+
//!  row 1    |  cell(1, 0)            | cell(1,1) |   Fill
//!           +------------------------+-----------+
//! ```
//!
//! Row 0 is the top row and column 0 the left column, so indices grow downwards and
//! rightwards. An intersection with no cell draws nothing — that is how the blank
//! corner above a right-hand strip is expressed, so no `None`/spacer placeholder is
//! needed. Tracks are the same idea the [facet grid](crate::facets) uses, for the
//! same reason: a column exists once, so every cell in it starts at the same x.
//!
//! # How a cell is drawn
//!
//! Each cell holds a view — a chart, or another page ([`View`]) — that knows only
//! its own `(0, 0)`. Three steps put it in place: the view is sized to the cell, so
//! its own layout solves inside that box; the rectangle its column or row shares is
//! written into it, so its axes, title and legend stack around *that*; and the
//! finished picture is drawn through an *offset backend*, a drawing target that
//! adds the cell's corner to every coordinate it receives. Nothing about the chart
//! changes on the way — the same marks, the same tick density, the same legend, at
//! another origin.
//!
//! # What this is not
//!
//! * **Not a facet.** A facet slices *one* grammar's data into equal panels with
//!   one shared axis grid ([`crate::facets`]); a composition arranges *whole views*
//!   that may differ in data, mark and size. Same picture over subsets of the rows
//!   → facet; different pictures on one page → composition.
//! * **Not a mark.** Nothing here knows what a histogram is, so a marginal strip is
//!   the ordinary `mark_hist` in a thin cell rather than a new chart type.
//!
//! # Example: a scatter with marginal histograms
//!
//! ```rust,ignore
//! let scatter = chart!(&iris)?
//!     .mark_point()?
//!     .encode((alt::x("petal_length"), alt::y("sepal_width"), alt::color("species")))?;
//!
//! // The same data, summarised along one axis each.
//! let top = chart!(&iris)?
//!     .mark_hist()?
//!     .encode((alt::x("petal_length").with_bins(18), alt::y("count")))?
//!     .with_show_x_axis(false);                        // its x scale is the scatter's
//!
//! let right = chart!(&iris)?
//!     .mark_hist()?
//!     .encode((alt::x("sepal_width").with_bins(18), alt::y("count")))?
//!     .coord_flip()                                    // count along the bottom
//!     .with_show_y_axis(false);                        // its y scale is the scatter's
//!
//! Composition::marginal(
//!     scatter,
//!     Marginals::new().with_top(top).with_right(right),
//! )
//! .with_spacing(6.0)
//! .with_legend_position(LegendPosition::Right)     // the legend, outside everything
//! .save("marginal.svg")?;
//! ```
//!
//! [`marginal`](Composition::marginal) is that layout, named — not a special case
//! in the engine: it builds a grid whose strip tracks are fixed, whose cells align
//! by data rectangle, with the corners left empty. Write the grid by hand with
//! [`grid`](Composition::grid) / [`with_cell`](Composition::with_cell) when the
//! layout is not a marginal one.
//!
//! # Where the rest is
//!
//! The book chapter *Composition: Several Views on One Page* carries the reasoning
//! and the walkthroughs: the seven **design principles** this module derives from, a
//! seven-step **tutorial** that builds the figure above one setting at a time, the
//! track arithmetic, what each [`Align`] shares and why the rectangle is an
//! intersection, how a nested page is measured through, and six complete recipes.

use crate::Precision;
use crate::coordinate::Rect;
use crate::core::composite::LegendMaterial;
use crate::core::context::{ChartSpec, PanelContext};
use crate::core::flow::Direction;
use crate::core::guide::LegendPosition;
use crate::core::layer::{RectConfig, RenderBackend, TextConfig};
use crate::core::semantic::{Group, Role};
use crate::error::ChartonError;
use crate::render::offset::OffsetBackend;
use crate::render::svg::{SvgDocument, SvgOptions};
use crate::scale::{Expansion, ScaleDomain};
use crate::theme::{Theme, TitleAnchor};

mod view;

pub use view::View;
use view::{Axis, CellPlan, CellView, ViewMetrics, same_kind};

#[cfg(feature = "pdf")]
use crate::render::pdf::{PdfDocument, PdfOptions};

/// How much room one **track** gets along its axis, in pixels.
///
/// A track is one slot of the grid: a column (measured in width) or a row
/// (measured in height). Every track is one of two kinds — [`Fixed`](Self::Fixed)
/// keeps exactly its pixels, [`Fill`](Self::Fill) takes a weighted share of what
/// the fixed tracks left over — which is the CSS-grid `px` / `1fr` pair, and what
/// lets a 60px strip stay 60px while the view beside it takes the rest.
///
/// The same value describes a *view's request* (`with_width` / `with_height`): the
/// track containing that view then takes the requested size, unless an explicit
/// [`with_column_width`](Composition::with_column_width) /
/// [`with_row_height`](Composition::with_row_height) says otherwise.
///
/// | You want | Write |
/// | --- | --- |
/// | a thin strip that never grows | `TrackSize::fixed(60.0)` (or just `60.0`) |
/// | "take whatever is left" | `TrackSize::fill(1.0)` (the default) |
/// | two-thirds against one-third | `TrackSize::fill(2.0)` + `TrackSize::fill(1.0)` |
/// | an exact split, no stretching | `TrackSize::fixed(300.0)` + `TrackSize::fixed(200.0)` |
///
/// The arithmetic — what is left after the gaps and the fixed tracks, worked
/// through on a 500px page — is in the book chapter *Composition*.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrackSize {
    /// Exactly this many pixels, whatever the page size does.
    ///
    /// This is the right choice for something that must stay thin — a 60px
    /// marginal histogram, a colour strip, a rule. Fixed tracks are paid for
    /// *first*, so they never shrink because a `Fill` neighbour grew.
    ///
    /// A negative value is treated as `0.0`. If the fixed tracks together ask
    /// for more than the page has, the `Fill` tracks collapse to `0` and the
    /// fixed ones overflow; the composition never silently resizes your choice.
    Fixed(f64),

    /// A share of the room the fixed tracks left over, given as a **weight**.
    ///
    /// The weight is relative, not a percentage: `Fill(1.0)` and `Fill(3.0)`
    /// in two columns split the leftover room 1:3. `Fill(1.0)` is the default,
    /// which reads "take an equal share of whatever is left".
    ///
    /// This is the CSS-grid `1fr` idea, and it is what makes a page respond to
    /// its canvas: the strips keep their pixels, the main view takes the rest.
    /// A weight of `0.0` (or less) claims nothing — a mistyped weight shrinks
    /// the track instead of eating the page.
    Fill(f64),
}

impl TrackSize {
    /// Exactly `px` pixels. See [`TrackSize::Fixed`].
    ///
    /// ```
    /// use charton::prelude::*;
    /// assert_eq!(TrackSize::fixed(60.0), TrackSize::Fixed(60.0));
    /// ```
    pub const fn fixed(px: f64) -> Self {
        Self::Fixed(px)
    }

    /// A share of the leftover room, proportional to `weight`. See
    /// [`TrackSize::Fill`].
    ///
    /// ```
    /// use charton::prelude::*;
    /// assert_eq!(TrackSize::fill(1.0), TrackSize::default());
    /// ```
    pub const fn fill(weight: f64) -> Self {
        Self::Fill(weight)
    }

    /// The extent this track claims, given the budget left for `Fill` tracks and
    /// the total weight of all of them. A `Fill` track with no weight claims
    /// nothing, so a mistyped `Fill(0.0)` shrinks instead of eating the page.
    fn resolve(&self, fill_budget: f64, fill_weight: f64) -> f64 {
        match *self {
            Self::Fixed(px) => px.max(0.0),
            Self::Fill(weight) => {
                if fill_weight > 0.0 {
                    fill_budget * weight.max(0.0) / fill_weight
                } else {
                    0.0
                }
            }
        }
    }
}

impl Default for TrackSize {
    /// [`TrackSize::fill(1.0)`](TrackSize::fill): take an equal share of what is left.
    fn default() -> Self {
        Self::Fill(1.0)
    }
}

/// A bare number is a **fixed** pixel size, so `with_column_width(1, 60.0)`
/// reads like any other length in the library.
impl From<f64> for TrackSize {
    fn from(px: f64) -> Self {
        Self::Fixed(px)
    }
}

/// A bare integer is a **fixed** pixel size. See the `f64` impl above.
impl From<u32> for TrackSize {
    fn from(px: u32) -> Self {
        Self::Fixed(px as f64)
    }
}

/// How the cells of a page relate to each other — and therefore what they share.
///
/// The three values say how *much* of the layout is shared. The two latter ones
/// combine Vega-Lite's `bounds` and `align`, which always travel together:
///
/// | charton | Vega-Lite | What is shared |
/// | --- | --- | --- |
/// | [`None`](Self::None) (default) | `bounds: "full"` | nothing — each cell is placed by its canvas, axis furniture and all |
/// | [`Each`](Self::Each) | `bounds: "flush"`, `align: "each"` | one data rectangle **per track**: a column shares one x rectangle, a row one y rectangle |
/// | [`All`](Self::All) | `bounds: "flush"`, `align: "all"` | one data rectangle for the **whole page** |
///
/// There is deliberately no fourth combination. "Flush bounds with no alignment"
/// is exactly [`None`](Self::None): a cell that keeps its own rectangle is placed
/// by its canvas whatever the bounds are called, so offering both would be two
/// names for one behaviour — and a setting that silently does nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// Nothing is shared: every cell keeps the layout it computed for itself and
    /// is placed by its **canvas**, axis labels, title and legend included.
    ///
    /// Ordinary page layout, and the right choice for a dashboard where nothing is
    /// expected to line up across cells.
    #[default]
    None,
    /// The cells of a track share one **data rectangle** (Vega-Lite's
    /// `bounds: "flush"` with `align: "each"`).
    ///
    /// A column shares one x rectangle and a row one y rectangle — what makes a
    /// marginal strip land on the same x as the chart it annotates. The rectangle
    /// is the **intersection** of the cells' own panels: the latest start and the
    /// earliest end.
    ///
    /// Intersecting, rather than uniting, is deliberate: it is the only choice
    /// where no cell's furniture is pushed outside the box it already owns, so
    /// alignment can never paint one cell over its neighbour.
    Each,
    /// Every cell shares **one** rectangle for the whole page (Vega-Lite's
    /// `bounds: "flush"` with `align: "all"`).
    ///
    /// All panels end up the same size on the same lines — the layout a dashboard
    /// of comparable charts wants. The rectangle is the intersection of every
    /// single-track cell's own panel, so a page with one narrow track pulls all the
    /// other panels in to that width.
    All,
}

/// Whether the cells that share a data rectangle also share the scale inside it.
///
/// Vega-Lite's `resolve: {scale: …}`, and only consulted when there *is* a shared
/// rectangle ([`Align::Each`] or [`Align::All`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Resolve {
    /// Adopt the domain of the group's largest cell (the default).
    ///
    /// Lining up rectangles while showing a different range in each would be worse
    /// than not aligning at all: the same value would land on two different x,
    /// which defeats the reason to align.
    #[default]
    Shared,
    /// Every cell keeps the domain it resolved for itself, so the same pixels mean
    /// different values in different cells.
    ///
    /// Right for cells that really do plot unrelated quantities; wrong for a
    /// marginal strip, whose whole job is to agree with the view beside it.
    Independent,
}

/// The views of a [`hconcat`](Composition::hconcat) / [`vconcat`](Composition::vconcat).
///
/// Implemented for a list of one kind of view (`Vec<LayeredChart>`, an array) and
/// for tuples of up to eight, where each element may be a different kind. Tuples
/// are what a nested layout needs: a `vconcat` of a strip and a page cannot be
/// one array, because the two are different types.
pub trait IntoViews {
    /// The views, in the order they were given.
    fn into_views(self) -> Vec<View>;
}

impl<T: Into<View>> IntoViews for Vec<T> {
    fn into_views(self) -> Vec<View> {
        self.into_iter().map(Into::into).collect()
    }
}

impl<T: Into<View>, const N: usize> IntoViews for [T; N] {
    fn into_views(self) -> Vec<View> {
        self.into_iter().map(Into::into).collect()
    }
}

/// `(a, b, …)` — one view per element, each its own kind.
macro_rules! views_of_tuple {
    ($($view:ident),+) => {
        #[allow(non_snake_case)]
        impl<$($view: Into<View>),+> IntoViews for ($($view,)+) {
            fn into_views(self) -> Vec<View> {
                let ($($view,)+) = self;
                vec![$($view.into(),)+]
            }
        }
    };
}

views_of_tuple!(A);
views_of_tuple!(A, B);
views_of_tuple!(A, B, C);
views_of_tuple!(A, B, C, D);
views_of_tuple!(A, B, C, D, E);
views_of_tuple!(A, B, C, D, E, F);
views_of_tuple!(A, B, C, D, E, F, G);
views_of_tuple!(A, B, C, D, E, F, G, H);

/// The strips around a main view, for [`Composition::marginal`].
///
/// A marginal plot is one main view plus up to four thin strips, each summarising
/// one of its axes. This type names *which* strips exist and how thick they are;
/// [`Composition::marginal`] turns that into the grid, the track sizes and the
/// alignment, so none of those settings can be forgotten or applied to the wrong
/// edge.
///
/// ```rust,ignore
/// Composition::marginal(scatter, Marginals::new()
///     .with_top(top_hist)
///     .with_right(right_hist)
///     .with_strip_size(60.0))
/// ```
///
/// Every strip is a full view of its own: it is placed, aligned and scaled like
/// any other cell, and what it draws — its axis, its legend, its grid — stays the
/// caller's decision. That is the one thing this type deliberately does *not*
/// guess.
#[derive(Clone, Default)]
pub struct Marginals {
    top: Option<View>,
    bottom: Option<View>,
    left: Option<View>,
    right: Option<View>,
    strip: f64,
}

impl Marginals {
    /// No strips yet, and a 60px strip size — the thickness a marginal histogram
    /// usually gets.
    pub const fn new() -> Self {
        Self {
            top: None,
            bottom: None,
            left: None,
            right: None,
            strip: 60.0,
        }
    }

    /// The strip above the main view: it summarises the **x** axis, so it shares
    /// the main view's horizontal data rectangle and horizontal scale.
    pub fn with_top(mut self, view: impl Into<View>) -> Self {
        self.top = Some(view.into());
        self
    }

    /// The strip below the main view. See [`with_top`](Self::with_top).
    pub fn with_bottom(mut self, view: impl Into<View>) -> Self {
        self.bottom = Some(view.into());
        self
    }

    /// The strip left of the main view: it summarises the **y** axis, so it
    /// shares the main view's vertical data rectangle and vertical scale.
    pub fn with_left(mut self, view: impl Into<View>) -> Self {
        self.left = Some(view.into());
        self
    }

    /// The strip right of the main view. See [`with_left`](Self::with_left).
    pub fn with_right(mut self, view: impl Into<View>) -> Self {
        self.right = Some(view.into());
        self
    }

    /// How thick every strip is, in pixels (60 by default).
    ///
    /// One number for all four edges: a figure with a 60px top strip and a 50px
    /// right strip is not a marginal plot, it is a dashboard — build that one
    /// with [`grid`](Composition::grid) directly.
    pub fn with_strip_size(mut self, px: f64) -> Self {
        self.strip = if px > 0.0 { px } else { 0.0 };
        self
    }
}

/// One cell of the track grid.
#[derive(Clone)]
struct Cell {
    row: usize,
    col: usize,
    row_span: usize,
    col_span: usize,
    view: View,
}

/// A page made of several views.
///
/// Built by a constructor that names its layout — [`grid`](Self::grid),
/// [`hconcat`](Self::hconcat), [`vconcat`](Self::vconcat),
/// [`marginal`](Self::marginal) — then filled with
/// [`with_cell`](Self::with_cell), sized with [`with_column_width`](Self::with_column_width)
/// / [`with_row_height`](Self::with_row_height), and told what to share with
/// [`with_align`](Self::with_align) and [`with_resolve`](Self::with_resolve).
///
/// See the [module documentation](self) for the model, and the book chapter
/// *Composition* for the track arithmetic, the alignment rules and the recipes.
#[derive(Clone)]
pub struct Composition {
    width: u32,
    height: u32,
    /// One entry per column track: its width.
    columns: Vec<TrackSize>,
    /// One entry per row track: its height.
    rows: Vec<TrackSize>,
    /// The views, each on the intersection of one row track and one column track.
    cells: Vec<Cell>,
    /// Gap between neighbouring columns, in pixels.
    column_spacing: f64,
    /// Gap between neighbouring rows, in pixels.
    row_spacing: f64,
    /// What the cells share.
    align: Align,
    /// Whether the scale follows the shared rectangle.
    resolve: Resolve,
    /// The cell a hoisted legend is taken from, when the caller named it.
    ///
    /// `None` lets the page pick the largest cell that has a legend; see
    /// [`with_legend_from`](Self::with_legend_from).
    legend_from: Option<(usize, usize)>,
    /// The page's stylesheet. Its `legend_position` is also the page's decision
    /// about hoisting: `None` keeps every legend in its own cell.
    theme: Theme,
    title: Option<String>,
    description: Option<String>,
    /// Pixel density of raster output, as on a single chart — the same figure
    /// leaves the library at the same resolution whether it is one chart or
    /// several.
    scale_factor: f32,
}

impl Composition {
    /// An empty grid of `rows` × `columns` tracks, every track
    /// [`TrackSize::fill(1.0)`](TrackSize::fill).
    ///
    /// The arguments are `(rows, columns)` — rows first, like
    /// `with_cell(row, col)`. Row 0 is the **top** row and column 0 the **left**
    /// column, so indices grow downwards and rightwards, like a table.
    ///
    /// The grid is the *matrix of tracks*, not of views: an intersection with no
    /// [`with_cell`](Self::with_cell) stays empty and draws nothing. That is how a
    /// blank corner is expressed — a 2 × 2 grid with three views is the marginal
    /// layout with its top-right corner left open.
    ///
    /// There is deliberately no `Composition::new()`: a composition *is* its
    /// layout, so every constructor names one ([`grid`](Self::grid),
    /// [`hconcat`](Self::hconcat), [`vconcat`](Self::vconcat),
    /// [`marginal`](Self::marginal)) instead of one of them being privileged, and
    /// the parameter order is always the one the method name describes.
    ///
    /// The canvas defaults to 500 × 400, the same as a single chart; set it with
    /// [`with_size`](Self::with_size).
    pub fn grid(rows: usize, columns: usize) -> Self {
        Self {
            width: 500,
            height: 400,
            columns: vec![TrackSize::default(); columns.max(1)],
            rows: vec![TrackSize::default(); rows.max(1)],
            cells: Vec::new(),
            column_spacing: 4.0,
            row_spacing: 4.0,
            align: Align::default(),
            resolve: Resolve::default(),
            legend_from: None,
            // The page theme's `legend_position` is the hoisting decision: `None`
            // means every cell draws its own legend, an edge means the page draws
            // one there. A single chart defaults to `Right`, a page to `None`.
            theme: Theme::default().with_legend_position(LegendPosition::None),
            title: None,
            description: None,
            scale_factor: 1.0,
        }
    }

    /// A single row of views, left to right.
    ///
    /// Each view keeps its own canvas; the row is as tall as the tallest one.
    /// A view can ask for its own width and height (`with_width` /
    /// `with_height`), which then sizes its column or row.
    ///
    /// The views may be given as a list or as a tuple, and the tuple may **mix**
    /// charts and compositions — which is what a nested layout needs:
    ///
    /// ```rust,ignore
    /// Composition::hconcat(vec![chart_a, chart_b])       // one kind of view
    /// Composition::vconcat((top_strip, inner_page))       // a chart and a page
    /// ```
    pub fn hconcat(views: impl IntoViews) -> Self {
        Self::sequence(views.into_views(), true)
    }

    /// A single column of views, top to bottom. See [`hconcat`](Self::hconcat).
    pub fn vconcat(views: impl IntoViews) -> Self {
        Self::sequence(views.into_views(), false)
    }

    /// The common implementation of [`hconcat`](Self::hconcat) and
    /// [`vconcat`](Self::vconcat): a one-dimensional grid.
    fn sequence(views: Vec<View>, horizontal: bool) -> Self {
        let count = views.len().max(1);
        let mut composition = if horizontal {
            Self::grid(1, count)
        } else {
            Self::grid(count, 1)
        };
        for (index, view) in views.into_iter().enumerate() {
            if horizontal {
                composition = composition.with_cell(0, index, view);
            } else {
                composition = composition.with_cell(index, 0, view);
            }
        }
        composition
    }

    /// A main view with **marginal strips** on some of its edges — the
    /// `ggExtra::ggMarginal` / Vega-Lite `concat` recipe, as one call.
    ///
    /// ```text
    ///            column 0                  column 1
    ///          +-------------------------+-----------+
    ///  row 0   |  top strip              |  (empty)  |   60px
    ///          +-------------------------+-----------+
    ///  row 1   |  main view              | right     |   the rest
    ///          |                         | strip     |
    ///          +-------------------------+-----------+
    ///               the rest                60px
    /// ```
    ///
    /// The layout is not a special case in the engine: it is a grid whose strip
    /// tracks are [`TrackSize::fixed`], whose [`Align`] is [`Align::Each`], and
    /// whose corners are left empty. Building that by hand is five coordinated
    /// settings, and missing any one of them fails *silently* — forget the
    /// alignment and the strips line up with the canvas instead of the data,
    /// forget `coord_flip` on the right strip and it grows sideways. This
    /// constructor is that composition, named.
    ///
    /// ```rust,ignore
    /// Composition::marginal(scatter, Marginals::new()
    ///         .with_top(top_hist)        // summarises x
    ///         .with_right(right_hist))   // summarises y, so it is `coord_flip`ped
    ///     .with_spacing(6.0)
    ///     .with_legend_position(LegendPosition::Right)
    ///     .save("marginal.svg")?;
    /// ```
    ///
    /// # What it decides, and what it leaves alone
    ///
    /// It decides the **layout**: how many tracks there are, which of them are
    /// fixed, where each view sits, and that the cells line up by their data
    /// rectangles. Everything the views draw stays their own business:
    ///
    /// * a strip usually hides the axis it shares (`with_show_x_axis(false)` for
    ///   a top strip) and its legend (`theme.with_show_legend(false)`), because
    ///   repeating either would say the same thing twice — but that is a
    ///   presentation choice, and a page is not allowed to make it for you;
    /// * a left or right strip is normally written as an upright histogram and
    ///   flipped with `coord_flip`; the flip is part of how that chart is
    ///   drawn, so it is applied to the view, not here.
    ///
    /// The one thing it *is* allowed to assume is that a strip summarising x sits
    /// above or below its main view (sharing a column, therefore sharing x) and a
    /// strip summarising y sits beside it (sharing a row, therefore sharing y).
    ///
    /// Passing `Marginals::new()` with no strips gives a plain one-cell page that
    /// aligns by data rectangle, which draws exactly like the chart alone.
    ///
    /// Reach for [`grid`](Self::grid) instead as soon as a figure stops matching
    /// those assumptions — strips of different thicknesses, a strip further from
    /// the main view, two main views, or a different [`Align`]. The result is an
    /// ordinary `Composition`, so tweaking it after the call (a corner cell,
    /// spacing, a title, a hoisted legend) never requires giving it up.
    pub fn marginal(main: impl Into<View>, margins: Marginals) -> Self {
        let rows = 1 + usize::from(margins.top.is_some()) + usize::from(margins.bottom.is_some());
        let cols = 1 + usize::from(margins.left.is_some()) + usize::from(margins.right.is_some());

        // Where the main view lands: the strips take one track each, so the main
        // view is offset by whichever of them sit above / to the left.
        let main_row = usize::from(margins.top.is_some());
        let main_col = usize::from(margins.left.is_some());
        let (last_row, last_col) = (rows - 1, cols - 1);

        // A strip is thin and must stay thin whatever the page size does; the main
        // view takes the rest. The cells are added in reading order, so the page
        // is the one you would have built by hand.
        let mut page = Self::grid(rows, cols)
            // Aligning by data rectangle is what makes these cells one figure
            // rather than three charts that happen to be near each other.
            .with_align(Align::Each);

        if let Some(strip) = margins.top {
            page = page
                .with_row_height(0, TrackSize::fixed(margins.strip))
                .with_cell(0, main_col, strip);
        }
        if let Some(strip) = margins.left {
            page = page
                .with_column_width(0, TrackSize::fixed(margins.strip))
                .with_cell(main_row, 0, strip);
        }

        page = page.with_cell(main_row, main_col, main);

        if let Some(strip) = margins.right {
            page = page
                .with_column_width(last_col, TrackSize::fixed(margins.strip))
                .with_cell(main_row, last_col, strip);
        }
        if let Some(strip) = margins.bottom {
            page = page
                .with_row_height(last_row, TrackSize::fixed(margins.strip))
                .with_cell(last_row, main_col, strip);
        }
        page
    }

    /// Sets how wide one **column** track is.
    ///
    /// `column` is the 0-based column index, `size` is either a fixed number of
    /// pixels or a share of the space left over (see [`TrackSize`]). The width of a
    /// column is the width its cells get, axis furniture included.
    ///
    /// ```rust,ignore
    /// Composition::grid(1, 2)
    ///     .with_column_width(0, TrackSize::fill(1.0))   // whatever is left
    ///     .with_column_width(1, TrackSize::fixed(60.0)) // a 60px strip
    /// ```
    ///
    /// An index outside the grid is ignored. A view placed in a single column
    /// can also ask for that width itself with `with_width`, and an explicit
    /// `with_column_width` always wins over such a request.
    pub fn with_column_width(mut self, column: usize, size: impl Into<TrackSize>) -> Self {
        if let Some(slot) = self.columns.get_mut(column) {
            *slot = size.into();
        }
        self
    }

    /// Sets how tall one **row** track is. See
    /// [`with_column_width`](Self::with_column_width).
    pub fn with_row_height(mut self, row: usize, size: impl Into<TrackSize>) -> Self {
        if let Some(slot) = self.rows.get_mut(row) {
            *slot = size.into();
        }
        self
    }

    /// Places a view in the cell at `row` / `col`.
    ///
    /// Both indices are 0-based and count from the top-left corner, rows first.
    /// A cell is the intersection of one row track and one column track, so its
    /// canvas is the size of those two tracks. Several views may be placed in
    /// the same cell; the later one is drawn on top.
    ///
    /// ```rust,ignore
    /// Composition::grid(2, 2)
    ///     .with_cell(0, 0, top_hist)   // row 0, column 0
    ///     .with_cell(1, 0, scatter)    // row 1, column 0
    ///     .with_cell(1, 1, right_hist) // row 1, column 1
    /// ```
    ///
    /// The order is `(row, col, view)`, matching [`grid(rows, columns)`](Self::grid).
    /// A cell that covers more than one track is placed with
    /// [`with_spanning_cell`](Self::with_spanning_cell).
    pub fn with_cell(self, row: usize, col: usize, view: impl Into<View>) -> Self {
        self.with_spanning_cell(row, col, 1, 1, view)
    }

    /// Places a view over a rectangle of the grid, i.e. a cell that spans
    /// several rows and/or columns.
    ///
    /// `row_span` / `col_span` are counts, not end indices: `(1, 2)` means "this
    /// cell is one row tall and two columns wide". Spans are how a wide strip
    /// sits above two columns, or one view sits beside a stack of others.
    ///
    /// ```rust,ignore
    /// Composition::grid(2, 2)
    ///     .with_spanning_cell(0, 0, 1, 2, wide_strip) // one row, both columns
    ///     .with_cell(1, 0, left)
    ///     .with_cell(1, 1, right)
    /// ```
    ///
    /// A spanning cell takes the tracks it covers plus the spacing between them.
    /// It does not take part in flush alignment (there is no single column whose
    /// panel it could share), and it never resizes the tracks it crosses.
    pub fn with_spanning_cell(
        mut self,
        row: usize,
        col: usize,
        row_span: usize,
        col_span: usize,
        view: impl Into<View>,
    ) -> Self {
        self.cells.push(Cell {
            row,
            col,
            row_span: row_span.max(1),
            col_span: col_span.max(1),
            view: view.into(),
        });
        self
    }

    /// The gap between neighbouring tracks, in pixels (4 by default) — both
    /// between columns and between rows.
    ///
    /// The gap is *between* tracks: a 2-column page pays for one gap, not two, and
    /// the spacing is taken out of the page before the tracks are sized. A page
    /// that wants a different gap on each axis sets them one at a time with
    /// [`with_column_spacing`](Self::with_column_spacing) and
    /// [`with_row_spacing`](Self::with_row_spacing).
    pub const fn with_spacing(mut self, px: f64) -> Self {
        let px = if px > 0.0 { px } else { 0.0 };
        self.column_spacing = px;
        self.row_spacing = px;
        self
    }

    /// The gap between neighbouring **columns**, in pixels.
    ///
    /// `wspace` in matplotlib, and the gap a marginal layout usually wants to be
    /// different from the vertical one: a strip sits close to the view it
    /// annotates, while two independent columns stand further apart.
    pub const fn with_column_spacing(mut self, px: f64) -> Self {
        self.column_spacing = if px > 0.0 { px } else { 0.0 };
        self
    }

    /// The gap between neighbouring **rows**, in pixels. See
    /// [`with_column_spacing`](Self::with_column_spacing).
    pub const fn with_row_spacing(mut self, px: f64) -> Self {
        self.row_spacing = if px > 0.0 { px } else { 0.0 };
        self
    }

    /// Chooses how the cells of the page line up (and therefore what they share).
    ///
    /// [`Align::None`] (the default) places every cell by its canvas;
    /// [`Align::Each`] gives the cells of a column one common data rectangle (and a
    /// row one), which is what a marginal strip needs; [`Align::All`] gives the
    /// whole page one. See [`Align`] for what each of them shares.
    pub const fn with_align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Chooses whether the cells that share a data rectangle also share the scale
    /// in it.
    ///
    /// [`Resolve::Shared`] (the default) is what makes a marginal plot a single
    /// figure: the strips adopt the domain the main view resolved. A cell that sets
    /// its own domain explicitly keeps it either way, and only numeric domains are
    /// shared (a list of labels would be renamed rather than widened).
    ///
    /// Only consulted when there *is* a shared rectangle — [`Align::Each`] or
    /// [`Align::All`].
    pub const fn with_resolve(mut self, resolve: Resolve) -> Self {
        self.resolve = resolve;
        self
    }

    /// Draws the legend of the **largest cell** on an edge of the page.
    ///
    /// A legend belongs to the view that declares it, so by default it is drawn
    /// inside that view's own band — which is right when the view sits at the page
    /// edge, and wrong when it sits between two data rectangles: the band of a
    /// middle column falls *between* two panels.
    ///
    /// This method moves it out instead. The cell keeps its axis furniture, the
    /// page reserves a band on the requested edge, and the legend is drawn there,
    /// lined up with the panel it explains:
    ///
    /// ```text
    ///   +--------+--------+----------------------+---------+
    ///   |        |        |   top histogram      |         |
    ///   |        |        +----------------------+         |
    ///   | legend |        |   main view          | right   |
    ///   |        |        |                      | strip   |
    ///   +--------+--------+----------------------+---------+
    /// ```
    ///
    /// The name, the argument and the meaning match
    /// [`Theme::with_legend_position`](crate::theme::Theme), which is how a single
    /// chart places its own legend; here it places the page's. It is literally the
    /// same field, so this method and
    /// `configure_theme(|t| t.with_legend_position(edge))` are the same call — a
    /// page has one legend position, not two.
    ///
    /// [`LegendPosition::None`] is the page default and means "hoist nothing": every
    /// cell keeps the legend it declares, in its own band.
    ///
    /// Only one legend is moved (the largest cell's); the others stay in their own
    /// cells, which the caller can switch off with `with_show_legend(false)` on the
    /// view itself. Guides are not merged across cells; that is a separate
    /// feature.
    pub fn with_legend_position(mut self, position: LegendPosition) -> Self {
        self.theme = self.theme.with_legend_position(position);
        self
    }

    /// Names the cell whose legend [`with_legend_position`](Self::with_legend_position)
    /// draws.
    ///
    /// Without it the owner is chosen for you: the **largest cell that has a
    /// legend**, which in a marginal layout is the main view and never one of the
    /// strips. That rule is right almost always, and invisible when it is not —
    /// two views of the same size with a legend each have no obvious winner.
    ///
    /// Naming the cell makes the choice explicit. It is a promise, not a hint: if
    /// that cell has no legend to give, the page draws none rather than quietly
    /// picking another.
    ///
    /// ```rust,ignore
    /// page.with_legend_position(LegendPosition::Right)
    ///     .with_legend_from(1, 0)   // the scatter at row 1, column 0
    /// ```
    pub const fn with_legend_from(mut self, row: usize, col: usize) -> Self {
        self.legend_from = Some((row, col));
        self
    }

    /// The page size in pixels.
    ///
    /// Defaults to 500 × 400, the canvas a single chart has, so a composition
    /// drops into a page of charts without a different scale.
    pub const fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Multiplies the pixel density of raster output, without changing the layout.
    ///
    /// Exactly the same knob a single chart has (`LayeredChart::with_scale_factor`):
    /// the page still lays out at its configured size, but `to_png` writes
    /// `width × height × factor` pixels. `1.0` (the default) means one pixel per
    /// point; `2.0` is a Retina / print-density copy of the same figure.
    pub const fn with_scale_factor(mut self, factor: f32) -> Self {
        self.scale_factor = if factor > 0.0 { factor } else { 1.0 };
        self
    }

    /// The theme used for the page itself (background, title typography).
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Modifies the page theme fluently.
    pub fn configure_theme<F>(mut self, f: F) -> Self
    where
        F: FnOnce(Theme) -> Theme,
    {
        self.theme = f(self.theme);
        self
    }

    /// The title drawn above the grid, and exported as the document's name.
    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The accessible description of the whole page.
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The page size.
    pub const fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    // ------------------------------------------------------------------------
    // Layout
    // ------------------------------------------------------------------------

    /// Resolves track starts and extents for one axis.
    fn tracks(sizes: &[TrackSize], total: f64, spacing: f64) -> (Vec<f64>, Vec<f64>) {
        let count = sizes.len();
        if count == 0 {
            return (Vec::new(), Vec::new());
        }

        let gaps = spacing * count.saturating_sub(1) as f64;
        let available = (total - gaps).max(0.0);
        let fixed: f64 = sizes
            .iter()
            .map(|size| match size {
                TrackSize::Fixed(px) => px.max(0.0),
                TrackSize::Fill(_) => 0.0,
            })
            .sum();
        let weight: f64 = sizes
            .iter()
            .map(|size| match size {
                TrackSize::Fill(weight) => weight.max(0.0),
                TrackSize::Fixed(_) => 0.0,
            })
            .sum();
        let fill_budget = (available - fixed).max(0.0);

        let mut starts = Vec::with_capacity(count);
        let mut extents = Vec::with_capacity(count);
        let mut cursor = 0.0;
        for size in sizes {
            let extent = size.resolve(fill_budget, weight);
            starts.push(cursor);
            extents.push(extent);
            cursor += extent + spacing;
        }
        (starts, extents)
    }

    /// The track sizes actually used, after letting a lone cell speak for its
    /// track.
    ///
    /// A view can ask for its own room (`with_width(TrackSize::Fixed(60.0))`), which
    /// is how Vega-Lite's `"height": 60` reads. That request is honoured only
    /// where it is unambiguous: a cell that occupies exactly one track, in a grid
    /// whose track was left at its default. An explicit
    /// [`with_column_width`](Self::with_column_width) /
    /// [`with_row_height`](Self::with_row_height) always wins, and a spanning cell
    /// never resizes the tracks it crosses.
    fn effective_track_sizes(&self) -> (Vec<TrackSize>, Vec<TrackSize>) {
        let mut columns = self.columns.clone();
        let mut rows = self.rows.clone();

        for cell in &self.cells {
            let view = &cell.view.0;
            if cell.col_span == 1
                && let Some(size) = view.requested_width()
                && let Some(slot) = columns.get_mut(cell.col)
                && *slot == TrackSize::default()
            {
                *slot = size;
            }
            if cell.row_span == 1
                && let Some(size) = view.requested_height()
                && let Some(slot) = rows.get_mut(cell.row)
                && *slot == TrackSize::default()
            {
                *slot = size;
            }
        }

        (columns, rows)
    }

    /// The box a cell covers, including the spacing between the tracks it spans.
    fn span_box(
        starts: &[f64],
        extents: &[f64],
        spacing: f64,
        first: usize,
        span: usize,
    ) -> (f64, f64) {
        let last = (first + span).min(extents.len());
        let start = starts.get(first).copied().unwrap_or(0.0);
        let extent: f64 = extents
            .get(first..last)
            .map(|slice| slice.iter().sum::<f64>())
            .unwrap_or(0.0)
            + spacing * (last.saturating_sub(first).saturating_sub(1)) as f64;
        (start, extent.max(0.0))
    }

    /// Places every cell, measures it, and settles the panels and scales its
    /// column and row share.
    ///
    /// `imposed` carries what a **parent** composition decided about this page
    /// (a shared rectangle and a shared scale), and `hidden_legend` names the one
    /// cell whose legend is drawn somewhere else, so that its own band is not
    /// reserved (otherwise the page-wide legend would leave a hole behind inside
    /// that cell).
    ///
    /// No fixed-point loop is needed: track sizes are a linear solve, and a pinned
    /// panel is absolute, so what is decided here is exactly what gets drawn.
    fn solve(
        &self,
        canvas: Rect,
        imposed: &Imposed,
        hidden_legend: Option<usize>,
    ) -> Result<Vec<Placed>, ChartonError> {
        if self.cells.is_empty() {
            return Err(ChartonError::Render(
                "A composition needs at least one cell".to_string(),
            ));
        }

        let (columns, rows) = self.effective_track_sizes();
        let (col_starts, col_extents) = Self::tracks(&columns, canvas.width, self.column_spacing);
        let (row_starts, row_extents) = Self::tracks(&rows, canvas.height, self.row_spacing);

        if col_extents.is_empty() || row_extents.is_empty() {
            return Err(ChartonError::Render(
                "A composition needs at least one row and one column".to_string(),
            ));
        }

        let mut placed = Vec::with_capacity(self.cells.len());
        for (index, cell) in self.cells.iter().enumerate() {
            if cell.row >= row_extents.len() || cell.col >= col_extents.len() {
                return Err(ChartonError::Render(format!(
                    "Cell at ({}, {}) is outside the {} x {} grid",
                    cell.row,
                    cell.col,
                    self.columns.len(),
                    self.rows.len()
                )));
            }
            let (dx, width) = Self::span_box(
                &col_starts,
                &col_extents,
                self.column_spacing,
                cell.col,
                cell.col_span,
            );
            let (dy, height) = Self::span_box(
                &row_starts,
                &row_extents,
                self.row_spacing,
                cell.row,
                cell.row_span,
            );
            let rect = Rect::new(canvas.x + dx, canvas.y + dy, width, height);

            // Every cell is measured at the size it will be drawn at, so the panel
            // alignment works on the panel that ends up in the output. A chart and
            // a nested composition are asked the same way, which is what lets
            // alignment cross a nesting boundary.
            let metrics = cell
                .view
                .0
                .measure(width, height, hidden_legend != Some(index))?;

            placed.push(Placed {
                rect,
                panel: metrics.panel,
                title_bottom: metrics.title_bottom,
                legend: metrics.legend,
                scales: metrics.scales,
                pinned: metrics.pinned,
                index,
            });
        }

        if self.align != Align::None {
            // A parent's rectangle and scale win over this page's own. They are
            // written into the **master** cell, which is the cell a parent
            // measured and pinned; `share_panels` and `share_scales` then carry
            // them to the rest of the track, so the page stays internally
            // consistent instead of being overridden at draw time.
            if let Some(master) = master_of(&placed) {
                if let Some(pin) = imposed.panel {
                    placed[master].panel = pin;
                }
                for axis in Axis::ALL {
                    if let Some(shared) = &imposed.scales[axis.index()] {
                        placed[master].scales[axis.index()] = Some(shared.clone());
                    }
                }
            }
            self.share_panels(&mut placed);
            if self.resolve == Resolve::Shared {
                self.share_scales(&mut placed);
            }
        }

        Ok(placed)
    }

    /// The bucket a track index falls into when a panel is shared.
    ///
    /// [`Align::Each`] shares one panel per track, so a bucket *is* the track.
    /// [`Align::All`] shares one panel for the whole page, so every track falls
    /// into bucket 0 and every cell meets every other cell in the intersection.
    const fn bucket(&self, index: usize) -> usize {
        match self.align {
            Align::All => 0,
            Align::Each | Align::None => index,
        }
    }

    /// Gives the cells that share a panel one common rectangle.
    ///
    /// The common rectangle is the **intersection** of the cells' own panels:
    /// the latest start and the earliest end. Intersecting, rather than uniting,
    /// guarantees that a pinned panel stays inside the box the cell already owns,
    /// so aligning can never push one cell's furniture over its neighbour.
    ///
    /// *Which* cells share is decided by [`Composition::bucket`]: per column and
    /// per row under [`Align::Each`], page-wide under [`Align::All`].
    fn share_panels(&self, placed: &mut [Placed]) {
        let mut columns: Vec<(f64, f64)> = Vec::new(); // (start, end)
        let mut rows: Vec<(f64, f64)> = Vec::new();

        for item in placed.iter() {
            let panel = item.panel;
            let cell = &self.cells[item.index];
            // Only single-track cells take part: a spanning cell covers several
            // tracks, so there is no single column whose panel it could share.
            if cell.col_span == 1 {
                narrow(
                    &mut columns,
                    self.bucket(cell.col),
                    panel.x,
                    panel.x + panel.width,
                );
            }
            if cell.row_span == 1 {
                narrow(
                    &mut rows,
                    self.bucket(cell.row),
                    panel.y,
                    panel.y + panel.height,
                );
            }
        }

        for item in placed.iter_mut() {
            let (col, row, col_span, row_span) = {
                let cell = &self.cells[item.index];
                (cell.col, cell.row, cell.col_span, cell.row_span)
            };
            let panel = &mut item.panel;
            if col_span == 1
                && let Some(&(start, end)) = columns.get(self.bucket(col))
                && end > start
            {
                panel.x = start;
                panel.width = (end - start).max(1.0);
            }
            if row_span == 1
                && let Some(&(start, end)) = rows.get(self.bucket(row))
                && end > start
            {
                // Sharing a data rectangle is never a reason to draw the cell's
                // title over its data, so the shared top stops below the title.
                let start = start.max(item.title_bottom);
                if end > start {
                    panel.y = start;
                    panel.height = (end - start).max(1.0);
                }
            }
        }
    }

    /// Gives the cells that share a panel one horizontal scale, and one vertical
    /// one.
    ///
    /// The reference is the largest cell of the group — the main view of a
    /// marginal composition is by construction the big one — and a cell that
    /// pinned its own domain keeps it: an explicit domain is a promise to the
    /// caller, never something a layout may overwrite.
    ///
    /// Which cells form a group is decided by [`Composition::bucket`], exactly as
    /// the panels are: a track under [`Align::Each`], the whole page under
    /// [`Align::All`].
    ///
    /// The directions are the **page's** (see [`Axis`]), not the channel names: a
    /// right-hand marginal is a `coord_flip`ped histogram, so its own y channel
    /// carries the count while the scale it borrows is the one drawn vertically.
    fn share_scales(&self, placed: &mut [Placed]) {
        let mut specs: [Vec<Option<SharedScale>>; 2] = [Vec::new(), Vec::new()];
        let mut areas: [Vec<f64>; 2] = [Vec::new(), Vec::new()];

        // Pass 1: find each group's reference scale.
        for item in placed.iter() {
            let cell = &self.cells[item.index];
            let area = item.panel.width * item.panel.height;
            for axis in Axis::ALL {
                let track = match axis {
                    Axis::Horizontal => cell.col,
                    Axis::Vertical => cell.row,
                };
                if span_of(cell, axis) != 1 {
                    continue;
                }
                prefer_largest(
                    &mut specs[axis.index()],
                    &mut areas[axis.index()],
                    self.bucket(track),
                    area,
                    item.scales[axis.index()].clone(),
                );
            }
        }

        // Pass 2: hand the reference scale to the cells that did not pin one.
        for item in placed.iter_mut() {
            let cell = &self.cells[item.index];
            let (col, row) = (cell.col, cell.row);
            for axis in Axis::ALL {
                if span_of(cell, axis) != 1 || item.pinned[axis.index()] {
                    continue;
                }
                let track = match axis {
                    Axis::Horizontal => col,
                    Axis::Vertical => row,
                };
                let Some(shared) = specs[axis.index()]
                    .get(self.bucket(track))
                    .and_then(Option::as_ref)
                else {
                    continue;
                };
                // Only a scale of the same kind may stand in: a shared list of
                // labels would rename the categories instead of widening a range.
                if let Some(own) = &item.scales[axis.index()]
                    && same_kind(own, shared)
                {
                    item.scales[axis.index()] = Some(shared.clone());
                }
            }
        }
    }

    // ------------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------------

    /// Draws the whole page into `backend`, inside `canvas`.
    ///
    /// Public entry point from the document wrappers; `canvas` is the full page
    /// (the title band, if any, is carved out here rather than by the caller, so
    /// a nested composition and the page behave the same way).
    pub fn render<B: RenderBackend>(&self, backend: &mut B) -> Result<(), ChartonError> {
        let canvas = Rect::new(0.0, 0.0, self.width as f64, self.height as f64);
        self.render_into(backend, canvas, &CellPlan::default())
    }

    /// Draws the page inside `canvas`, honouring what a parent decided about it.
    ///
    /// `plan` is the parent's [`CellPlan`] on the page's own top-level call
    /// (nothing imposed), and the parent's decisions when this page is a cell of
    /// another one.
    fn render_into(
        &self,
        backend: &mut dyn RenderBackend,
        canvas: Rect,
        plan: &CellPlan,
    ) -> Result<(), ChartonError> {
        // 1. The page keeps its margins (the theme's, as a fraction of the
        //    canvas), the title takes the top of what is left, and the grid gets
        //    the rest. A page with no title and no legend therefore still has the
        //    same breathing room around its cells as a single chart.
        let content = self.page_content(canvas);
        let area = self.render_title(backend, content)?;

        // 2. Solve the page: the legend band (if the page draws one), then the
        //    cells inside what is left.
        let imposed = Imposed {
            panel: plan.panel,
            scales: clone_scales(&plan.scales),
        };
        let legend = if plan.show_legend {
            LegendUse::Page
        } else {
            LegendUse::Elsewhere
        };
        let page = self.layout(area, &imposed, legend)?;

        // 3. Draw the cells, each moved to its own rectangle.
        for item in &page.cells {
            backend.begin_group(&Group::new(Role::View));
            // The cell draws in its own `(0, 0)` coordinates; the offset backend
            // moves the whole picture to the cell's rectangle on the page.
            let mut target = OffsetBackend::new(&mut *backend, item.rect.x, item.rect.y);
            self.cells[item.index].view.0.draw(
                &mut target,
                item.rect.width,
                item.rect.height,
                &self.cell_plan(item, page.hidden),
            )?;
            backend.end_group();
        }

        // 4. Draw the page-wide legend last, over the empty band.
        if let (Some(legend), true) = (&page.legend, plan.show_legend) {
            self.render_hoisted_legend(backend, legend)?;
        }

        Ok(())
    }

    /// What the page tells one of its cells to do.
    ///
    /// The panel and the scales are only imposed when something is shared
    /// ([`Align::Each`] or [`Align::All`]): under [`Align::None`] a cell keeps
    /// exactly the layout it computed for itself, so pinning it to the rectangle
    /// it already had would be a promise about rounding rather than about
    /// alignment.
    fn cell_plan(&self, item: &Placed, hoisted_legend: Option<usize>) -> CellPlan {
        let shared = self.align != Align::None;
        CellPlan {
            panel: shared.then_some(item.panel),
            show_legend: hoisted_legend != Some(item.index),
            scales: if shared {
                clone_scales(&item.scales)
            } else {
                [None, None]
            },
        }
    }

    /// The page inside the theme's margins.
    ///
    /// The margins are read exactly as a single chart reads them — fractions of
    /// the canvas — so a composition and a chart sit in a document with the same
    /// outer spacing. Set the theme margins to `0.0` to let the cells reach the
    /// canvas edge.
    fn page_content(&self, canvas: Rect) -> Rect {
        let (width, height) = (canvas.width, canvas.height);
        let left = self.theme.left_margin * width;
        let right = self.theme.right_margin * width;
        let top = self.theme.top_margin * height;
        let bottom = self.theme.bottom_margin * height;
        Rect::new(
            canvas.x + left,
            canvas.y + top,
            (width - left - right).max(1.0),
            (height - top - bottom).max(1.0),
        )
    }

    /// The rectangle the cells are solved in: the page inside the theme's
    /// margins, below the title band.
    ///
    /// Used when a nested page is measured, where nothing is drawn and so
    /// [`Self::render_title`] cannot be used to find out how much the title took.
    fn grid_area(&self, canvas: Rect) -> Rect {
        let content = self.page_content(canvas);
        let band = self.title_band();
        Rect::new(
            content.x,
            content.y + band,
            content.width,
            (content.height - band).max(1.0),
        )
    }

    /// Everything the page needs before a single mark is drawn.
    ///
    /// `legend` says who draws this page's legend, if anyone:
    /// [`LegendUse::Page`] lets the page hoist one cell's legend onto an edge,
    /// [`LegendUse::Elsewhere`] means a parent composition draws it, so no cell
    /// here may.
    ///
    /// The page is solved once to find out who owns the legend, and a second time
    /// inside the band that leaves: two looks, never a loop, because nothing about
    /// the solve is iterative.
    fn layout(
        &self,
        area: Rect,
        imposed: &Imposed,
        legend: LegendUse,
    ) -> Result<PageLayout, ChartonError> {
        // The legend is chosen against the *whole* page: the band about to be
        // reserved must not change which cell owns the legend.
        let mut natural = self.solve(area, imposed, None)?;

        match legend {
            // Nobody here draws it, so the cell that would have must not either.
            LegendUse::Elsewhere => {
                let hidden = self.legend_owner(&natural);
                let cells = match hidden {
                    Some(owner) => self.solve(area, imposed, Some(owner))?,
                    None => natural,
                };
                Ok(PageLayout {
                    cells,
                    legend: None,
                    hidden,
                })
            }
            // The page draws its own legend, unless it never asked for one: then
            // every cell keeps the legend it declares, in its own band.
            LegendUse::Page if !self.draws_own_legend() => Ok(PageLayout {
                cells: natural,
                legend: None,
                hidden: None,
            }),
            LegendUse::Page => {
                let Some(owner) = self.legend_owner(&natural) else {
                    return Ok(PageLayout {
                        cells: natural,
                        legend: None,
                        hidden: None,
                    });
                };
                let hoisted = self.hoisted_legend(area, owner, &mut natural);
                let cells = self.solve(hoisted.grid, imposed, Some(owner))?;
                Ok(PageLayout {
                    cells,
                    legend: Some(hoisted),
                    hidden: Some(owner),
                })
            }
        }
    }

    /// The cell whose legend the page draws somewhere else.
    ///
    /// By default the **largest cell that has a legend** (by canvas): in a marginal layout
    /// that is the main view, and the strips (which have none) never compete. A
    /// caller can name the cell instead with [`with_legend_from`](Self::with_legend_from),
    /// which is a promise rather than a hint — a named cell with nothing to hand
    /// over leaves the page with no legend instead of quietly picking another.
    fn legend_owner(&self, placed: &[Placed]) -> Option<usize> {
        if let Some((row, col)) = self.legend_from {
            return self
                .cells
                .iter()
                .position(|cell| cell.row == row && cell.col == col)
                .filter(|index| placed[*index].legend.is_some());
        }
        placed
            .iter()
            .filter(|item| item.legend.is_some())
            .max_by(|a, b| {
                a.footprint()
                    .total_cmp(&b.footprint())
                    .then(b.index.cmp(&a.index))
            })
            .map(|item| item.index)
    }

    /// Reserves a page band for one cell's legend, if the page draws it.
    ///
    /// The band is laid out the way a single chart lays out its own legend: the
    /// theme's page margin keeps it off the canvas edge, the legend margin
    /// separates it from the grid, and the legend may draw in the whole open area
    /// between the two, so an estimated text width that comes out short does not
    /// clip a label.
    fn hoisted_legend(&self, area: Rect, owner: usize, placed: &mut [Placed]) -> HoistedLegend {
        let item = &mut placed[owner];
        let material = item
            .legend
            .take()
            .expect("the owner was chosen because it has a legend");

        // Pack the guides against the panel they explain, then charge the page for
        // the strip, exactly the way a single chart charges itself.
        let direction = Direction::for_legend(self.theme.legend_position);
        let budget = direction
            .measure(material.panel.width, material.panel.height)
            .main;
        let plan = crate::core::layout::LayoutEngine::pack_guides(
            &material.guides,
            self.theme.legend_position,
            budget,
            &material.theme,
        );
        let thickness = crate::core::layout::LayoutEngine::legend_thickness(
            &plan,
            area.width,
            area.height,
            &material.theme,
        );

        // The same two gaps a single chart puts on either side of its legend: the
        // page margin outside, the legend margin towards the grid.
        let outer = match self.theme.legend_position {
            LegendPosition::Left => self.theme.left_margin * area.width,
            LegendPosition::Right => self.theme.right_margin * area.width,
            LegendPosition::Top => self.theme.top_margin * area.height,
            LegendPosition::Bottom => self.theme.bottom_margin * area.height,
            LegendPosition::None => 0.0,
        };
        let edges = reserve_edge(
            self.theme.legend_position,
            area,
            outer,
            thickness,
            self.theme.legend_margin,
        );

        HoistedLegend {
            band: edges.band,
            clip: edges.clip,
            grid: edges.grid,
            cell: item.rect,
            plan,
            material,
        }
    }

    /// Draws a hoisted legend in the band reserved for it.
    ///
    /// The legend is drawn with the *cell's* theme and coordinate system (so its
    /// symbols and typography are the ones it was measured with), and its context
    /// panel is the cell's panel moved into page coordinates, which is what makes
    /// the legend line up with the data rectangle it explains.
    fn render_hoisted_legend(
        &self,
        backend: &mut dyn RenderBackend,
        legend: &HoistedLegend,
    ) -> Result<(), ChartonError> {
        let spec = ChartSpec {
            aesthetics: &legend.material.aesthetics,
            theme: &legend.material.theme,
        };
        let panel = Rect::new(
            legend.cell.x + legend.material.panel.x,
            legend.cell.y + legend.material.panel.y,
            legend.material.panel.width,
            legend.material.panel.height,
        );
        let ctx = PanelContext::new(&spec, legend.material.coord.clone(), panel);
        // The legend renderers are generic over a sized backend; a zero offset
        // is how a `dyn` target reaches them without teaching every renderer
        // about trait objects.
        let mut target = OffsetBackend::new(backend, 0.0, 0.0);
        crate::render::legend_renderer::LegendRenderer::render_legend(
            &mut target,
            &legend.material.guides,
            &legend.plan,
            &legend.material.theme,
            &ctx,
            &legend.band,
            &legend.clip,
        )
    }

    /// Whether this page draws a legend of its own on an edge of its box.
    ///
    /// A page that hoists a legend owns it, so it has none to hand to a parent:
    /// a legend can be claimed once.
    const fn draws_own_legend(&self) -> bool {
        !matches!(self.theme.legend_position, LegendPosition::None)
    }

    /// The height of the page title band (0 when the page has no title).
    ///
    /// Read both when drawing and when measuring a nested page, so a parent
    /// measures the band the child will actually reserve.
    fn title_band(&self) -> f64 {
        if self.title.is_some() {
            self.theme.title_size * 1.2 + self.theme.title_padding
        } else {
            0.0
        }
    }

    /// Draws the page title, and returns the rectangle left for the grid.
    fn render_title(
        &self,
        backend: &mut dyn RenderBackend,
        canvas: Rect,
    ) -> Result<Rect, ChartonError> {
        let Some(title) = &self.title else {
            return Ok(canvas);
        };

        let band = self.title_band();
        let (x, anchor) = match self.theme.title_anchor {
            TitleAnchor::Start => (canvas.x, "start"),
            TitleAnchor::Middle => (canvas.x + canvas.width / 2.0, "middle"),
            TitleAnchor::End => (canvas.x + canvas.width, "end"),
        };

        backend.begin_group(&Group::new(Role::Title));
        backend.draw_text(TextConfig {
            x: x as Precision,
            y: (canvas.y + band / 2.0) as Precision,
            text: title.clone(),
            font_size: self.theme.title_size as Precision,
            font_family: self.theme.title_family.clone(),
            color: self.theme.title_color,
            text_anchor: anchor.to_string(),
            dominant_baseline: "middle".into(),
            font_weight: "bold".to_string(),
            font_style: "normal".to_string(),
            opacity: 1.0,
            angle: 0.0,
        })?;
        backend.end_group();

        Ok(Rect::new(
            canvas.x,
            canvas.y + band,
            canvas.width,
            (canvas.height - band).max(1.0),
        ))
    }

    /// Paints the page background, then every cell.
    fn render_page<B: RenderBackend>(&self, backend: &mut B) -> Result<(), ChartonError> {
        backend.begin_group(&Group::new(Role::Background));
        backend.draw_rect(RectConfig {
            x: 0.0,
            y: 0.0,
            width: self.width as Precision,
            height: self.height as Precision,
            fill: self.theme.background_color,
            stroke: "none".into(),
            stroke_width: 0.0,
            opacity: 1.0,
        });
        backend.end_group();
        self.render(backend)
    }

    // ------------------------------------------------------------------------
    // Export
    // ------------------------------------------------------------------------

    /// Renders the page as an SVG document.
    pub fn to_svg(&self) -> Result<String, ChartonError> {
        self.to_svg_with(&SvgOptions::default())
    }

    /// Renders the page as an SVG document with explicit serialisation options.
    ///
    /// The options decide how the document is written, never what it looks like:
    /// the same composition with different options draws exactly the same
    /// picture. Each cell is a `<g class="charton-view">` scope, so a stylesheet
    /// can address a cell as a unit.
    pub fn to_svg_with(&self, options: &SvgOptions) -> Result<String, ChartonError> {
        SvgDocument {
            width: self.width,
            height: self.height,
            title: self.title.as_deref(),
            description: self.description.as_deref(),
            options,
        }
        .render(|backend| self.render_page(backend))
    }

    /// Renders the page as a PNG image (CPU raster path).
    ///
    /// The bitmap is the page size multiplied by
    /// [`with_scale_factor`](Self::with_scale_factor), exactly as a single
    /// chart's `to_png` is: the same figure exports at the same density whether
    /// it is one chart or several.
    #[cfg(feature = "png")]
    pub fn to_png(&self) -> Result<Vec<u8>, ChartonError> {
        let plan = crate::render::backend::raster::RasterPlan::new(
            self.width,
            self.height,
            self.scale_factor,
        );
        crate::render::backend::raster::render_document(
            &plan,
            self.theme.background_color,
            |backend| self.render_page(backend),
        )?
        .encode_png()
        .map_err(|e| ChartonError::Render(format!("Failed to encode PNG: {e}")))
    }

    /// Renders the page as a tagged PDF document.
    #[cfg(feature = "pdf")]
    pub fn to_pdf(&self) -> Result<Vec<u8>, ChartonError> {
        self.to_pdf_with(&PdfOptions::default())
    }

    /// Renders the page as a PDF document with explicit options.
    ///
    /// A page with several views is still one tagged figure: the document carries
    /// the page title and description, and the cells' own titles are not repeated
    /// in the structure tree.
    #[cfg(feature = "pdf")]
    pub fn to_pdf_with(&self, options: &PdfOptions) -> Result<Vec<u8>, ChartonError> {
        PdfDocument {
            width: self.width,
            height: self.height,
            title: self.title.as_deref(),
            description: self.description.as_deref(),
            options,
        }
        .render(|backend| self.render_page(backend))
    }

    /// Writes the page to `path`, choosing the format from the extension
    /// (`svg`, `png`, `pdf`).
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ChartonError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|dir| !dir.exists()) {
            std::fs::create_dir_all(parent).map_err(ChartonError::Io)?;
        }

        match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_lowercase)
            .as_deref()
        {
            Some("svg") => std::fs::write(path, self.to_svg()?).map_err(ChartonError::Io),
            Some("png") => {
                #[cfg(feature = "png")]
                {
                    std::fs::write(path, self.to_png()?).map_err(ChartonError::Io)
                }
                #[cfg(not(feature = "png"))]
                {
                    Err(ChartonError::Unimplemented(
                        "PNG export needs the 'png' feature".to_string(),
                    ))
                }
            }
            Some("pdf") => {
                #[cfg(feature = "pdf")]
                {
                    std::fs::write(path, self.to_pdf()?).map_err(ChartonError::Io)
                }
                #[cfg(not(feature = "pdf"))]
                {
                    Err(ChartonError::Unimplemented(
                        "PDF export needs the 'pdf' feature".to_string(),
                    ))
                }
            }
            Some(other) => Err(ChartonError::Unimplemented(format!(
                "Output format '{other}' is not supported"
            ))),
            None => Err(ChartonError::Unimplemented(
                "Output format could not be determined from the file extension".to_string(),
            )),
        }
    }

    /// Displays the page in an EVCXR/Jupyter notebook.
    pub fn show(&self) -> Result<(), ChartonError> {
        let svg = self.to_svg()?;
        if std::env::var("EVCXR_IS_RUNTIME").is_ok() {
            println!("EVCXR_BEGIN_CONTENT text/html\n{svg}\nEVCXR_END_CONTENT");
        }
        Ok(())
    }
}

// ----------------------------------------------------------------------------
// A page, as a cell
// ----------------------------------------------------------------------------

/// A composition is a view too, which is what makes nesting recursive *and*
/// alignable.
///
/// Everything it reports comes from its **master cell** — the largest by canvas,
/// which in a marginal layout is the main view and never a strip:
///
/// * `panel` is that cell's data rectangle, so a parent can align a strip of its
///   own with the middle of this page;
/// * `scales` are that cell's scales, so a parent can hand this page a range and
///   have its own sharing pass distribute it;
/// * `legend` is the guides its cells would draw, unless this page already draws
///   them on its own edge — a legend can only be claimed once.
///
/// When a parent pins the panel, [`Composition::solve`] writes the pin into the
/// master cell *before* sharing, so the pin propagates to the rest of the track
/// instead of being applied at draw time behind the page's back.
impl CellView for Composition {
    fn measure(
        &self,
        width: f64,
        height: f64,
        show_legend: bool,
    ) -> Result<ViewMetrics, ChartonError> {
        let canvas = Rect::new(0.0, 0.0, width, height);
        let area = self.grid_area(canvas);
        let legend = if show_legend {
            LegendUse::Page
        } else {
            LegendUse::Elsewhere
        };
        let mut page = self.layout(area, &Imposed::none(), legend)?;

        let master = master_of(&page.cells).ok_or_else(|| {
            ChartonError::Render("A composition needs at least one cell".to_string())
        })?;

        // A page that keeps its own guides to itself can hand one on: that is the
        // legend of the cell that would have owned it.
        let legend = if show_legend && !self.draws_own_legend() {
            let owner = self.legend_owner(&page.cells);
            owner.and_then(|index| page.cells[index].legend.take())
        } else {
            None
        };

        let cell = &page.cells[master];
        Ok(ViewMetrics {
            panel: cell.panel,
            // The page's own furniture already sits above its master panel, and a
            // shared rectangle can only move down (the intersection takes the
            // latest start), so nothing here can paint a title over the data.
            title_bottom: 0.0,
            legend,
            scales: clone_scales(&cell.scales),
            pinned: cell.pinned,
        })
    }

    fn draw(
        &self,
        backend: &mut dyn RenderBackend,
        width: f64,
        height: f64,
        plan: &CellPlan,
    ) -> Result<(), ChartonError> {
        self.render_into(backend, Rect::new(0.0, 0.0, width, height), plan)
    }
}

/// Narrows the common interval of one track to include one more member.
///
/// The neutral interval is the whole line, so a track with a single cell keeps
/// exactly that cell's panel.
fn narrow(slots: &mut Vec<(f64, f64)>, index: usize, start: f64, end: f64) {
    if slots.len() <= index {
        slots.resize(index + 1, (f64::NEG_INFINITY, f64::INFINITY));
    }
    let slot = &mut slots[index];
    slot.0 = slot.0.max(start);
    slot.1 = slot.1.min(end);
}

/// Where a legend strip goes when the page reserves one of its edges.
struct EdgeBand {
    /// The strip the legend is drawn in.
    band: Rect,
    /// The page area left for the grid.
    grid: Rect,
    /// The open area between the page edge and the grid. Wider than `band`, so a
    /// label a little wider than estimated is still drawn in full.
    clip: Rect,
}

/// Splits the area along `edge`: `outer` keeps the strip off the canvas edge,
/// `thickness` is the strip itself, and `gap` separates it from the grid.
fn reserve_edge(
    edge: LegendPosition,
    area: Rect,
    outer: f64,
    thickness: f64,
    gap: f64,
) -> EdgeBand {
    let reserved = outer + thickness + gap;
    let (right, bottom) = (area.x + area.width, area.y + area.height);

    match edge {
        LegendPosition::Left => {
            let grid = Rect::new(
                area.x + reserved,
                area.y,
                (area.width - reserved).max(1.0),
                area.height,
            );
            EdgeBand {
                band: Rect::new(area.x + outer, area.y, thickness, area.height),
                clip: Rect::new(area.x, area.y, grid.x - area.x, area.height),
                grid,
            }
        }
        LegendPosition::Right => {
            let grid = Rect::new(
                area.x,
                area.y,
                (area.width - reserved).max(1.0),
                area.height,
            );
            EdgeBand {
                band: Rect::new(right - outer - thickness, area.y, thickness, area.height),
                clip: Rect::new(
                    grid.x + grid.width,
                    area.y,
                    right - (grid.x + grid.width),
                    area.height,
                ),
                grid,
            }
        }
        LegendPosition::Top => {
            let grid = Rect::new(
                area.x,
                area.y + reserved,
                area.width,
                (area.height - reserved).max(1.0),
            );
            EdgeBand {
                band: Rect::new(area.x, area.y + outer, area.width, thickness),
                clip: Rect::new(area.x, area.y, area.width, grid.y - area.y),
                grid,
            }
        }
        LegendPosition::Bottom => {
            let grid = Rect::new(
                area.x,
                area.y,
                area.width,
                (area.height - reserved).max(1.0),
            );
            EdgeBand {
                band: Rect::new(area.x, bottom - outer - thickness, area.width, thickness),
                clip: Rect::new(
                    area.x,
                    grid.y + grid.height,
                    area.width,
                    bottom - (grid.y + grid.height),
                ),
                grid,
            }
        }
        LegendPosition::None => EdgeBand {
            band: area,
            grid: area,
            clip: area,
        },
    }
}

/// The solved page: every cell placed, plus the legend the page draws itself.
struct PageLayout {
    cells: Vec<Placed>,
    /// The legend the page draws in a band of its own, if it draws one.
    legend: Option<HoistedLegend>,
    /// The cell whose legend is drawn elsewhere (or not at all), so it must not
    /// reserve its band or draw it a second time.
    hidden: Option<usize>,
}

/// A legend that the page draws, and the band it got for it.
struct HoistedLegend {
    /// The strip the legend is drawn in.
    band: Rect,
    /// The open area between the page edge and the grid. Wider than `band`, so a
    /// label a little wider than estimated is still drawn in full.
    clip: Rect,
    /// The page area left for the grid.
    grid: Rect,
    /// That cell's canvas, used to move the legend's context panel into page space.
    cell: Rect,
    plan: crate::core::layout::LegendLayoutPlan,
    material: LegendMaterial,
}

/// What a **parent** composition imposes on this page when it solves it.
///
/// Both fields are written into the master cell before the page shares them out,
/// so a parent's decision reaches every cell that should follow it.
#[derive(Clone, Default)]
struct Imposed {
    /// The data rectangle the master cell must adopt.
    panel: Option<Rect>,
    /// The scale the master cell must adopt, per page direction.
    scales: [Option<SharedScale>; 2],
}

impl Imposed {
    /// Nothing imposed: this page is free to lay itself out.
    fn none() -> Self {
        Self::default()
    }
}

/// Who draws the legend of one page.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LegendUse {
    /// The page itself, hoisted onto an edge of its box.
    Page,
    /// A parent composition, which took the legend through
    /// [`CellView::measure`]; no cell here may draw it.
    Elsewhere,
}

/// A positional scale a page agreed on.
///
/// Deliberately opaque to everything outside: it is a resolved domain plus the
/// padding that goes with it, and only [`CellView`] implementations ever build
/// one.
#[derive(Clone)]
pub(crate) struct SharedScale {
    pub(crate) domain: ScaleDomain,
    pub(crate) expand: Expansion,
}

/// Remembers the scale of the largest cell seen so far on a track.
///
/// The main view of a marginal layout is by construction the big one, which is
/// why "largest" is the right reference: the strips adopt what the scatter
/// resolved, never the other way round.
fn prefer_largest(
    specs: &mut Vec<Option<SharedScale>>,
    areas: &mut Vec<f64>,
    index: usize,
    area: f64,
    spec: Option<SharedScale>,
) {
    if specs.len() <= index {
        specs.resize_with(index + 1, || None);
        areas.resize(index + 1, f64::NEG_INFINITY);
    }
    if area > areas[index] {
        areas[index] = area;
        specs[index] = spec;
    }
}

/// A cell after solving: where its canvas goes and what it must keep.
struct Placed {
    rect: Rect,
    /// The view's data rectangle, after the page's sharing and any pin from a
    /// parent. This is what a cell is told to adopt when it draws.
    panel: Rect,
    /// Bottom edge of the cell's own title band (0 when it has none).
    title_bottom: f64,
    /// The guides the cell would draw, if a page may draw them instead.
    legend: Option<LegendMaterial>,
    /// The scale that governs each **page direction**, after sharing.
    scales: [Option<SharedScale>; 2],
    /// Whether the cell pinned those scales itself.
    pinned: [bool; 2],
    /// Index into [`Composition::cells`], so the geometry and the cell itself
    /// can never come from different places.
    index: usize,
}

impl Placed {
    /// How big the cell is on the page: the reference for "the largest cell".
    ///
    /// The **canvas**, not the panel, on purpose. A strip is thin because its
    /// track is [`TrackSize::Fixed`], and that is the signal that says "I am furniture";
    /// its *panel* can be large precisely because it hides the axes a neighbour
    /// draws. Measuring the canvas keeps a page's master cell — the one a parent
    /// aligns and pins — the cell a reader would point at, and it does not depend
    /// on how much furniture each cell happens to draw.
    fn footprint(&self) -> f64 {
        self.rect.width * self.rect.height
    }
}

/// The cell a parent aligns and pins: the page's **largest cell**.
///
/// "Largest" is the cell's canvas, not its panel — see [`Placed::footprint`]. Ties
/// go to the **first** cell, so the choice is reproducible: a parent measuring a
/// nested page and that page solving it must agree on which cell the pin lands on,
/// or the two would disagree about what was aligned.
fn master_of(placed: &[Placed]) -> Option<usize> {
    placed
        .iter()
        .max_by(|a, b| {
            a.footprint()
                .total_cmp(&b.footprint())
                .then(b.index.cmp(&a.index))
        })
        .map(|item| item.index)
}

/// How many tracks a cell covers along `axis`.
const fn span_of(cell: &Cell, axis: Axis) -> usize {
    match axis {
        Axis::Horizontal => cell.col_span,
        Axis::Vertical => cell.row_span,
    }
}

/// Copies the per-direction scales out of a [`CellPlan`].
fn clone_scales(scales: &[Option<SharedScale>; 2]) -> [Option<SharedScale>; 2] {
    [scales[0].clone(), scales[1].clone()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track_extents(sizes: &[TrackSize], total: f64, spacing: f64) -> Vec<f64> {
        Composition::tracks(sizes, total, spacing).1
    }

    #[test]
    fn fixed_tracks_keep_their_size_and_fill_tracks_share_the_rest() {
        let extents = track_extents(
            &[
                TrackSize::fixed(60.0),
                TrackSize::fill(1.0),
                TrackSize::fill(3.0),
            ],
            420.0,
            10.0,
        );
        assert_eq!(extents[0], 60.0);
        // 420 - 2 gaps - 60 fixed = 340, split 1:3.
        assert!((extents[1] - 85.0).abs() < 1e-9, "{extents:?}");
        assert!((extents[2] - 255.0).abs() < 1e-9, "{extents:?}");
    }

    #[test]
    fn a_fill_track_never_goes_negative_when_fixed_tracks_overfill() {
        let extents = track_extents(&[TrackSize::fixed(500.0), TrackSize::fill(1.0)], 200.0, 0.0);
        assert_eq!(extents, vec![500.0, 0.0]);
    }

    #[test]
    fn a_three_by_three_grid_honours_fixed_tracks() {
        // Columns: Fill(1) | Fixed(60) | Fill(1)   -> (500 - 2 gaps - 60) / 2 each
        // Rows:    Fixed(60) | Fill(1) | Fixed(60) -> 400 - 2 gaps - 2 * 60
        let columns = [
            TrackSize::fill(1.0),
            TrackSize::fixed(60.0),
            TrackSize::fill(1.0),
        ];
        let rows = [
            TrackSize::fixed(60.0),
            TrackSize::fill(1.0),
            TrackSize::fixed(60.0),
        ];
        let widths = track_extents(&columns, 500.0, 6.0);
        let heights = track_extents(&rows, 400.0, 6.0);

        assert!((widths[0] - 214.0).abs() < 1e-9, "{widths:?}");
        assert!((widths[1] - 60.0).abs() < 1e-9, "{widths:?}");
        assert!((widths[2] - 214.0).abs() < 1e-9, "{widths:?}");
        assert!((heights[0] - 60.0).abs() < 1e-9, "{heights:?}");
        assert!((heights[1] - 268.0).abs() < 1e-9, "{heights:?}");
        assert!((heights[2] - 60.0).abs() < 1e-9, "{heights:?}");

        // 214 + 6 + 60 + 6 + 214 = 500 and 60 + 6 + 268 + 6 + 60 = 400.
        assert!((widths.iter().sum::<f64>() + 12.0 - 500.0).abs() < 1e-9);
        assert!((heights.iter().sum::<f64>() + 12.0 - 400.0).abs() < 1e-9);
    }

    #[test]
    fn fixed_tracks_are_paid_for_before_the_weights_are_applied() {
        // The order of the tracks does not change the arithmetic, only where the
        // extents land.
        let a = track_extents(
            &[
                TrackSize::fixed(60.0),
                TrackSize::fill(1.0),
                TrackSize::fill(1.0),
            ],
            500.0,
            6.0,
        );
        let b = track_extents(
            &[
                TrackSize::fill(1.0),
                TrackSize::fixed(60.0),
                TrackSize::fill(1.0),
            ],
            500.0,
            6.0,
        );
        assert_eq!(a, vec![60.0, 214.0, 214.0]);
        assert_eq!(b, vec![214.0, 60.0, 214.0]);
    }

    #[test]
    fn a_spanning_cell_covers_the_spacing_between_its_tracks() {
        let starts = vec![0.0, 110.0];
        let extents = vec![100.0, 100.0];
        let (start, extent) = Composition::span_box(&starts, &extents, 10.0, 0, 2);
        assert_eq!(start, 0.0);
        assert_eq!(extent, 210.0);
    }
}
