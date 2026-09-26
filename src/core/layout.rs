use super::context::PanelContext;
use super::flow::{self, Direction, Extent};
use super::guide::{
    ColorBarGeometry, EntryMetrics, EntryOffset, GuideSpec, LegendPosition, MeasuredGuide,
};
use super::utils::estimate_text_width;
use crate::coordinate::Rect;
use crate::theme::Theme;

/// Physical constraints calculated for axis areas.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct AxisLayoutConstraints {
    pub bottom: f64,
    pub left: f64,
}

/// Margin reserved on each side of the plot for legend placement.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct LegendLayoutConstraints {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

/// One guide (a legend or a colour bar) placed inside the legend strip.
#[derive(Debug, Clone)]
pub(crate) struct GuideBlockLayout {
    /// Index into the `GuideSpec` slice this block was measured from.
    pub(crate) index: usize,
    /// Top-left corner of the block, relative to the top-left of the strip.
    pub(crate) offset_x: f64,
    pub(crate) offset_y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    /// Symbol/label geometry shared by all of the block's entries.
    pub(crate) metrics: EntryMetrics,
    /// Pre-computed top-left corner of every entry, relative to this block.
    ///
    /// Renderers must draw these offsets verbatim instead of wrapping again:
    /// duplicated wrapping logic is what used to let the measured size and the
    /// drawn size drift apart.
    pub(crate) entries: Vec<EntryOffset>,
    /// Resolved gradient geometry, for colour bars. Also drawn verbatim.
    pub(crate) colorbar: Option<ColorBarGeometry>,
}

/// The fully resolved legend: where each block goes and how big the strip is.
#[derive(Debug, Clone)]
pub(crate) struct LegendLayoutPlan {
    pub(crate) position: LegendPosition,
    pub(crate) blocks: Vec<GuideBlockLayout>,
    /// Bounding box of the whole strip, containing every block.
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl LegendLayoutPlan {
    /// An empty plan, for charts that do not draw a legend.
    pub(crate) const fn empty(position: LegendPosition) -> Self {
        Self {
            position,
            blocks: Vec::new(),
            width: 0.0,
            height: 0.0,
        }
    }
}

pub struct LayoutEngine;

impl LayoutEngine {
    /// Measures every guide and packs the resulting blocks into one strip.
    ///
    /// This is the only place that decides where legend content goes. It works at
    /// two nested levels, and both levels are handled by [`flow::pack`]:
    ///
    /// 1. **Inside a block**: the entries of one guide are turned into boxes and
    ///    packed by [`GuideSpec::measure`]. A box there is one `● Label` row.
    /// 2. **Between blocks**: the blocks are packed by this function, with the very
    ///    same algorithm. A box here is one whole guide, so it contains its title
    ///    plus all of its entries, which step 1 has already laid out.
    ///
    /// # What the budget is here
    ///
    /// `main_budget` is how long the strip may become along the packing direction
    /// before it wraps onto another line (a column for a left/right legend, a row
    /// for a top/bottom one). The caller derives it from the plot panel: a legend
    /// must never be longer than the panel it sits next to, otherwise it would run
    /// past the axis line.
    pub(crate) fn pack_guides(
        specs: &[GuideSpec],
        position: LegendPosition,
        main_budget: f64,
        theme: &Theme,
    ) -> LegendLayoutPlan {
        let direction = Direction::for_legend(position);

        // Level 1: measure every block, which also resolves its entry positions.
        let measured: Vec<MeasuredGuide> = specs
            .iter()
            .map(|spec| spec.measure(theme, direction, main_budget))
            .collect();

        // Level 2: pack the blocks with the very same algorithm. Note that the
        // budget is *not* reduced here: a block already fits it by construction,
        // because its own entries were wrapped against it in level 1.
        let extents: Vec<Extent> = measured
            .iter()
            .map(|guide| direction.measure(guide.size.width, guide.size.height))
            .collect();
        let packed = flow::pack(
            &extents,
            main_budget,
            theme.legend_block_gap,
            theme.legend_block_gap,
        );

        let blocks: Vec<GuideBlockLayout> = measured
            .into_iter()
            .zip(&packed.offsets)
            .enumerate()
            .map(|(index, (guide, &(main, cross)))| {
                let (offset_x, offset_y) = direction.resolve(main, cross);
                GuideBlockLayout {
                    index,
                    offset_x,
                    offset_y,
                    width: guide.size.width,
                    height: guide.size.height,
                    metrics: guide.metrics,
                    entries: guide.entries,
                    colorbar: guide.colorbar,
                }
            })
            .collect();

        let (width, height) = direction.resolve(packed.main_total, packed.cross_total);

        // Invariants guarded in debug builds: every entry, and every gradient
        // bar, must stay inside the box reserved for its block. A violation means
        // the measurement and the drawing disagree about the same geometry, which
        // is precisely the class of bug this layout engine exists to make
        // impossible.
        debug_assert!(blocks.iter().all(|block| {
            let block_extent = direction.measure(block.width, block.height);
            block.entries.iter().all(|entry| {
                entry.x >= 0.0 && entry.y >= 0.0 && entry.x < block.width && entry.y < block.height
            }) && block.colorbar.is_none_or(|bar| {
                bar.length <= block_extent.main && bar.thickness <= block_extent.cross
            })
        }));

        LegendLayoutPlan {
            position,
            blocks,
            width,
            height,
        }
    }

    /// The region a legend is allowed to paint in: everything between the panel
    /// and the canvas edge on the side the legend sits on.
    ///
    /// Normally the strip is packed to be no longer than the panel it sits next
    /// to, so it fits inside this band on its own and the clip changes nothing.
    /// It matters when a legend is far too large to ever fit -- hundreds of
    /// categories on a small canvas: the space reserved for it is clamped so that
    /// the panel keeps a usable size (`legend_constraints_for_plan`), leaving the
    /// strip longer than the space it was given. Clipping to this band is what
    /// keeps that surplus from being painted over the data: the legend shows its
    /// leading entries and the rest is cut off, instead of covering the plot.
    ///
    /// The band is deliberately generous -- it is the whole half-plane outside the
    /// panel, not just the panel's own extent. It is a safety net, not the thing
    /// that lays the legend out, so it should only ever catch genuine overflow.
    pub(crate) fn legend_band(
        position: LegendPosition,
        panel: &Rect,
        canvas_w: f64,
        canvas_h: f64,
    ) -> Rect {
        let (x, y, width, height) = match position {
            LegendPosition::Right => (
                panel.x + panel.width,
                0.0,
                canvas_w - (panel.x + panel.width),
                canvas_h,
            ),
            LegendPosition::Left => (0.0, 0.0, panel.x, canvas_h),
            LegendPosition::Top => (0.0, 0.0, canvas_w, panel.y),
            LegendPosition::Bottom => (
                0.0,
                panel.y + panel.height,
                canvas_w,
                canvas_h - (panel.y + panel.height),
            ),
            // No legend is drawn at all in this case; hand back the whole canvas.
            LegendPosition::None => (0.0, 0.0, canvas_w, canvas_h),
        };
        Rect::new(x, y, width.max(0.0), height.max(0.0))
    }

    /// Turns a measured strip into the margin it needs, expressed as space taken
    /// away from the plot panel.
    ///
    /// The margin is clamped so that the panel always keeps a usable size, even
    /// if the legend would happily eat the whole canvas. The strip itself is not
    /// resized by that clamp -- only the space reserved for it is.
    pub(crate) fn legend_constraints_for_plan(
        plan: &LegendLayoutPlan,
        canvas_w: f64,
        canvas_h: f64,
        margin_gap: f64,
        theme: &Theme,
    ) -> LegendLayoutConstraints {
        let mut constraints = LegendLayoutConstraints::default();
        if plan.blocks.is_empty() || matches!(plan.position, LegendPosition::None) {
            return constraints;
        }

        if matches!(plan.position, LegendPosition::Left | LegendPosition::Right) {
            let min_panel_w = f64::max(theme.min_panel_size, canvas_w * theme.panel_defense_ratio);
            let max_width = (canvas_w - min_panel_w - theme.axis_reserve_buffer).max(0.0);
            let reserve = f64::min(plan.width, max_width) + margin_gap;
            if plan.position == LegendPosition::Right {
                constraints.right = reserve;
            } else {
                constraints.left = reserve;
            }
        } else {
            let min_panel_h = f64::max(theme.min_panel_size, canvas_h * theme.panel_defense_ratio);
            let max_height = (canvas_h - min_panel_h - theme.axis_reserve_buffer).max(0.0);
            let reserve = f64::min(plan.height, max_height) + margin_gap;
            if plan.position == LegendPosition::Top {
                constraints.top = reserve;
            } else {
                constraints.bottom = reserve;
            }
        }
        constraints
    }

    /// Calculates layout constraints based on predicted axis dimensions.
    ///
    /// This method uses the chart's reference dimensions to estimate the "worst-case"
    /// margin required for labels and titles.
    pub fn calculate_axis_constraints(
        ctx: &PanelContext,
        theme: &Theme,
        reference_width: f64,
        reference_height: f64,
    ) -> AxisLayoutConstraints {
        let mut constraints = AxisLayoutConstraints::default();
        let coord = ctx.coord.clone();
        let is_flipped = coord.is_flipped();

        // 1. Resolve Bottom Axis:
        // Uses X-scale by default; uses Y-scale if the coordinate system is flipped.
        let (b_scale, b_angle, b_title, b_pad) = if is_flipped {
            (
                coord.get_y_scale(),
                theme.y_tick_label_angle,
                coord.get_y_label(),
                theme.label_padding,
            )
        } else {
            (
                coord.get_x_scale(),
                theme.x_tick_label_angle,
                coord.get_x_label(),
                theme.label_padding,
            )
        };
        constraints.bottom = Self::estimate_axis_dimension(
            b_scale,
            b_angle,
            b_title,
            b_pad,
            theme,
            true,
            reference_width,
        );

        // 2. Resolve Left Axis:
        // Uses Y-axis by default, or X-axis if flipped.
        let (l_scale, l_angle, l_title, l_pad) = if is_flipped {
            (
                coord.get_x_scale(),
                theme.x_tick_label_angle,
                coord.get_x_label(),
                theme.label_padding,
            )
        } else {
            (
                coord.get_y_scale(),
                theme.y_tick_label_angle,
                coord.get_y_label(),
                theme.label_padding,
            )
        };
        constraints.left = Self::estimate_axis_dimension(
            l_scale,
            l_angle,
            l_title,
            l_pad,
            theme,
            false,
            reference_height,
        );

        constraints
    }

    /// Estimates the total physical 'depth' required for an axis.
    ///
    /// For a Bottom axis, this represents the total height (from the X-axis line down to the SVG edge).
    /// For a Left axis, this represents the total width (from the Y-axis line left to the SVG edge).
    ///
    /// It accounts for:
    /// 1. Tick mark lines.
    /// 2. Padding between ticks and labels.
    /// 3. The bounding box of rotated labels (using trigonometry).
    /// 4. Padding between labels and the axis title.
    /// 5. The height of the title text itself.
    /// 6. A final safety buffer for the SVG edge.
    fn estimate_axis_dimension(
        scale: &dyn crate::scale::ScaleTrait,
        angle_deg: f64,
        title: &str,
        label_padding: f64, // The padding between labels and title (theme.label_padding)
        theme: &Theme,
        is_horizontal_axis: bool,
        available_space: f64,
    ) -> f64 {
        // Physical constants (should match those used in draw_ticks_and_labels)
        let tick_line_len = 6.0;
        let title_gap = 5.0; // Distance between labels and the title text
        let edge_buffer = 10.0; // Prevents the title from touching the very edge of the SVG
        let angle_rad = angle_deg.to_radians();

        // 1. Predictive Tick Generation
        // We must generate the same number of ticks as the renderer to ensure we
        // measure the actual strings (like "1.0000E7") that will be displayed.
        let final_count = theme.suggest_tick_count(available_space);
        let ticks = scale.suggest_ticks(final_count);

        // 2. Compute the physical footprint of the labels
        // Rotated text creates a bounding box. We need the projection of this box
        // onto the axis perpendicular to the chart.
        let max_label_footprint = ticks
            .iter()
            .map(|t| {
                // Approximation of string width based on character weights
                let w = estimate_text_width(&t.label, theme.tick_label_size);
                // The height of the font (cap-height approximation)
                let h = theme.tick_label_size;

                // Formula for the height/width of a rotated rectangle:
                // For Bottom axis (horizontal), we need vertical depth: |w*sin| + |h*cos|
                // For Left axis (vertical), we need horizontal depth: |w*cos| + |h*sin|
                if is_horizontal_axis {
                    w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
                } else {
                    w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
                }
            })
            .fold(0.0, f64::max);

        // 3. Title Area Calculation
        // If a title exists, we need to reserve space for the gap and the text height.
        let title_area = if title.is_empty() {
            0.0
        } else {
            // We reserve the full height of the title font (label_size) plus the
            // padding and the gap. This ensures the title doesn't overlap labels
            // and doesn't get clipped by the SVG boundary.
            title_gap + theme.label_size + label_padding
        };

        // 4. Summation of Layout Segments
        // Total Depth = [Tick] + [Padding] + [Label Box] + [Title Area] + [Edge Buffer]
        tick_line_len + theme.tick_label_padding + max_label_footprint + title_area + edge_buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::aesthetics::AestheticMapping;
    use crate::scale::mapper::VisualMapper;
    use crate::scale::{Expansion, Scale, ScaleDomain, create_scale};

    /// A guide with two categories and a label wide enough to matter.
    fn legend_specs() -> Vec<GuideSpec> {
        let scale = create_scale(
            &Scale::Discrete,
            ScaleDomain::Discrete(vec!["A".into(), "B".into()]),
            Expansion {
                mult: (0.0, 0.0),
                add: (0.0, 0.0),
            },
            Some(VisualMapper::new_shape_default()),
        )
        .unwrap();

        vec![GuideSpec::new(
            "group".into(),
            ScaleDomain::Discrete(vec!["A".into(), "B".into()]),
            vec![AestheticMapping {
                field: "group".into(),
                title: None,
                scale_impl: scale,
            }],
        )]
    }

    #[test]
    fn legend_reserves_space_on_the_requested_side_only() {
        let specs = legend_specs();
        let theme = Theme::default();
        let (canvas_w, canvas_h) = (500.0, 400.0);

        for (position, expected) in [
            (
                LegendPosition::Left,
                LegendLayoutConstraints {
                    left: 1.0,
                    ..Default::default()
                },
            ),
            (
                LegendPosition::Right,
                LegendLayoutConstraints {
                    right: 1.0,
                    ..Default::default()
                },
            ),
            (
                LegendPosition::Top,
                LegendLayoutConstraints {
                    top: 1.0,
                    ..Default::default()
                },
            ),
            (
                LegendPosition::Bottom,
                LegendLayoutConstraints {
                    bottom: 1.0,
                    ..Default::default()
                },
            ),
        ] {
            let plan = LayoutEngine::pack_guides(&specs, position, 300.0, &theme);
            let reserve =
                LayoutEngine::legend_constraints_for_plan(&plan, canvas_w, canvas_h, 8.0, &theme);

            // Normalise to 1.0 so that only the *side* is compared, not the size.
            let actual = LegendLayoutConstraints {
                top: f64::from(reserve.top > 0.0),
                bottom: f64::from(reserve.bottom > 0.0),
                left: f64::from(reserve.left > 0.0),
                right: f64::from(reserve.right > 0.0),
            };
            assert_eq!(actual.top, expected.top, "top for {position:?}");
            assert_eq!(actual.bottom, expected.bottom, "bottom for {position:?}");
            assert_eq!(actual.left, expected.left, "left for {position:?}");
            assert_eq!(actual.right, expected.right, "right for {position:?}");
        }
    }

    #[test]
    fn vertical_legends_wrap_into_columns_before_exceeding_the_budget() {
        let specs = legend_specs();
        let theme = Theme::default();

        // A budget too small for even two entries forces one column per entry.
        let plan = LayoutEngine::pack_guides(&specs, LegendPosition::Right, 20.0, &theme);
        let block = &plan.blocks[0];

        assert_eq!(block.entries.len(), 2);
        // The second entry must sit in a new column, i.e. to the right.
        assert!(block.entries[1].x > block.entries[0].x);
        assert_eq!(block.entries[1].y, block.entries[0].y);
    }

    #[test]
    fn entries_stay_inside_their_block_and_below_the_title() {
        let specs = legend_specs();
        let theme = Theme::default();
        let plan = LayoutEngine::pack_guides(&specs, LegendPosition::Right, 300.0, &theme);
        let block = &plan.blocks[0];

        let title_height = theme.legend_label_size * 1.1 + theme.legend_title_gap;
        assert!(block.entries[0].y >= title_height);

        // Nothing may escape the box the layout reserved for the block.
        for entry in &block.entries {
            assert!(
                entry.x >= 0.0 && entry.x < block.width,
                "entry.x = {}",
                entry.x
            );
            assert!(
                entry.y >= 0.0 && entry.y < block.height,
                "entry.y = {}",
                entry.y
            );
        }
    }

    /// The band a legend is clipped to must never touch the plot panel, whatever
    /// the legend position and however squeezed the panel is.
    #[test]
    fn legend_band_never_intersects_the_panel() {
        let canvases = [(720.0, 480.0), (300.0, 300.0), (2000.0, 200.0)];
        // A comfortable panel, and one squeezed against the safety floor.
        let panels = [
            Rect::new(102.0, 48.0, 530.0, 336.0),
            Rect::new(102.0, 48.0, 100.0, 100.0),
        ];

        for (canvas_w, canvas_h) in canvases {
            for panel in panels {
                for position in [
                    LegendPosition::Right,
                    LegendPosition::Left,
                    LegendPosition::Top,
                    LegendPosition::Bottom,
                ] {
                    let band = LayoutEngine::legend_band(position, &panel, canvas_w, canvas_h);

                    let overlap_x =
                        (band.x + band.width).min(panel.x + panel.width) - band.x.max(panel.x);
                    let overlap_y =
                        (band.y + band.height).min(panel.y + panel.height) - band.y.max(panel.y);

                    assert!(
                        overlap_x.max(0.0) * overlap_y.max(0.0) == 0.0,
                        "{position:?} on {canvas_w}x{canvas_h}: band {band:?} covers the panel {panel:?}"
                    );
                    // The band has to be on the legend's side of the panel, so it
                    // must also be non-empty on a sane canvas.
                    assert!(band.width >= 0.0 && band.height >= 0.0);
                }
            }
        }
    }

    /// The band is the half-plane outside the panel on the legend's side.
    #[test]
    fn legend_band_sits_outside_the_requested_edge() {
        let panel = Rect::new(100.0, 50.0, 500.0, 300.0);
        let band = |position| LayoutEngine::legend_band(position, &panel, 800.0, 400.0);

        assert_eq!(band(LegendPosition::Right).x, 600.0);
        assert_eq!(band(LegendPosition::Left).width, 100.0);
        assert_eq!(band(LegendPosition::Top).height, 50.0);
        assert_eq!(band(LegendPosition::Bottom).y, 350.0);
    }

    #[test]
    fn a_hidden_legend_reserves_nothing() {
        let theme = Theme::default();
        let plan = LegendLayoutPlan::empty(LegendPosition::Right);
        let constraints =
            LayoutEngine::legend_constraints_for_plan(&plan, 500.0, 400.0, 8.0, &theme);

        assert_eq!(constraints.top, 0.0);
        assert_eq!(constraints.bottom, 0.0);
        assert_eq!(constraints.left, 0.0);
        assert_eq!(constraints.right, 0.0);
    }

    /// A continuous colour mapping, used to exercise the gradient-bar path.
    fn colorbar_spec() -> GuideSpec {
        let scale = create_scale(
            &Scale::Linear,
            ScaleDomain::Continuous(0.0, 100.0),
            Expansion {
                mult: (0.0, 0.0),
                add: (0.0, 0.0),
            },
            Some(VisualMapper::new_color_default(
                &Scale::Linear,
                &Theme::default(),
            )),
        )
        .unwrap();

        GuideSpec::new(
            "value".into(),
            ScaleDomain::Continuous(0.0, 100.0),
            vec![AestheticMapping {
                field: "value".into(),
                title: None,
                scale_impl: scale,
            }],
        )
    }

    /// A vertical gradient bar scales with the panel but never exceeds its cap,
    /// and the geometry the plan records is the geometry a renderer will draw.
    #[test]
    fn vertical_colorbar_length_is_derived_from_the_budget() {
        let theme = Theme::default();
        let specs = [colorbar_spec()];

        for (budget, expected) in [
            (100.0, 70.0),  // 70% of a tight budget
            (200.0, 140.0), // still scaling
            (286.0, 200.0), // 0.7 * 286 = 200.2 -> capped
            (700.0, 200.0), // a tall chart must not produce a giant bar
        ] {
            let plan = LayoutEngine::pack_guides(&specs, LegendPosition::Right, budget, &theme);
            let bar = plan.blocks[0].colorbar.expect("colour bar geometry");

            assert!(
                (bar.length - expected).abs() < 1e-6,
                "budget {budget}: expected bar length {expected}, got {}",
                bar.length
            );
            // The bar plus its title has to fit the space the block reserved.
            assert!(bar.length + 20.2 <= plan.blocks[0].height + 1e-6);
        }
    }

    /// A horizontal gradient bar is clamped, and never longer than its block.
    #[test]
    fn horizontal_colorbar_length_is_clamped() {
        let theme = Theme::default();
        let specs = [colorbar_spec()];

        for (budget, expected) in [(60.0, 150.0), (220.0, 220.0), (900.0, 300.0)] {
            let plan = LayoutEngine::pack_guides(&specs, LegendPosition::Top, budget, &theme);
            let block = &plan.blocks[0];
            let bar = block.colorbar.expect("colour bar geometry");

            assert!(
                (bar.length - expected).abs() < 1e-6,
                "budget {budget}: expected bar length {expected}, got {}",
                bar.length
            );
            assert!(bar.length <= block.width + 1e-6);
        }
    }
}
