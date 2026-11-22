//! Drawing a finished chart at a different position: a backend that only shifts
//! coordinates.
//!
//! # The problem it solves
//!
//! Every chart is laid out in **its own canvas**, starting at `(0, 0)`:
//!
//! ```text
//!        the chart's own canvas (500 x 400)          the page (500 x 400)
//!        +------------------------------+            +------------------------------+
//!        | title                        |            |                              |
//!        |   +----------------------+   |            |                              |
//!        |   |      panel           |   |            |  <- a composition wants the  |
//!        |   |                      |   |            |     chart *here*, not at 0,0 |
//!        |   +----------------------+   |            |                              |
//!        +------------------------------+            +------------------------------+
//!```
//!
//! A chart knows nothing about the page it will be placed on, and none of its
//! renderers (marks, axes, legend, title) take an origin argument. Teaching all of
//! them about page offsets would mean touching every renderer and every future
//! one, for a question none of them can answer ("where on the page am I?").
//!
//! # What this file does instead
//!
//! It wraps the drawing target. The chart still believes it is drawing at
//! `(0, 0)`; every coordinate it emits is shifted by `(dx, dy)` on the way out:
//!
//! ```text
//!   chart.render(&mut OffsetBackend::new(page_backend, 320.0, 60.0))
//!
//!   chart emits:  circle at (12, 30)   ->  page receives:  circle at (332, 90)
//!   chart emits:  clip   at (0, 0, w, h) -> page receives: clip   at (320, 60, w, h)
//! ```
//!
//! The result is the very same picture, moved. Nothing is scaled, clipped,
//! rotated or re-laid out: sizes, fonts, colours, group roles and the order of the
//! drawing all pass through untouched.
//!
//! # What has to be shifted
//!
//! Exactly two things:
//!
//! * the **origin** of every primitive — circle centre, rect corner, path points,
//!   text anchor, line ends, gradient corner;
//! * the **clip rectangle** carried by a [`Group`], because a clip lives in the
//!   same coordinate space as the drawing it protects. Its width and height do not
//!   change: the panel still clips the marks, just somewhere else on the page.
//!
//! A group's role and identity are *not* shifted — a mark is a mark wherever it is
//! drawn, and semantic output (SVG classes, PDF tags) must not depend on position.
//!
//! # Where it is used
//!
//! [`Composition`](crate::composition::Composition) draws every cell through one of
//! these, once per cell:
//!
//! ```text
//!   for cell in cells {
//!       cell.chart.render(&mut OffsetBackend::new(backend, cell.x, cell.y))
//!   }
//! ```
//!
//! without it, every cell would have to be pasted into the page by hand.
//!
//! # A note on the inner backend
//!
//! The wrapped backend is a `&mut dyn RenderBackend` (a trait object) rather than a
//! generic type parameter. A generic wrapper would itself be a `RenderBackend`, so
//! the compiler would have to consider `OffsetBackend<OffsetBackend<…>>` chains of
//! unbounded depth and give up (the error is "overflow evaluating the trait
//! bound"). Composition never wraps one offset backend in another — a nested
//! composition is solved in its own coordinates and offset as a whole — so one
//! level of dynamic dispatch is enough, and cheaper to compile.

use crate::Precision;
use crate::coordinate::Rect;
use crate::core::layer::{
    CircleConfig, GradientRectConfig, LineConfig, PathConfig, RectConfig, RenderBackend, TextConfig,
};
use crate::core::semantic::{Element, Group};
use crate::error::ChartonError;

/// A drawing target that moves everything it receives by `(dx, dy)`.
///
/// Create one per cell of a composition:
///
/// ```rust,ignore
/// let mut target = OffsetBackend::new(backend, cell_rect.x, cell_rect.y);
/// chart.render(&mut target)?;          // the chart now sits at cell_rect
/// ```
pub(crate) struct OffsetBackend<'a> {
    /// The backend the shifted drawing is forwarded to.
    inner: &'a mut dyn RenderBackend,
    /// How far right every coordinate moves, in the page's pixels.
    dx: f64,
    /// How far down every coordinate moves, in the page's pixels.
    dy: f64,
}

impl<'a> OffsetBackend<'a> {
    /// Wraps `inner`, so that `(0, 0)` in the drawing becomes `(dx, dy)` on the
    /// page.
    pub(crate) fn new(inner: &'a mut dyn RenderBackend, dx: f64, dy: f64) -> Self {
        Self { inner, dx, dy }
    }

    /// One x coordinate, moved.
    fn shift_x(&self, x: Precision) -> Precision {
        (x as f64 + self.dx) as Precision
    }

    /// One y coordinate, moved.
    fn shift_y(&self, y: Precision) -> Precision {
        (y as f64 + self.dy) as Precision
    }

    /// One rectangle, moved. Its width and height are what they were.
    fn shift_rect(&self, rect: Rect) -> Rect {
        Rect::new(rect.x + self.dx, rect.y + self.dy, rect.width, rect.height)
    }
}

impl RenderBackend for OffsetBackend<'_> {
    fn draw_circle(&mut self, mut config: CircleConfig) {
        config.x = self.shift_x(config.x);
        config.y = self.shift_y(config.y);
        self.inner.draw_circle(config);
    }

    fn draw_rect(&mut self, mut config: RectConfig) {
        config.x = self.shift_x(config.x);
        config.y = self.shift_y(config.y);
        self.inner.draw_rect(config);
    }

    fn draw_path(&mut self, mut config: PathConfig) {
        for subpath in &mut config.subpaths {
            for point in subpath.iter_mut() {
                point.0 = self.shift_x(point.0);
                point.1 = self.shift_y(point.1);
            }
        }
        self.inner.draw_path(config);
    }

    fn draw_text(&mut self, mut config: TextConfig) -> Result<(), ChartonError> {
        config.x = self.shift_x(config.x);
        config.y = self.shift_y(config.y);
        self.inner.draw_text(config)
    }

    fn draw_line(&mut self, mut config: LineConfig) {
        config.x1 = self.shift_x(config.x1);
        config.y1 = self.shift_y(config.y1);
        config.x2 = self.shift_x(config.x2);
        config.y2 = self.shift_y(config.y2);
        self.inner.draw_line(config);
    }

    fn draw_gradient_rect(&mut self, mut config: GradientRectConfig) {
        config.x = self.shift_x(config.x);
        config.y = self.shift_y(config.y);
        self.inner.draw_gradient_rect(config);
    }

    fn begin_group(&mut self, group: &Group<'_>) {
        // Everything about the scope travels unchanged except its clip, which is
        // geometry and therefore belongs to the shifted coordinate space.
        let moved = Group {
            role: group.role,
            layer: group.layer,
            series: group.series,
            mark: group.mark,
            label: group.label,
            clip: group.clip.map(|rect| self.shift_rect(rect)),
        };
        self.inner.begin_group(&moved);
    }

    fn end_group(&mut self) {
        self.inner.end_group();
    }

    fn begin_element(&mut self, element: Element<'_>) {
        self.inner.begin_element(element);
    }

    fn end_element(&mut self) {
        self.inner.end_element();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A backend that remembers what reached it, so the shift can be checked
    /// without going through a document format.
    #[derive(Default)]
    struct Recorder {
        rects: Vec<(f64, f64, f64, f64)>,
        circles: Vec<(f64, f64)>,
        texts: Vec<(f64, f64, String)>,
        clip: Option<Rect>,
    }

    impl RenderBackend for Recorder {
        fn draw_circle(&mut self, config: CircleConfig) {
            self.circles.push((config.x as f64, config.y as f64));
        }
        fn draw_rect(&mut self, config: RectConfig) {
            self.rects.push((
                config.x as f64,
                config.y as f64,
                config.width as f64,
                config.height as f64,
            ));
        }
        fn draw_path(&mut self, _: PathConfig) {}
        fn draw_text(&mut self, config: TextConfig) -> Result<(), ChartonError> {
            self.texts
                .push((config.x as f64, config.y as f64, config.text));
            Ok(())
        }
        fn draw_line(&mut self, _: LineConfig) {}
        fn draw_gradient_rect(&mut self, _: GradientRectConfig) {}
        fn begin_group(&mut self, group: &Group<'_>) {
            if let Some(rect) = group.clip {
                self.clip = Some(rect);
            }
        }
    }

    fn rect_at(x: f32, y: f32) -> RectConfig {
        RectConfig {
            x,
            y,
            width: 10.0,
            height: 5.0,
            fill: crate::visual::color::SingleColor::new("black"),
            stroke: crate::visual::color::SingleColor::none(),
            stroke_width: 0.0,
            opacity: 1.0,
        }
    }

    #[test]
    fn primitives_move_by_the_offset_and_keep_their_size() {
        let mut recorder = Recorder::default();
        {
            let mut page = OffsetBackend::new(&mut recorder, 100.0, 40.0);
            page.draw_rect(rect_at(5.0, 6.0));
        }
        assert_eq!(recorder.rects, vec![(105.0, 46.0, 10.0, 5.0)]);
    }

    #[test]
    fn a_group_keeps_its_identity_and_moves_its_clip() {
        let mut recorder = Recorder::default();
        {
            let mut page = OffsetBackend::new(&mut recorder, 100.0, 40.0);
            page.begin_group(
                &Group::new(crate::core::semantic::Role::Plot)
                    .clip(Rect::new(1.0, 2.0, 30.0, 20.0)),
            );
            page.end_group();
        }
        let clip = recorder.clip.expect("the clip is forwarded, not dropped");
        assert_eq!(
            (clip.x, clip.y, clip.width, clip.height),
            (101.0, 42.0, 30.0, 20.0),
            "the clip moves with the drawing; its size is unchanged"
        );
    }

    #[test]
    fn an_offset_of_zero_changes_nothing() {
        let mut recorder = Recorder::default();
        {
            let mut page = OffsetBackend::new(&mut recorder, 0.0, 0.0);
            page.draw_rect(rect_at(5.0, 6.0));
        }
        assert_eq!(recorder.rects, vec![(5.0, 6.0, 10.0, 5.0)]);
    }
}
