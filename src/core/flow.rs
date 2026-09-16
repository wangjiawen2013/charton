//! One-dimensional "flex-wrap" packing, shared by every level of the legend.
//!
//! A legend is always a sequence of boxes laid out along one axis, spilling
//! onto a new line when the current one runs out of room: the entries inside a
//! single guide, and the guide blocks inside the legend strip. Only the
//! *direction* differs between legend positions, never the algorithm -- so the
//! algorithm lives here once, written in terms of a packing frame
//! (`main` / `cross`) instead of `x` / `y`:
//!
//! ```text
//!                    main ──────────────►
//!              ───────┬───────┬───────┐
//!      cross   │   A   │   B   │   C   │
//!        │     ├───────┴───────┴───────┤   ← wrap: advances by the thickest
//!        ▼     │   D   │   E   │       │     box of the previous line
//!              └───────┴──────────────┘
//! ```
//!
//! [`Direction::Horizontal`] maps `main → x` / `cross → y` (legends on the top
//! and bottom edges), [`Direction::Vertical`] transposes them (left and right
//! edges).
//!
//! # Vocabulary
//!
//! - **box** -- one rectangle to be placed. The packer never looks inside a box,
//!   it only knows the two numbers in [`Extent`]. What a box *contains* therefore
//!   depends on which level is being packed:
//!   - at the **entry** level a box is one legend entry: the symbol cell, the gap
//!     after the symbol, and the label text. It does *not* include the gap that
//!     separates it from the next entry (that is `gap`), and it does *not* include
//!     the block title (the title is added around the entries by
//!     `GuideSpec::measure`).
//!   - at the **block** level a box is one whole guide: its title *and* all of its
//!     entries. Unlike an entry, a block box does contain a title.
//!
//!   Either way a box is "content only": the spacing between boxes is always
//!   added by the packer, never baked into a box.
//! - **run** -- the whole sequence of boxes passed to [`pack`] in one call. The
//!   result describes that run's bounding rectangle.
//! - **line** -- one row of boxes along the packing direction: a *row* when
//!   packing horizontally, a *column* when packing vertically. The code says
//!   "line" because it does not care which of the two it is.
//! - **packing direction** -- the axis boxes advance along: left to right
//!   ([`Direction::Horizontal`]) or top to bottom ([`Direction::Vertical`]). It is
//!   picked by the legend position, see [`Direction::for_legend`].
//! - **`main`** -- the box's size along the packing direction: its width when the
//!   packing direction is horizontal, its height when it is vertical. It decides
//!   where the next box of the line starts, and whether the box fits on the line at
//!   all.
//! - **`cross`** -- the box's size across the packing direction: its height when the
//!   packing direction is horizontal, its width when it is vertical. Used only when
//!   a line wraps: the next line starts past the *biggest* box of the current line,
//!   never past the last one. [`Extent`] has the picture.
//! - **`cursor`** -- the running position along the packing direction at which the
//!   *next* box would be placed, measured from the start of the current line. It is
//!   therefore both "where the next box begins" and "how much of this line is used
//!   so far".
//! - **`budget`** -- how much room is available along the packing direction
//!   (`main_budget`) before a line has to wrap. It is the length of the container
//!   the run must fit into, so a box fits only while its far edge `cursor + main`
//!   stays within the budget. For a legend the container is the plot panel, so the
//!   budget is the panel's extent along the packing direction, minus the block
//!   title where the title occupies the packing axis.
//! - **`gap`** -- spacing inserted between two boxes on the same line.
//! - **`line_gap`** -- spacing inserted between two lines.

/// The axis a run of boxes is packed along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    /// Boxes advance left to right and spill onto a new row below.
    Horizontal,
    /// Boxes advance top to bottom and spill onto a new column to the right.
    Vertical,
}

impl Direction {
    /// The packing direction used by a legend on the given edge.
    ///
    /// Top/bottom legends form a horizontal strip, left/right legends form a
    /// vertical one.
    pub(crate) const fn for_legend(edge: crate::core::guide::LegendPosition) -> Self {
        use crate::core::guide::LegendPosition::{Bottom, Left, None, Right, Top};
        match edge {
            Top | Bottom => Direction::Horizontal,
            // `None` never reaches the packer, but it has to answer something.
            Left | Right | None => Direction::Vertical,
        }
    }

    /// Re-expresses a physical size in this direction's packing frame.
    pub(crate) const fn measure(self, width: f64, height: f64) -> Extent {
        match self {
            Direction::Horizontal => Extent {
                main: width,
                cross: height,
            },
            Direction::Vertical => Extent {
                main: height,
                cross: width,
            },
        }
    }

    /// Converts a packing-frame pair back into physical `(x, y)`.
    ///
    /// The mapping is a plain transpose, so this also turns a
    /// `(main_total, cross_total)` pair back into `(width, height)`.
    pub(crate) const fn resolve(self, main: f64, cross: f64) -> (f64, f64) {
        match self {
            Direction::Horizontal => (main, cross),
            Direction::Vertical => (cross, main),
        }
    }
}

/// One box's size, expressed in a packing frame instead of `(width, height)`.
///
/// See the [module documentation](self) for what a "box" contains at each level
/// and for the meaning of `main` / `cross`. It is not simply `(width, height)`
/// because the same physical box has to be described differently depending on
/// which way the run is packed -- the two directions answer two different
/// questions:
///
/// ```text
///                            Direction::Horizontal    Direction::Vertical
///   main  (advance)                width                    height
///   cross (thickness)             height                   width
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Extent {
    /// The box's size *along* the packing direction: its width when the boxes
    /// advance left to right, its height when they advance top to bottom.
    ///
    /// Think of it as the box's advance -- the distance the cursor moves forward
    /// *because of this box*. The gap that follows the box is a separate
    /// contribution, added by the packer.
    ///
    /// It answers two questions:
    ///
    /// - Does the box still fit? Its far edge is `cursor + main`, and it fits
    ///   while that stays inside the budget. This is the *only* thing that decides
    ///   whether a line wraps.
    /// - Where does the next box on this line begin? At `cursor + main + gap`.
    pub(crate) main: f64,
    /// The box's size *across* the packing direction: its height when the boxes
    /// advance left to right, its width when they advance top to bottom.
    ///
    /// One thing and one thing only uses this number: deciding where the *next*
    /// line starts. It never decides where a box goes inside its own line, nor
    /// whether it fits there -- `main` does both.
    ///
    /// Boxes on a line are laid side by side, so they all share the same distance
    /// across the packing direction, and the next line has to clear the box that
    /// reaches furthest -- the biggest one, not the last one:
    ///
    /// ```text
    ///   │   A  ││ B  │     A is 3 units across the packing direction,
    ///   │      │└───┘      B is only 1
    ///   └──────┘           so the line reaches 3, not 1
    ///   ┌───────────┐
    ///   │ next line │      and the next line starts 3 + line_gap from there
    ///   └───────────┘
    /// ```
    ///
    /// Measuring to the *last* box (B, which reaches 1) would start the next line
    /// at `1 + line_gap`, right inside A -- the two lines would overlap. That is
    /// why a line is as thick as its `max(cross)`, not as its last box.
    pub(crate) cross: f64,
}

/// Where [`pack`] placed the boxes, plus the size of the whole run.
#[derive(Clone, Debug, Default)]
pub(crate) struct FlowResult {
    /// `(main, cross)` offset of every box, in the same order as the input and
    /// relative to the **top-left corner of the run** -- not to the canvas. Use
    /// [`Direction::resolve`] to turn such a pair into an `(x, y)` offset.
    ///
    /// The offset locates the box's top-left corner, never its centre.
    pub(crate) offsets: Vec<(f64, f64)>,
    /// Total length of the run along the packing direction: the length of its
    /// longest line. Every line starts at 0, so this bounds every box.
    pub(crate) main_total: f64,
    /// Total length of the run across the packing direction: the end of the last
    /// line, with the gaps between lines included.
    pub(crate) cross_total: f64,
}

/// Packs a run of boxes into lines, wrapping when the current line is full.
///
/// This is the only wrapping implementation in the crate. A legend calls it
/// twice -- once with the entries of each guide, once with the guide blocks -- so
/// both levels get identical behaviour for free.
///
/// # Parameters
///
/// - `extents` -- the boxes, in the order they should be placed. The order is the
///   reading order of the legend.
/// - `main_budget` -- the **budget**: how far the run may extend along the packing
///   direction before a line has to wrap. It is a *soft* limit. A box that is by
///   itself longer than the budget is still placed on a line of its own, because
///   there is nowhere else for it to go; callers that must not overflow have to
///   check [`FlowResult::main_total`] against their budget afterwards.
/// - `gap` -- spacing between two boxes on the same line.
/// - `line_gap` -- spacing between two lines.
///
/// # How a box is placed
///
/// The **cursor** starts at 0 on every line and always holds the position where
/// the next box would begin, so it doubles as "how much of this line is used so
/// far". Adding the box's own length gives the position of its far edge:
///
/// ```text
///   0              cursor           cursor + main
///   |----------------|--------------------|
///   |  boxes so far  |        main        |
///   |----------------|--------------------|
///                                      budget
///   placed while cursor + main <= budget      (fits)
///   wraps  while cursor + main >  budget      (far edge would stick out)
/// ```
///
/// Only `main` takes part in that comparison; `cross` never decides whether a box
/// fits.
///
/// # How a line wraps
///
/// On wrap, the cursor goes back to 0 and `line_start` -- the position of the
/// current line across the packing direction -- moves past the line that was just
/// finished, plus `line_gap`. "Past the line" means past the box that reaches
/// furthest across, not past the last box placed: boxes on a line sit side by
/// side, so a big box early in the line and a small box after it still share the
/// same band, and the next line has to clear the big one. Measuring to the last
/// box would draw the next line on top of it.
///
/// A line is only ever wrapped away from once something has been placed on it, so
/// an oversized box cannot produce an endless series of empty lines.
///
/// # Returns
///
/// The offset of each box relative to the run's top-left corner, plus the run's
/// total size. See [`FlowResult`].
pub(crate) fn pack(extents: &[Extent], main_budget: f64, gap: f64, line_gap: f64) -> FlowResult {
    let mut offsets = Vec::with_capacity(extents.len());
    // Position along the packing direction where the next box of the current line
    // begins. Because it is advanced by `main + gap`, it already includes the gap
    // that follows the last placed box, which is why `cursor + main` below is that
    // box's far edge and not its far edge plus a gap.
    let mut cursor = 0.0;
    // Position of the current line across the packing direction. This is the only
    // value a wrap changes, and the only place a box's `cross` has any effect.
    let mut line_start = 0.0;
    // Reaches furthest across the packing direction on the current line so far.
    // The line after this one starts past it, not past the last box placed.
    let mut line_thickness = 0.0;
    // Longest line seen, i.e. the run's extent along the packing direction.
    let mut main_total: f64 = 0.0;
    // End of the lowest line seen, i.e. the run's extent across it.
    let mut cross_total: f64 = 0.0;

    for extent in extents {
        // The box fits while its far edge stays inside the budget. Wrapping is only
        // allowed once this line holds something, so that a box which is oversized
        // all by itself still gets placed (and simply overflows) instead of
        // spawning empty lines forever.
        if cursor > 0.0 && cursor + extent.main > main_budget {
            // Push the new line out beyond the thickest box of the line just
            // finished, not beyond its last box.
            line_start += line_thickness + line_gap;
            cursor = 0.0;
            line_thickness = 0.0;
        }

        // Top-left corner of the box within the run.
        offsets.push((cursor, line_start));

        // Advance by the box plus the gap after it. The gap is added
        // unconditionally and subtracted again when measuring the line's length,
        // which keeps this loop free of "am I the last one?" tests.
        cursor += extent.main + gap;
        line_thickness = line_thickness.max(extent.cross);
        main_total = main_total.max(cursor - gap);
        cross_total = cross_total.max(line_start + line_thickness);
    }

    FlowResult {
        offsets,
        main_total,
        cross_total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extent(main: f64, cross: f64) -> Extent {
        Extent { main, cross }
    }

    #[test]
    fn packs_everything_on_one_line_when_it_fits() {
        // 30 + 4 + 30 = 64, well inside the budget.
        let result = pack(&[extent(30.0, 10.0), extent(30.0, 10.0)], 100.0, 4.0, 8.0);

        assert_eq!(result.offsets, vec![(0.0, 0.0), (34.0, 0.0)]);
        assert_eq!(result.main_total, 64.0);
        assert_eq!(result.cross_total, 10.0);
    }

    #[test]
    fn wraps_using_the_thickest_box_of_the_previous_line() {
        // Box A is taller than B, so C must start below A, not below B.
        let result = pack(
            &[extent(60.0, 40.0), extent(20.0, 10.0), extent(60.0, 10.0)],
            100.0,
            0.0,
            5.0,
        );

        assert_eq!(result.offsets[2], (0.0, 45.0));
        assert_eq!(result.cross_total, 55.0);
    }

    #[test]
    fn places_an_oversized_box_on_its_own_line() {
        // The budget is 50 but the box is 80 wide: it still gets placed.
        let result = pack(&[extent(80.0, 10.0)], 50.0, 0.0, 0.0);

        assert_eq!(result.offsets, vec![(0.0, 0.0)]);
        assert_eq!(result.main_total, 80.0);
    }

    #[test]
    fn empty_input_produces_an_empty_run() {
        let result = pack(&[], 100.0, 4.0, 8.0);

        assert!(result.offsets.is_empty());
        assert_eq!((result.main_total, result.cross_total), (0.0, 0.0));
    }

    #[test]
    fn does_not_count_the_trailing_gap() {
        // Two boxes with a gap that must not be added after the last one.
        let result = pack(&[extent(10.0, 1.0), extent(10.0, 1.0)], 100.0, 5.0, 5.0);

        assert_eq!(result.main_total, 25.0);
    }
}
