//! Choosing how many ticks an axis draws.
//!
//! A scale can turn "about this many ticks" into human-friendly values
//! (`1, 2, 5, 10, ...`), but it cannot know how much room the rendered text
//! needs. That decision lives here. The resolver takes the scale, the panel
//! length, the label rotation and the theme's label size, and returns the tick
//! list the axis should draw.
//!
//! # The automatic rule
//!
//! Used when the theme has no pinned count:
//!
//! 1. **Length budget.** One label per [`SPACING_IN_FONTS`] font sizes gives a
//!    target count, clamped to [`MIN_HINT_TICKS`]`..=`[`MAX_HINT_TICKS`]. The
//!    clamp keeps a large canvas from turning into a wall of labels: text keeps
//!    its physical size, so past a point more labels only add clutter.
//! 2. **Snap to the scale's ladder.** The target is handed to the scale, which
//!    returns its own nice values (`1/2/5` steps, calendar steps, powers of
//!    ten). Because it rounds, the list can be slightly denser than requested.
//! 3. **Measure and back off.** The labels that were really produced are
//!    compared as oriented text boxes. Labels on the same line need
//!    [`LABEL_GAP_IN_FONTS`] font sizes between them; rotated neighbours only
//!    need their parallel text lines to clear each other, because the diagonal
//!    offset already separates the glyphs. If a pair does not fit, a continuous
//!    scale is asked for fewer ticks, which moves it to its next coarser step,
//!    and a categorical scale keeps every `n`th category. The first list that
//!    fits wins.
//!
//! If even the coarsest continuous step (or the sparsest categorical subset)
//! does not fit, only the two end labels are kept: the axis still reads as a
//! range, and the resolver never invents an interval outside the scale's
//! ladder.
//!
//! # Overrides
//!
//! Explicit tick values are drawn as given. A pinned count
//! ([`Theme::with_tick_count`](crate::theme::Theme::with_tick_count)) is passed
//! to the scale unchanged and skips the label-fit pass: the caller owns the
//! density, including the risk of overlap on a narrow panel.
//!
//! This module only decides *how many* ticks and where their labels would
//! collide. The physical side, the title and the band belong to
//! [`crate::core::axis`]; the label rotation and size arrive as plain inputs,
//! because they change how much room a label needs but not whether it is the
//! bottom or the left axis.

use crate::core::utils::{estimate_text_height, estimate_text_width};
use crate::scale::{ExplicitTick, Scale, ScaleTrait, Tick};
use crate::theme::Theme;

/// The fewest ticks the length budget asks for.
///
/// Two ticks would only label the ends of the axis, which reads as a scale
/// rather than a chart, so the hint starts at three.
const MIN_HINT_TICKS: usize = 3;

/// The fewest ticks the search asks the scale for.
///
/// One below [`MIN_HINT_TICKS`], because a scale turns a count into a *step*,
/// not into exactly that many ticks: asking for two is how the coarsest useful
/// step is requested (`range / 2` snapped down to `1/2/5`), and that step often
/// still yields three labels.
const MIN_SEARCH_TICKS: usize = 2;

/// The most ticks the length budget ever asks for, however long the axis is.
///
/// Without a cap, a 2000-pixel axis would ask for ~40 labels at the default
/// font size, which is a wall of text rather than a scale. Ten matches the
/// upper end of mainstream defaults (D3's `ticks` asks for ten, Matplotlib's
/// auto locator tops out around nine); twelve was a little too dense.
const MAX_HINT_TICKS: usize = 10;

/// Room one label wants along the axis, in multiples of the tick font size.
///
/// This is the "comfortable" density: four font sizes between neighbours looks
/// neither sparse nor cramped for both short numbers and rotated labels.
const SPACING_IN_FONTS: f64 = 4.0;

/// Gap between two labels that continue along the same text line, in multiples
/// of the tick font size.
///
/// This is the horizontal-label case: one em is roughly two lowercase letters
/// (at ~0.55em each), so neighbouring labels stay visibly separate instead of
/// reading as one long string.
const LABEL_GAP_IN_FONTS: f64 = 1.0;

/// Gap between two parallel lines of rotated text, in multiples of the tick
/// font size.
///
/// Rotated labels are not concatenated end to end: each sits on its own
/// parallel line, and the diagonal offset already separates the glyphs. The
/// line box is a full em, but the visible ink is smaller, so only this small
/// clearance is needed between two ink bands.
const LINE_GAP_IN_FONTS: f64 = 0.1;

/// Which way the labels run, for measuring a label box against the axis.
///
/// Density decisions only need the axis vector: a horizontal axis advances in
/// x, a vertical one in y. Which physical side of the panel the axis sits on is
/// a different question, owned by [`crate::core::axis`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AxisDirection {
    /// Left to right: the bottom axis of a standard chart, or an angular axis.
    Horizontal,
    /// Bottom to top: the left axis of a standard chart.
    Vertical,
}

impl AxisDirection {
    /// The unit vector the axis advances along, in screen space.
    const fn unit(self) -> (f64, f64) {
        match self {
            Self::Horizontal => (1.0, 0.0),
            Self::Vertical => (0.0, 1.0),
        }
    }

    /// The label's extent perpendicular to the axis.
    ///
    /// This is the side of the rotated line box that an axis band has to
    /// reserve for the text. It deliberately uses the full line height, not the
    /// smaller ink height used for collision checks, so descenders are never
    /// clipped.
    pub(crate) fn across(self, label: &LabelBox, angle_deg: f64) -> f64 {
        let (sin, cos) = angle_deg.to_radians().sin_cos();
        match self {
            Self::Horizontal => label.width * sin.abs() + label.line_height * cos.abs(),
            Self::Vertical => label.width * cos.abs() + label.line_height * sin.abs(),
        }
    }
}

/// The box a rendered label occupies.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LabelBox {
    /// Advance width of the text.
    pub(crate) width: f64,
    /// Full em line box, used when reserving axis depth.
    pub(crate) line_height: f64,
    /// Visible glyph height, used when checking rotated lines for collisions.
    pub(crate) ink_height: f64,
}

/// Estimates the box a label occupies, without a backend to measure it.
///
/// The line height is the font size (an em), a little taller than the visible
/// glyphs, so the axis band never clips a descender. The ink height is the
/// smaller extent that actually decides whether two parallel lines of rotated
/// text touch.
pub(crate) fn label_box(text: &str, font_size: f64) -> LabelBox {
    LabelBox {
        width: estimate_text_width(text, font_size),
        line_height: font_size,
        ink_height: estimate_text_height(text, font_size),
    }
}

/// The geometry used to decide whether neighbouring labels collide.
#[derive(Clone, Copy, Debug)]
struct LabelFit {
    direction: AxisDirection,
    angle_deg: f64,
    font_size: f64,
}

impl LabelFit {
    /// Builds the collision geometry for one axis and label rotation.
    const fn new(direction: AxisDirection, angle_deg: f64, font_size: f64) -> Self {
        Self {
            direction,
            angle_deg,
            font_size,
        }
    }

    /// Whether every neighbouring pair is separated.
    ///
    /// Two labels fit when either their text runs end to end with enough gap
    /// (the horizontal case) or their parallel baselines are far enough apart
    /// (the rotated case). Testing both is what lets a 45-degree date label sit
    /// closer along the axis than its own width without being thinned: its long
    /// dimension is not what would hit the neighbour.
    fn fits(&self, scale: &dyn ScaleTrait, ticks: &[Tick], available: f64) -> bool {
        if ticks.len() <= 1 {
            return true;
        }

        let (sin, cos) = self.angle_deg.to_radians().sin_cos();
        let along_text = (cos, sin);
        let across_text = (-sin, cos);
        let (axis_x, axis_y) = self.direction.unit();
        let same_line_gap = self.font_size * LABEL_GAP_IN_FONTS;
        let line_gap = self.font_size * LINE_GAP_IN_FONTS;

        ticks.windows(2).all(|pair| {
            let distance = tick_gap(scale, &pair[0], &pair[1], available);
            let (dx, dy) = (axis_x * distance, axis_y * distance);
            let along = (dx * along_text.0 + dy * along_text.1).abs();
            let across = (dx * across_text.0 + dy * across_text.1).abs();

            let first = label_box(&pair[0].label, self.font_size);
            let second = label_box(&pair[1].label, self.font_size);
            let same_line_clear = along >= first.width.max(second.width) + same_line_gap;
            let parallel_clear = across >= first.ink_height.max(second.ink_height) + line_gap;

            same_line_clear || parallel_clear
        })
    }
}

/// How many ticks the panel length alone would allow.
///
/// This is the length-driven half of the automatic rule: one label per
/// [`SPACING_IN_FONTS`] font sizes, clamped to
/// [`MIN_HINT_TICKS`]`..=`[`MAX_HINT_TICKS`]. The final count comes from
/// [`resolve_ticks`], which also measures the labels and may ask for fewer.
fn count_hint(available_pixels: f64, font_size: f64) -> usize {
    let comfortable = (font_size * SPACING_IN_FONTS).max(16.0);
    if !available_pixels.is_finite() || available_pixels <= 0.0 || !comfortable.is_finite() {
        return MIN_HINT_TICKS;
    }
    let by_space = (available_pixels / comfortable).floor();
    // Cast after clamping so an enormous (or NaN) panel cannot overflow `usize`.
    by_space.clamp(MIN_HINT_TICKS as f64, MAX_HINT_TICKS as f64) as usize
}

/// Resolves the ticks one axis draws.
///
/// * `available` is the length of the axis in logical pixels.
/// * `direction` is the way it runs on the canvas.
/// * `angle_deg` is the rotation applied to its labels.
///
/// An explicit tick list always wins: the caller asked for exactly those
/// values. Otherwise `theme.tick_count` decides: a pinned count is passed to
/// the scale as is, and the default `None` measures the labels before settling
/// on a count.
pub(crate) fn resolve_ticks(
    scale: &dyn ScaleTrait,
    explicit: Option<&[ExplicitTick]>,
    available: f64,
    direction: AxisDirection,
    angle_deg: f64,
    theme: &Theme,
) -> Vec<Tick> {
    if let Some(explicit) = explicit {
        return scale.create_explicit_ticks(explicit);
    }

    match theme.tick_count {
        Some(count) => scale.suggest_ticks(count.max(1)),
        None => auto_ticks(scale, available, direction, angle_deg, theme),
    }
}

/// Picks the densest tick list whose labels still fit.
///
/// The search is greedy and exact, which is affordable because an axis has at
/// most [`MAX_HINT_TICKS`] candidates: start at the length budget and step down
/// one tick at a time until every neighbouring pair clears the label geometry.
/// The rejected lists are never drawn, so grid lines and labels always agree;
/// nothing is painted first and hidden afterwards.
fn auto_ticks(
    scale: &dyn ScaleTrait,
    available: f64,
    direction: AxisDirection,
    angle_deg: f64,
    theme: &Theme,
) -> Vec<Tick> {
    let font_size = theme.tick_label_size.max(1.0);
    let densest = count_hint(available, font_size);
    let layout = LabelFit::new(direction, angle_deg, font_size);

    // A categorical scale owns its tick list: one tick per category, no matter
    // what count it is asked for. When those labels do not fit, thin the list
    // here rather than asking the scale for a different count it would ignore.
    if matches!(scale.scale_type(), Scale::Discrete) {
        let ticks = scale.suggest_ticks(densest);
        return fit_discrete(scale, ticks, available, layout);
    }

    let mut crowded: Option<Vec<Tick>> = None;

    // Search the scale's step ladder from dense to coarse. The lower bound is
    // one below the hint's floor on purpose: see `MIN_SEARCH_TICKS`.
    for count in (MIN_SEARCH_TICKS..=densest).rev() {
        let ticks = scale.suggest_ticks(count);

        // Nothing to space out: a single tick or none is a domain problem, not
        // a density problem.
        if ticks.len() <= 1 {
            return ticks;
        }

        // Measure the list the scale really produced: `suggest_ticks` snaps
        // the request to nice values, so the list can be denser than the
        // request, and it is the list that has to fit.
        if layout.fits(scale, &ticks, available) {
            return ticks;
        }

        crowded = Some(ticks);
    }

    // Even the coarsest step the scale offers does not fit, which happens when
    // one label is nearly as wide as the axis. Label only the two ends: both
    // are real, nice-aligned values, and no interval is invented. Dropping
    // individual interior labels instead would produce steps such as 150 or
    // uneven gaps, which mislead a reader who expects equal spacing.
    crowded.map(keep_ends).unwrap_or_default()
}

/// Keeps every `n`th label of a categorical axis until the labels fit.
///
/// A discrete scale always returns one tick per category, so the continuous
/// back-off loop cannot make it coarser. Dropping categories outright would
/// misrepresent the axis, but showing every category when the names are long
/// turns the labels into one unreadable string. Keeping a regular subset is the
/// categorical equivalent of moving to a coarser nice step.
fn fit_discrete(
    scale: &dyn ScaleTrait,
    ticks: Vec<Tick>,
    available: f64,
    layout: LabelFit,
) -> Vec<Tick> {
    if ticks.len() <= 2 || layout.fits(scale, &ticks, available) {
        return ticks;
    }

    // Every category gets a chance. A step is accepted as soon as the labels
    // that survive it fit; the loop is cheap because each step visits only the
    // labels it keeps, and an axis rarely needs a large step.
    for step in 2..ticks.len() {
        let thinned: Vec<Tick> = ticks.iter().step_by(step).cloned().collect();
        if layout.fits(scale, &thinned, available) {
            return thinned;
        }
    }

    keep_ends(ticks)
}

/// Reduces a tick list to its two end labels.
///
/// The last resort of the automatic mode. Both ends of the visible nice range
/// stay labelled, so the axis still reads as a range; the interior is left to
/// the user (rotate the labels, shorten the format, or pin ticks).
fn keep_ends(ticks: Vec<Tick>) -> Vec<Tick> {
    match ticks.len() {
        0 | 1 => ticks,
        len => vec![ticks[0].clone(), ticks[len - 1].clone()],
    }
}

/// The distance, in pixels, between two ticks on the axis.
fn tick_gap(scale: &dyn ScaleTrait, first: &Tick, second: &Tick, available: f64) -> f64 {
    let a = scale.normalize(first.value);
    let b = scale.normalize(second.value);
    (b - a).abs() * available
}

/// The smallest distance, in pixels, between any two neighbouring ticks.
#[cfg(test)]
fn nearest_gap(scale: &dyn ScaleTrait, ticks: &[Tick], available: f64) -> f64 {
    ticks
        .windows(2)
        .map(|pair| tick_gap(scale, &pair[0], &pair[1], available))
        .fold(f64::INFINITY, f64::min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::{Expansion, ScaleDomain, create_scale};

    fn linear_scale(min: f64, max: f64) -> std::sync::Arc<dyn ScaleTrait> {
        create_scale(
            &Scale::Linear,
            ScaleDomain::Continuous(min, max),
            Expansion {
                mult: (0.0, 0.0),
                add: (0.0, 0.0),
            },
            None,
        )
        .unwrap()
    }

    fn discrete_scale(labels: &[String]) -> std::sync::Arc<dyn ScaleTrait> {
        create_scale(
            &Scale::Discrete,
            ScaleDomain::Discrete(labels.to_vec()),
            Expansion {
                mult: (0.0, 0.0),
                add: (0.5, 0.5),
            },
            None,
        )
        .unwrap()
    }

    /// A vertical axis is limited by label *height*, so a tall panel with short
    /// numbers carries a dense but still readable scale.
    #[test]
    fn a_tall_axis_packs_short_labels() {
        let theme = Theme::default();
        let scale = linear_scale(0.0, 100.0);
        let ticks = resolve_ticks(
            scale.as_ref(),
            None,
            600.0,
            AxisDirection::Vertical,
            0.0,
            &theme,
        );

        // 600 / (13 x 4) = 11.5, capped at ten. All labels are short, so the
        // densest candidate should survive the fit check.
        assert!(
            (MIN_HINT_TICKS..=MAX_HINT_TICKS + 1).contains(&ticks.len()),
            "expected a dense but readable y axis, got {} ticks",
            ticks.len()
        );

        // Stacked y labels only need their ink bands to clear each other.
        let gap = nearest_gap(scale.as_ref(), &ticks, 600.0);
        let ink = ticks
            .iter()
            .map(|tick| estimate_text_height(&tick.label, theme.tick_label_size))
            .fold(0.0, f64::max);
        let air = ink + theme.tick_label_size * LINE_GAP_IN_FONTS;
        assert!(gap >= air, "labels are too close: gap {gap}, need {air}");
    }

    /// Wide labels must reduce the count until the widest one fits between its
    /// neighbours.
    #[test]
    fn long_labels_reduce_the_count_until_they_fit() {
        let theme = Theme::default();
        let scale = linear_scale(0.0, 1_000_000.0);
        let short = resolve_ticks(
            scale.as_ref(),
            None,
            900.0,
            AxisDirection::Horizontal,
            0.0,
            &theme,
        );

        // A formatter that makes every label wide ("1000000 milliseconds").
        let format = crate::scale::formatter::LabelFormat::new().with_suffix(" milliseconds");
        let wide = crate::scale::formatter::format_scale(scale, Some(&format));
        let long = resolve_ticks(
            wide.as_ref(),
            None,
            900.0,
            AxisDirection::Horizontal,
            0.0,
            &theme,
        );

        assert!(
            long.len() < short.len(),
            "wide labels should leave fewer ticks ({} vs {})",
            long.len(),
            short.len()
        );
        let gap = nearest_gap(wide.as_ref(), &long, 900.0);
        let widest = long
            .iter()
            .map(|t| estimate_text_width(&t.label, theme.tick_label_size))
            .fold(0.0, f64::max);
        let air = theme.tick_label_size * LABEL_GAP_IN_FONTS;
        assert!(
            gap + 0.001 >= widest + air,
            "labels are too close: gap {gap}, widest {widest}, air {air}"
        );
    }

    /// A categorical scale ignores the length budget, so crowded category names
    /// used to be drawn on top of each other. It now keeps every `n`th label.
    #[test]
    fn crowded_category_labels_are_thinned_evenly() {
        let theme = Theme::default();
        let labels: Vec<String> = (0..20).map(|i| format!("category {i:02}")).collect();
        let scale = discrete_scale(&labels);

        let ticks = resolve_ticks(
            scale.as_ref(),
            None,
            240.0,
            AxisDirection::Horizontal,
            0.0,
            &theme,
        );

        assert!(
            ticks.len() < labels.len(),
            "the long category names should have been thinned"
        );
        assert!(ticks.len() >= 2, "an axis still needs labels");

        let step = ticks[1].value - ticks[0].value;
        assert!(step >= 2.0, "step should skip at least one category");
        assert!(
            ticks
                .windows(2)
                .all(|pair| pair[1].value - pair[0].value == step),
            "the kept categories should be evenly spaced: {:?}",
            ticks.iter().map(|tick| tick.value).collect::<Vec<_>>()
        );

        let gap = nearest_gap(scale.as_ref(), &ticks, 240.0);
        let widest = ticks
            .iter()
            .map(|tick| estimate_text_width(&tick.label, theme.tick_label_size))
            .fold(0.0, f64::max);
        let air = theme.tick_label_size * LABEL_GAP_IN_FONTS;
        assert!(
            gap + 0.001 >= widest + air,
            "kept labels are still too close: gap {gap}, widest {widest}, air {air}"
        );
    }

    /// Short category labels are left alone; thinning only starts when the
    /// measured labels need the room.
    #[test]
    fn readable_category_labels_are_not_dropped() {
        let theme = Theme::default();
        let labels: Vec<String> = ["A", "B", "C"].into_iter().map(String::from).collect();
        let scale = discrete_scale(&labels);

        let ticks = resolve_ticks(
            scale.as_ref(),
            None,
            600.0,
            AxisDirection::Horizontal,
            0.0,
            &theme,
        );

        assert_eq!(ticks.len(), labels.len());
    }

    /// A huge canvas must not turn into a wall of labels.
    #[test]
    fn large_panels_are_capped() {
        let theme = Theme::default();
        let scale = linear_scale(0.0, 100.0);
        let ticks = resolve_ticks(
            scale.as_ref(),
            None,
            8_000.0,
            AxisDirection::Horizontal,
            0.0,
            &theme,
        );
        // The scale may answer with one more tick than requested when it snaps
        // to a nice step, but the budget itself stays capped.
        assert!(
            ticks.len() <= MAX_HINT_TICKS + 1,
            "capped count overshot: {}",
            ticks.len()
        );
        assert!(ticks.len() >= MIN_HINT_TICKS);
    }

    /// A tiny panel still gets a readable scale.
    #[test]
    fn small_panels_keep_a_floor() {
        let theme = Theme::default();
        let scale = linear_scale(0.0, 100.0);
        let ticks = resolve_ticks(
            scale.as_ref(),
            None,
            90.0,
            AxisDirection::Horizontal,
            0.0,
            &theme,
        );
        assert!(
            ticks.len() >= MIN_HINT_TICKS - 1,
            "got {} ticks",
            ticks.len()
        );
    }

    /// A pinned `with_tick_count` is honoured regardless of the panel length,
    /// and `with_auto_tick_count` brings the automatic placement back.
    #[test]
    fn explicit_density_is_honoured() {
        let scale = linear_scale(0.0, 100.0);

        let pinned = Theme::default().with_tick_count(20);
        for available in [300.0, 900.0, 3_000.0] {
            let ticks = resolve_ticks(
                scale.as_ref(),
                None,
                available,
                AxisDirection::Horizontal,
                0.0,
                &pinned,
            );
            // The scale snaps 20 to nice values: 0, 5, ..., 100 is 21 ticks.
            assert!(
                (18..=22).contains(&ticks.len()),
                "explicit count not honoured on a {available}px axis: {}",
                ticks.len()
            );
        }

        let auto = Theme::default().with_auto_tick_count();
        let small = resolve_ticks(
            scale.as_ref(),
            None,
            300.0,
            AxisDirection::Horizontal,
            0.0,
            &auto,
        );
        let large = resolve_ticks(
            scale.as_ref(),
            None,
            3_000.0,
            AxisDirection::Horizontal,
            0.0,
            &auto,
        );
        assert!(
            large.len() > small.len(),
            "auto should follow the panel length ({} vs {})",
            large.len(),
            small.len()
        );
    }

    /// A coarser nice step is preferred over thinning: the scale is asked for
    /// fewer ticks and answers with its next step down the `1/2/5` ladder.
    #[test]
    fn a_coarser_nice_step_is_used_before_giving_up() {
        let theme = Theme::default();
        // "0 units" and friends need roughly 60px, so the 1-step list (0, 1,
        // 2, 3 at 40px apart) cannot fit, but the 2-step list (0, 2) can.
        let format = crate::scale::formatter::LabelFormat::new().with_suffix(" units");
        let scale = crate::scale::formatter::format_scale(linear_scale(0.0, 3.0), Some(&format));

        let ticks = resolve_ticks(
            scale.as_ref(),
            None,
            120.0,
            AxisDirection::Horizontal,
            0.0,
            &theme,
        );

        let values: Vec<f64> = ticks.iter().map(|tick| tick.value).collect();
        assert_eq!(values, vec![0.0, 2.0], "expected the coarser 2-step");
    }

    /// When even the coarsest step is too tight, only the two ends stay
    /// labelled. No invented steps, no uneven gaps.
    #[test]
    fn extremely_long_labels_fall_back_to_the_ends() {
        let theme = Theme::default();
        let format =
            crate::scale::formatter::LabelFormat::new().with_suffix(" milliseconds elapsed");
        let wide =
            crate::scale::formatter::format_scale(linear_scale(0.0, 1_000_000.0), Some(&format));

        let ticks = resolve_ticks(
            wide.as_ref(),
            None,
            300.0,
            AxisDirection::Horizontal,
            0.0,
            &theme,
        );

        let values: Vec<f64> = ticks.iter().map(|tick| tick.value).collect();
        assert_eq!(values, vec![0.0, 1_000_000.0]);

        let widest = ticks
            .iter()
            .map(|tick| estimate_text_width(&tick.label, theme.tick_label_size))
            .fold(0.0, f64::max);
        let gap = nearest_gap(wide.as_ref(), &ticks, 300.0);
        let air = theme.tick_label_size * LABEL_GAP_IN_FONTS;
        assert!(
            gap + 0.001 >= widest + air,
            "the two end labels still collide: gap {gap}, widest {widest}, air {air}"
        );
    }

    /// A pinned count is not clamped to the automatic 3..10 window.
    #[test]
    fn a_pinned_count_is_not_clamped() {
        let theme = Theme::default().with_tick_count(30);
        let ticks = resolve_ticks(
            linear_scale(0.0, 100.0).as_ref(),
            None,
            900.0,
            AxisDirection::Horizontal,
            0.0,
            &theme,
        );
        assert!(
            ticks.len() > MAX_HINT_TICKS,
            "a pinned count should exceed the automatic window, got {}",
            ticks.len()
        );
    }

    /// Rotated labels are parallel lines, so their length does not decide the
    /// axis spacing. At the candlestick's 16px tick distance, a 45-degree date
    /// keeps every category instead of being thinned to every third one.
    #[test]
    fn diagonal_labels_use_line_spacing_not_label_width() {
        let theme = Theme::default();
        let labels: Vec<String> = (1..=22).map(|day| format!("07-{day:02}")).collect();
        let scale = discrete_scale(&labels);

        let flat = resolve_ticks(
            scale.as_ref(),
            None,
            352.0,
            AxisDirection::Horizontal,
            0.0,
            &theme,
        );
        let diagonal = resolve_ticks(
            scale.as_ref(),
            None,
            352.0,
            AxisDirection::Horizontal,
            -45.0,
            &theme,
        );

        assert_eq!(
            diagonal.len(),
            labels.len(),
            "45-degree dates should keep every category"
        );
        assert!(
            diagonal.len() > flat.len(),
            "rotation should make room ({} vs {})",
            diagonal.len(),
            flat.len()
        );
    }

    /// A rotated label takes up its height, not its width, along a horizontal
    /// axis, so rotating lets more ticks fit.
    #[test]
    fn rotation_makes_room_for_more_labels() {
        let theme = Theme::default();
        let format = crate::scale::formatter::LabelFormat::new().with_suffix(" units");
        let wide = crate::scale::formatter::format_scale(linear_scale(0.0, 1_000.0), Some(&format));
        let flat = resolve_ticks(
            wide.as_ref(),
            None,
            600.0,
            AxisDirection::Horizontal,
            0.0,
            &theme,
        );
        let rotated = resolve_ticks(
            wide.as_ref(),
            None,
            600.0,
            AxisDirection::Horizontal,
            90.0,
            &theme,
        );
        assert!(
            rotated.len() >= flat.len(),
            "rotating should not lose ticks ({} vs {})",
            rotated.len(),
            flat.len()
        );
    }
}
