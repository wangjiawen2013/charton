//! Where every part of the chart ends up on the canvas.
//!
//! A chart is not just data. It also has a title, one or more legends and a
//! pair of axes, and every one of them needs room. This module works out how
//! much room each of them takes and hands back the rectangle that is left for
//! the data itself.
//!
//! The canvas is treated as a set of nested rectangles, a bit like a picture
//! frame:
//!
//! ```text
//!   +------------------------------------------+  canvas edge
//!   |  outer margin                             |
//!   |   +----------------------------------+    |
//!   |   |  title                            |   |
//!   |   +----------------------------------+    |
//!   |   |  legend (top)                     |   |
//!   |   +----------------------------------+    |
//!   |   |                                   |   |
//!   |   |  plot panel                       |   |
//!   |   |                                   |   |
//!   |   +----------------------------------+    |
//!   |                                           |
//!   +------------------------------------------+
//! ```
//!
//! The strips around the panel are called "bands". A band knows which edge it
//! belongs to, how thick it is, and how much breathing room it needs before the
//! next thing inside it. The bands on one edge are stacked from the canvas edge
//! inwards. The title is always the outermost band on top, so a legend placed
//! at the top ends up below the title instead of under it. A legend on the
//! left, right or bottom sits outside the axis, so it never lands on the axis
//! labels.
//!
//! The sizes feed back into each other: the axes get deeper when there is more
//! room, the legend wraps into fewer rows when the panel is wider, and the
//! panel is exactly what is left once the two of them have taken their share.
//! The caller breaks that loop by measuring a few times until nothing changes.
//! This module only provides the measuring and the stacking.

use super::context::PanelContext;
use super::flow::{self, Direction, Extent};
use super::guide::{
    ColorBarGeometry, EntryMetrics, EntryOffset, GuideSpec, LegendPosition, MeasuredGuide,
};
use super::utils::estimate_text_width;
use crate::coordinate::Rect;
use crate::theme::Theme;

/// How much room the axes take on the left and below the panel.
///
/// The chart draws the value scale down the left edge and the category scale
/// along the bottom, so these two numbers describe the only space the axes
/// claim. They are measured before the panel is finalised. When the chart is
/// split into facets, the same numbers are handed to the facet grid, which puts
/// the axes inside the grid instead of around the whole chart.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct AxisLayoutConstraints {
    pub bottom: f64,
    pub left: f64,
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

/// What a reserved band actually holds.
///
/// The title and the legend are drawn by their own renderers, so the layout
/// hands them a finished rectangle to draw in. The axes are different: the
/// coordinate system already knows how to draw them from the panel alone, so
/// their band exists only to keep the panel away from the edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BandKind {
    Title,
    Legend,
    XAxis,
    YAxis,
}

/// A strip of space reserved on one edge of the panel.
///
/// `thickness` is how far the strip reaches inwards from its edge, and `gap` is
/// the breathing room between this strip and whatever comes next inside it.
/// Adding the two up for every band on a side gives the total space that side
/// takes away from the panel.
pub(crate) struct Band {
    pub(crate) kind: BandKind,
    pub(crate) thickness: f64,
    pub(crate) gap: f64,
}

/// The bands reserved on each of the four edges, outermost first.
///
/// "Outermost first" means the order runs from the canvas edge inwards, so the
/// first band on the top edge is the one closest to the top of the picture.
#[derive(Default)]
pub(crate) struct Bands {
    pub(crate) top: Vec<Band>,
    pub(crate) bottom: Vec<Band>,
    pub(crate) left: Vec<Band>,
    pub(crate) right: Vec<Band>,
}

/// Total space each edge gives up, gaps included.
///
/// The caller compares this between measuring passes to notice when the layout
/// has stopped moving.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(crate) struct Reservations {
    pub(crate) top: f64,
    pub(crate) bottom: f64,
    pub(crate) left: f64,
    pub(crate) right: f64,
}

impl Reservations {
    /// Adds up the bands on each edge.
    pub(crate) fn of(bands: &Bands) -> Self {
        let along = |bands: &[Band]| bands.iter().map(|b| b.thickness + b.gap).sum();
        Self {
            top: along(&bands.top),
            bottom: along(&bands.bottom),
            left: along(&bands.left),
            right: along(&bands.right),
        }
    }
}

/// The finished physical layout.
///
/// `panel` is where the data is drawn. `title` and `legend` are the rectangles
/// their renderers must paint in, and they are `None` when the chart has no
/// title or no legend. The axes do not need a rectangle here: the coordinate
/// system draws them straight from `panel`.
pub(crate) struct LayoutPlan {
    pub(crate) panel: Rect,
    pub(crate) title: Option<Rect>,
    pub(crate) legend: Option<Rect>,
}

pub struct LayoutEngine;

impl LayoutEngine {
    /// Packs every guide into one legend strip.
    ///
    /// This works at two nested levels, and both are handled by the very same
    /// routine, [`flow::pack`]:
    ///
    /// 1. **Inside a block**: the entries of one guide are turned into boxes and
    ///    packed by [`GuideSpec::measure`]. A box there is one `● Label` row.
    /// 2. **Between blocks**: the blocks are packed here. A box at this level is
    ///    one whole guide, so it contains its title plus all of its entries,
    ///    which step 1 has already laid out.
    ///
    /// `main_budget` is how long the strip may become along the packing
    /// direction before it wraps onto another line (a column for a left/right
    /// legend, a row for a top/bottom one). The caller derives it from the plot
    /// panel: a legend must never be longer than the panel it sits next to,
    /// otherwise it would run past the axis line.
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
        // bar, must stay inside the box reserved for its block.
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

    /// How thick the legend strip is on its edge.
    ///
    /// A legend on the top or bottom edge grows sideways, so its thickness is
    /// its height. A legend on the left or right edge grows downwards, so its
    /// thickness is its width.
    ///
    /// The number is capped so that a legend can never squeeze the panel out of
    /// existence. The cap only limits how much room is *reserved*; a legend that
    /// is larger than the cap still draws its full size and is cut off at the
    /// reserved edge.
    pub(crate) fn legend_thickness(
        plan: &LegendLayoutPlan,
        canvas_w: f64,
        canvas_h: f64,
        theme: &Theme,
    ) -> f64 {
        if plan.blocks.is_empty() {
            return 0.0;
        }

        let (raw, canvas) = match plan.position {
            LegendPosition::Top | LegendPosition::Bottom => (plan.height, canvas_h),
            LegendPosition::Left | LegendPosition::Right => (plan.width, canvas_w),
            LegendPosition::None => (0.0, 0.0),
        };

        // Keep at least a usable slice of the canvas for the panel.
        let min_panel = (canvas * theme.panel_defense_ratio).max(theme.min_panel_size);
        let max_reserved = (canvas - min_panel - theme.axis_reserve_buffer).max(0.0);
        raw.min(max_reserved)
    }

    /// Lays out the bands that surround the panel.
    ///
    /// The order within each edge runs from the canvas inwards, which is what
    /// keeps the title above a top legend and the legend below a bottom axis.
    /// The axes are left out entirely for a faceted chart, because there they
    /// live inside the facet grid rather than around the whole chart.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn build_bands(
        theme: &Theme,
        has_title: bool,
        show_legend: bool,
        legend_position: LegendPosition,
        legend_thickness: f64,
        axis: &AxisLayoutConstraints,
        outer_axes: bool,
    ) -> Bands {
        let mut bands = Bands::default();

        // The title is always the outermost thing on top, so a top legend ends
        // up between the title and the panel instead of on top of it.
        if has_title {
            bands.top.push(Band {
                kind: BandKind::Title,
                thickness: theme.title_size * 1.2,
                gap: theme.title_padding,
            });
        }
        if show_legend && legend_position == LegendPosition::Top {
            bands.top.push(Band {
                kind: BandKind::Legend,
                thickness: legend_thickness,
                gap: theme.legend_margin,
            });
        }

        // On the other three edges the legend goes outside the axis, so the
        // tick labels and the axis title stay clear of it.
        if show_legend && legend_position == LegendPosition::Bottom {
            bands.bottom.push(Band {
                kind: BandKind::Legend,
                thickness: legend_thickness,
                gap: theme.legend_margin,
            });
        }
        if show_legend && legend_position == LegendPosition::Left {
            bands.left.push(Band {
                kind: BandKind::Legend,
                thickness: legend_thickness,
                gap: theme.legend_margin,
            });
        }
        if show_legend && legend_position == LegendPosition::Right {
            bands.right.push(Band {
                kind: BandKind::Legend,
                thickness: legend_thickness,
                gap: theme.legend_margin,
            });
        }

        if outer_axes {
            bands.bottom.push(Band {
                kind: BandKind::XAxis,
                thickness: axis.bottom,
                gap: 0.0,
            });
            bands.left.push(Band {
                kind: BandKind::YAxis,
                thickness: axis.left,
                gap: 0.0,
            });
        }

        bands
    }

    /// The panel left over once every band has taken its share.
    ///
    /// The result never shrinks below `min_panel`, so a chart that is far too
    /// small can overflow instead of collapsing to nothing. The caller uses
    /// this to measure the axes and the legend against the panel they will
    /// actually sit next to.
    pub(crate) fn panel_from(content: Rect, reservations: &Reservations, min_panel: f64) -> Rect {
        let width = (content.width - reservations.left - reservations.right).max(min_panel);
        let height = (content.height - reservations.top - reservations.bottom).max(min_panel);
        Rect::new(
            content.x + reservations.left,
            content.y + reservations.top,
            width,
            height,
        )
    }

    /// Places every band and returns the final panel.
    ///
    /// This is the last step of the layout: the sizes have already been agreed
    /// on, so here they are only turned into rectangles. The panel is worked out
    /// first, and then the bands are stacked outwards from its edges, innermost
    /// first. Building the bands out from the panel this way means they can never
    /// be drawn on top of it, even when the panel is so squeezed that it hits its
    /// smallest allowed size.
    pub(crate) fn arrange(content: Rect, bands: &Bands, min_panel: f64) -> LayoutPlan {
        let reservations = Reservations::of(bands);
        let panel = Self::panel_from(content, &reservations, min_panel);

        let mut title = None;
        let mut legend = None;

        // The title is centred over the whole canvas, so its band spans the full
        // width. A legend only ever sits next to the panel, so its band spans
        // exactly the panel on the cross axis and can never reach the title.
        let cross_of = |kind: BandKind, along_side: bool| {
            if along_side {
                // Top/bottom band: the cross axis is horizontal.
                if kind == BandKind::Title {
                    (content.x, content.width)
                } else {
                    (panel.x, panel.width)
                }
            } else {
                (panel.y, panel.height)
            }
        };

        // Top: walk up from the panel. The last band in the list is the one
        // closest to the panel, so the list is walked backwards.
        let mut edge = panel.y;
        for band in bands.top.iter().rev() {
            edge -= band.gap;
            edge -= band.thickness;
            let (x, width) = cross_of(band.kind, true);
            let rect = Rect::new(x, edge, width, band.thickness);
            match band.kind {
                BandKind::Title => title = Some(rect),
                BandKind::Legend => legend = Some(rect),
                _ => {}
            }
        }

        // Bottom: walk down from the panel.
        let mut edge = panel.y + panel.height;
        for band in bands.bottom.iter().rev() {
            edge += band.gap;
            let (x, width) = cross_of(band.kind, true);
            let rect = Rect::new(x, edge, width, band.thickness);
            edge += band.thickness;
            if band.kind == BandKind::Legend {
                legend = Some(rect);
            }
        }

        // Left: walk left from the panel.
        let mut edge = panel.x;
        for band in bands.left.iter().rev() {
            edge -= band.gap;
            edge -= band.thickness;
            let (y, height) = cross_of(band.kind, false);
            let rect = Rect::new(edge, y, band.thickness, height);
            if band.kind == BandKind::Legend {
                legend = Some(rect);
            }
        }

        // Right: walk right from the panel.
        let mut edge = panel.x + panel.width;
        for band in bands.right.iter().rev() {
            edge += band.gap;
            let (y, height) = cross_of(band.kind, false);
            let rect = Rect::new(edge, y, band.thickness, height);
            edge += band.thickness;
            if band.kind == BandKind::Legend {
                legend = Some(rect);
            }
        }

        LayoutPlan {
            panel,
            title,
            legend,
        }
    }

    /// Calculates layout constraints based on predicted axis dimensions.
    ///
    /// This method uses the chart's reference dimensions to estimate the
    /// "worst-case" margin required for labels and titles.
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
    /// For a Bottom axis, this represents the total height (from the X-axis line
    /// down to the SVG edge). For a Left axis, this represents the total width
    /// (from the Y-axis line left to the SVG edge).
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

    /// The title band must sit above a top legend, never on top of it.
    #[test]
    fn title_band_sits_above_a_top_legend() {
        let theme = Theme::default();
        let legend = 50.0;
        let bands = LayoutEngine::build_bands(
            &theme,
            true,
            true,
            LegendPosition::Top,
            legend,
            &AxisLayoutConstraints {
                bottom: 40.0,
                left: 50.0,
            },
            true,
        );

        // Top edge, from the canvas inwards: title first, then legend.
        assert_eq!(bands.top[0].kind, BandKind::Title);
        assert_eq!(bands.top[1].kind, BandKind::Legend);
    }

    /// Everything placed on a side must stay outside the panel.
    #[test]
    fn bands_never_cover_the_panel() {
        let theme = Theme::default();
        let content = Rect::new(50.0, 40.0, 800.0, 600.0);

        for position in [
            LegendPosition::Right,
            LegendPosition::Left,
            LegendPosition::Top,
            LegendPosition::Bottom,
        ] {
            for has_title in [false, true] {
                let bands = LayoutEngine::build_bands(
                    &theme,
                    has_title,
                    true,
                    position,
                    80.0,
                    &AxisLayoutConstraints {
                        bottom: 45.0,
                        left: 55.0,
                    },
                    true,
                );
                let plan = LayoutEngine::arrange(content, &bands, theme.min_panel_size);

                let overlaps = |a: &Rect, b: &Rect| {
                    let x = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
                    let y = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
                    x.max(0.0) * y.max(0.0) > 0.0
                };

                if let Some(title) = plan.title {
                    assert!(
                        !overlaps(&title, &plan.panel),
                        "{position:?}: title {title:?} covers the panel {:?}",
                        plan.panel
                    );
                }
                if let Some(legend) = plan.legend {
                    assert!(
                        !overlaps(&legend, &plan.panel),
                        "{position:?}: legend {legend:?} covers the panel {:?}",
                        plan.panel
                    );
                    if let Some(title) = plan.title {
                        assert!(
                            !overlaps(&title, &legend),
                            "{position:?}: title {title:?} covers the legend {legend:?}"
                        );
                    }
                }
            }
        }
    }

    /// A chart without a legend only gives up room for the axis.
    #[test]
    fn a_hidden_legend_reserves_nothing() {
        let theme = Theme::default();
        let bands = LayoutEngine::build_bands(
            &theme,
            false,
            false,
            LegendPosition::Right,
            0.0,
            &AxisLayoutConstraints {
                bottom: 30.0,
                left: 40.0,
            },
            true,
        );

        assert!(bands.top.is_empty());
        assert!(bands.left.iter().all(|b| b.kind == BandKind::YAxis));
    }
}
