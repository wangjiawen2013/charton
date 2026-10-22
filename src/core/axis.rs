//! The physical axes of a panel.
//!
//! One axis is described by a single [`AxisView`], built from the coordinate
//! system, the panel rectangle, the caller's explicit ticks and the theme. It
//! answers every question the rest of the crate asks about that axis:
//!
//! * **what it shows** — which data scale sits on this side (the `x` scale on
//!   the bottom and the `y` scale on the left, swapped by `coord_flip`), its
//!   title, its label rotation, and the ticks it draws;
//! * **where it sits** — [`AxisView::tick_point`] places a tick on the spine and
//!   [`AxisSide::outward`] says which way its band grows;
//! * **how deep it is** — [`AxisView::band`] gives the offsets and the total
//!   depth that the layout engine reserves.
//!
//! Because the layout engine reads `band().depth` from the same object the
//! renderers draw from, the reserved strip and the painted axis are guaranteed
//! to agree. Keeping the side mapping here also means the flip rule and the
//! explicit-tick precedence are implemented once instead of in every renderer.
//!
//! How *many* ticks an axis gets is a separate concern owned by
//! [`crate::core::ticks`], which needs the panel length and the label size but
//! never the physical side. Views are cheap and stateless: building one
//! resolves at most a dozen labels, so passes simply rebuild them rather than
//! cache state that could go stale.

use crate::coordinate::{CoordinateTrait, Rect};
use crate::core::ticks::{AxisDirection, label_box, resolve_ticks};
use crate::scale::{ExplicitTick, ScaleTrait, Tick};
use crate::theme::Theme;

/// Extra room between the outermost text and the canvas edge.
///
/// The canvas margins provide the real breathing room; this is only a small
/// safety strip so a text estimate that comes out a hair short does not clip.
const EDGE_BUFFER: f64 = 4.0;

/// The two physical sides of a panel that can carry a value axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AxisSide {
    /// The horizontal axis below the panel.
    Bottom,
    /// The vertical axis to the left of the panel.
    Left,
}

impl AxisSide {
    /// Which way the axis runs on the canvas.
    pub(crate) const fn direction(self) -> AxisDirection {
        match self {
            Self::Bottom => AxisDirection::Horizontal,
            Self::Left => AxisDirection::Vertical,
        }
    }

    /// The panel length the labels have to be spaced along.
    pub(crate) const fn length(self, panel: &Rect) -> f64 {
        match self {
            Self::Bottom => panel.width,
            Self::Left => panel.height,
        }
    }

    /// Whether this side shows the `y` data scale.
    ///
    /// The bottom carries `x` and the left carries `y`, until `coord_flip`
    /// swaps them. Keeping the rule here means no renderer has to spell out the
    /// flip again.
    pub(crate) const fn carries_y(self, flipped: bool) -> bool {
        match self {
            Self::Bottom => flipped,
            Self::Left => !flipped,
        }
    }

    /// The unit vector pointing away from the panel, i.e. into the band.
    pub(crate) const fn outward(self) -> (f64, f64) {
        match self {
            Self::Bottom => (0.0, 1.0),
            Self::Left => (-1.0, 0.0),
        }
    }
}

/// A resolved physical axis: the scale it shows, the ticks it draws, and where
/// those sit relative to the panel.
pub(crate) struct AxisView<'a> {
    side: AxisSide,
    panel: &'a Rect,
    theme: &'a Theme,
    scale: &'a dyn ScaleTrait,
    angle_deg: f64,
    title: &'a str,
    ticks: Vec<Tick>,
}

impl<'a> AxisView<'a> {
    /// Resolves one physical axis of `panel`.
    ///
    /// `x_explicit` / `y_explicit` are the caller's explicit tick values,
    /// addressed by *data* axis; this is the only place that knows how a flip
    /// maps them onto physical sides.
    pub(crate) fn new(
        coord: &'a dyn CoordinateTrait,
        side: AxisSide,
        panel: &'a Rect,
        x_explicit: Option<&'a [ExplicitTick]>,
        y_explicit: Option<&'a [ExplicitTick]>,
        theme: &'a Theme,
    ) -> Self {
        let (scale, angle_deg, title, explicit) = if side.carries_y(coord.is_flipped()) {
            (
                coord.get_y_scale(),
                theme.y_tick_label_angle,
                coord.get_y_label(),
                y_explicit,
            )
        } else {
            (
                coord.get_x_scale(),
                theme.x_tick_label_angle,
                coord.get_x_label(),
                x_explicit,
            )
        };

        let ticks = resolve_ticks(
            scale,
            explicit,
            side.length(panel),
            side.direction(),
            angle_deg,
            theme,
        );

        Self {
            side,
            panel,
            theme,
            scale,
            angle_deg,
            title,
            ticks,
        }
    }

    pub(crate) const fn side(&self) -> AxisSide {
        self.side
    }

    pub(crate) const fn panel(&self) -> &Rect {
        self.panel
    }

    pub(crate) const fn theme(&self) -> &Theme {
        self.theme
    }

    /// The data scale projected onto this physical side.
    pub(crate) const fn scale(&self) -> &'a dyn ScaleTrait {
        self.scale
    }

    /// Rotation applied to the labels, in degrees.
    pub(crate) const fn angle_deg(&self) -> f64 {
        self.angle_deg
    }

    /// The axis title, empty when the coordinate system has none.
    pub(crate) const fn title(&self) -> &'a str {
        self.title
    }

    /// The ticks this axis draws, already respecting explicit values.
    pub(crate) fn ticks(&self) -> &[Tick] {
        &self.ticks
    }

    /// Where a tick sits on the axis spine, in screen space.
    ///
    /// `norm` is the scale's normalized value. This is the one place that knows
    /// the bottom axis runs left-to-right at the panel's baseline and the left
    /// axis runs bottom-to-top at the panel's left edge.
    pub(crate) fn tick_point(&self, norm: f64) -> (f64, f64) {
        match self.side {
            AxisSide::Bottom => (
                self.panel.x + norm * self.panel.width,
                self.panel.y + self.panel.height,
            ),
            AxisSide::Left => (
                self.panel.x,
                self.panel.y + (1.0 - norm) * self.panel.height,
            ),
        }
    }

    /// How the axis band is laid out, measured from the axis spine outwards.
    pub(crate) fn band(&self) -> AxisBand {
        let direction = self.side.direction();
        let label_extent = self
            .ticks
            .iter()
            .map(|tick| {
                direction.across(
                    &label_box(&tick.label, self.theme.tick_label_size),
                    self.angle_deg,
                )
            })
            .fold(0.0, f64::max);

        let label_offset = self.theme.tick_length + self.theme.tick_label_padding;
        let has_title = !self.title.is_empty();
        let title_height = if has_title {
            self.theme.label_size
        } else {
            0.0
        };
        let title_offset = label_offset
            + label_extent
            + if has_title {
                self.theme.label_padding
            } else {
                0.0
            };

        AxisBand {
            label_offset,
            title_offset,
            title_height,
            // The title offset already includes `label_padding` when there is a
            // title, so the depth is a single summation for both cases.
            depth: title_offset + title_height + EDGE_BUFFER,
        }
    }
}

/// The depth of an axis band and the offsets of the things inside it.
///
/// All offsets are measured from the axis spine (the panel edge) outwards, so
/// the same numbers place the labels/title and size the reserved band. Sharing
/// them is what keeps the measurement and the drawing from drifting apart.
pub(crate) struct AxisBand {
    /// Spine to the label anchor (past the tick mark and its padding).
    pub(crate) label_offset: f64,
    /// Spine to the near edge of the title band (equal to the label's far edge
    /// plus padding when there is a title).
    pub(crate) title_offset: f64,
    /// Title depth; zero when the axis has no title.
    pub(crate) title_height: f64,
    /// Total depth to reserve, edge buffer included.
    pub(crate) depth: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinate::cartesian::Cartesian2D;
    use crate::scale::{Expansion, Scale, ScaleDomain, create_scale};

    fn coord(flipped: bool) -> Cartesian2D {
        let x = create_scale(
            &Scale::Linear,
            ScaleDomain::Continuous(0.0, 10.0),
            Expansion {
                mult: (0.0, 0.0),
                add: (0.0, 0.0),
            },
            None,
        )
        .unwrap();
        let y = create_scale(
            &Scale::Linear,
            ScaleDomain::Continuous(0.0, 100.0),
            Expansion {
                mult: (0.0, 0.0),
                add: (0.0, 0.0),
            },
            None,
        )
        .unwrap();
        Cartesian2D::new(x, y, "x".into(), "y".into(), flipped)
    }

    fn panel() -> Rect {
        Rect::new(0.0, 0.0, 800.0, 500.0)
    }

    /// `coord_flip` moves each data axis to the other physical side, and its
    /// label rotation and title travel with it.
    #[test]
    fn a_flip_moves_each_data_axis_to_the_other_side() {
        let panel = panel();
        let theme = Theme::default()
            .with_x_tick_label_angle(30.0)
            .with_y_tick_label_angle(-40.0);

        let normal = coord(false);
        let bottom = AxisView::new(&normal, AxisSide::Bottom, &panel, None, None, &theme);
        let left = AxisView::new(&normal, AxisSide::Left, &panel, None, None, &theme);
        assert_eq!(
            bottom.scale().domain(),
            (0.0, 10.0),
            "bottom is the x scale"
        );
        assert_eq!(bottom.title(), "x");
        assert_eq!(bottom.angle_deg(), 30.0);
        assert_eq!(left.scale().domain(), (0.0, 100.0), "left is the y scale");
        assert_eq!(left.title(), "y");
        assert_eq!(left.angle_deg(), -40.0);

        let flipped = coord(true);
        let bottom = AxisView::new(&flipped, AxisSide::Bottom, &panel, None, None, &theme);
        let left = AxisView::new(&flipped, AxisSide::Left, &panel, None, None, &theme);
        assert_eq!(
            bottom.scale.domain(),
            (0.0, 100.0),
            "flipped: y on the bottom"
        );
        assert_eq!(bottom.title(), "y");
        assert_eq!(bottom.angle_deg(), -40.0);
        assert_eq!(left.scale().domain(), (0.0, 10.0), "flipped: x on the left");
        assert_eq!(left.title(), "x");
        assert_eq!(left.angle_deg(), 30.0);
    }

    /// Explicit ticks are addressed by data axis and follow the same flip.
    #[test]
    fn explicit_ticks_travel_with_their_data_axis() {
        let panel = panel();
        let theme = Theme::default();
        let x_ticks = vec![ExplicitTick::Continuous(2.5)];
        let y_ticks = vec![ExplicitTick::Continuous(75.0)];

        let flipped = coord(true);
        let bottom = AxisView::new(
            &flipped,
            AxisSide::Bottom,
            &panel,
            Some(&x_ticks),
            Some(&y_ticks),
            &theme,
        );
        let left = AxisView::new(
            &flipped,
            AxisSide::Left,
            &panel,
            Some(&x_ticks),
            Some(&y_ticks),
            &theme,
        );

        assert_eq!(bottom.ticks().len(), 1);
        assert_eq!(
            bottom.ticks()[0].value,
            75.0,
            "flipped: y ticks on the bottom"
        );
        assert_eq!(left.ticks().len(), 1);
        assert_eq!(left.ticks()[0].value, 2.5, "flipped: x ticks on the left");
    }

    /// The band is one summation: label anchor, label box, title, buffer.
    #[test]
    fn the_band_is_one_summation() {
        let theme = Theme::default();
        let panel = panel();
        let c = coord(false);
        let view = AxisView::new(&c, AxisSide::Bottom, &panel, None, None, &theme);
        let band = view.band();

        assert_eq!(
            band.label_offset,
            theme.tick_length + theme.tick_label_padding
        );
        assert_eq!(band.title_height, theme.label_size);
        assert_eq!(
            band.depth,
            band.title_offset + band.title_height + EDGE_BUFFER
        );
        assert!(band.title_offset > band.label_offset);
    }

    /// Long labels deepen the band, so they cannot be clipped.
    #[test]
    fn long_labels_deepen_the_band() {
        let theme = Theme::default();
        let panel = panel();
        let c = coord(false);
        let short = vec![ExplicitTick::Labeled(5.0, "5".into())];
        let long = vec![ExplicitTick::Labeled(5.0, "five and a half units".into())];

        let short = AxisView::new(&c, AxisSide::Left, &panel, None, Some(&short), &theme).band();
        let long = AxisView::new(&c, AxisSide::Left, &panel, None, Some(&long), &theme).band();

        assert!(
            long.depth > short.depth,
            "the long label needs a deeper band ({} vs {})",
            long.depth,
            short.depth
        );
    }
}
