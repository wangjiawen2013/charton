use super::flow::{self, Direction};
use crate::core::aesthetics::{AestheticMapping, GlobalAesthetics};
use crate::core::utils::estimate_text_width;
use crate::scale::ScaleDomain;
use crate::scale::Tick;
use crate::scale::mapper::VisualMapper;
use crate::theme::Theme;
use std::collections::BTreeMap;

/// Width of the square reserved for a legend symbol.
///
/// Every entry gets the same cell width so that symbols form a straight column
/// and labels start at the same offset, regardless of the glyph in use.
pub(crate) const ENTRY_CELL_WIDTH: f64 = 18.0;

/// Smallest length a guide may wrap its entries into. Guards against a budget
/// that has been eaten up completely by titles or margins, which would
/// otherwise make the packer emit one entry per line forever.
const MIN_ENTRY_BUDGET: f64 = 20.0;

/// Thickness (short side) of a continuous gradient bar, in pixels.
const COLORBAR_THICKNESS: f64 = 15.0;
/// A horizontal gradient bar stretches with the available width, within bounds.
const COLORBAR_WIDTH_MIN: f64 = 150.0;
const COLORBAR_WIDTH_MAX: f64 = 300.0;
/// A vertical gradient bar takes a share of the panel height instead, capped so
/// that it never dominates a tall chart.
const COLORBAR_HEIGHT_RATIO: f64 = 0.7;
const COLORBAR_HEIGHT_MAX: f64 = 200.0;
/// Number of tick marks a colour bar labels.
const COLORBAR_TICK_COUNT: usize = 5;

/// Geometry shared by every entry of a guide. All entries are the same size.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EntryMetrics {
    /// Width of the square that holds the symbol.
    pub(crate) cell: f64,
    /// Height of one entry row.
    pub(crate) row: f64,
}

/// Resolved geometry of a continuous gradient bar.
///
/// The bar is measured once, here, and then drawn verbatim. Renderers must not
/// recompute its size from the space available to them: doing so is what used to
/// make a colour bar a fixed 150px long regardless of the panel it belonged to.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ColorBarGeometry {
    /// Length along the bar, i.e. the axis the gradient runs down.
    pub(crate) length: f64,
    /// Thickness across the bar.
    pub(crate) thickness: f64,
}

impl EntryMetrics {
    const fn new(font_size: f64) -> Self {
        Self {
            cell: ENTRY_CELL_WIDTH,
            row: ENTRY_CELL_WIDTH.max(font_size),
        }
    }
}

/// Where one entry sits inside its block.
///
/// Coordinates are relative to the block's **top-left corner**, which is where the
/// block's title begins -- not where the first entry begins: `y` already includes
/// the height of the title strip plus the gap below it. Adding it to the block
/// origin therefore yields the top-left corner of the entry's own row, not its
/// centre (the renderer adds half a row for the symbol and the label baseline).
#[derive(Debug, Clone, Copy)]
pub(crate) struct EntryOffset {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

/// A guide after measurement: how much room it needs and where its entries go.
///
/// Measuring and placing happen here, once. Renderers consume [`Self::entries`]
/// verbatim instead of re-running the wrapping themselves -- duplicated wrapping
/// logic is exactly what used to let the measured and the drawn layout disagree.
#[derive(Debug, Clone)]
pub(crate) struct MeasuredGuide {
    pub(crate) size: GuideSize,
    pub(crate) metrics: EntryMetrics,
    /// Positions are relative to the block origin and already sit below the
    /// title. Empty for colour bars, which have no discrete entries.
    pub(crate) entries: Vec<EntryOffset>,
    /// Present only for colour bars, which carry a gradient instead of entries.
    pub(crate) colorbar: Option<ColorBarGeometry>,
}

/// Represents the physical rectangular area required by a Guide (Legend or ColorBar).
/// Used by the LayoutEngine to reserve space and calculate the final Plot Panel.
#[derive(Debug, Clone, Copy, Default)]
pub struct GuideSize {
    pub width: f64,
    pub height: f64,
}

/// The visual representation strategy for a data field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuideKind {
    /// A discrete list of symbols and labels. Used for categorical data,
    /// or when multiple aesthetics (e.g., Color + Shape) are merged.
    Legend,
    /// A continuous gradient strip. Used exclusively for continuous Color mappings.
    ColorBar,
}

/// Specification for a Guide (Legend or ColorBar), acting as the bridge
/// between abstract data scales and visual rendering instructions.
///
/// Following the "Grammar of Graphics" (like ggplot2), a single GuideSpec
/// consolidates multiple aesthetics (Color, Shape, Size) if they map to the same field.
pub struct GuideSpec {
    /// The title displayed above the guide (usually the data field name).
    pub title: String,
    /// The data field name this guide represents (e.g., "mpg", "class").
    pub field: String,
    /// Determines if this is rendered as a discrete list or a gradient bar.
    pub kind: GuideKind,
    /// The data range and type (Categorical or Continuous).
    pub domain: ScaleDomain,
    /// The collection of visual mappings tied to this specific field.
    pub mappings: Vec<AestheticMapping>,
}

impl GuideSpec {
    /// Constructs a GuideSpec and performs **Semantic Inference**:
    /// 1. If any mapping involves Size or Shape, it is forced to be a `Legend`.
    /// 2. Only if it is strictly a continuous Color mapping does it become a `ColorBar`.
    pub fn new(field: String, domain: ScaleDomain, mappings: Vec<AestheticMapping>) -> Self {
        let mut has_complex_geometry = false;
        let mut is_continuous_color = false;

        for m in &mappings {
            if let Some(mapper) = m.scale_impl.mapper() {
                match mapper {
                    // Size and Shape require discrete symbol keys
                    VisualMapper::Size { .. } | VisualMapper::Shape { .. } => {
                        has_complex_geometry = true;
                    }
                    // Continuous color can potentially use a gradient bar
                    VisualMapper::ContinuousColor { .. } => {
                        is_continuous_color = true;
                    }
                    _ => {}
                }
            }
        }
        // If it involves symbols (Shape/Size) or mixed channels, we use Legend mode.
        // ColorBar is reserved for pure continuous color mapping.
        let kind = if is_continuous_color && !has_complex_geometry {
            GuideKind::ColorBar
        } else {
            GuideKind::Legend
        };

        Self {
            title: field.clone(),
            field,
            kind,
            domain,
            mappings,
        }
    }

    /// Measures the guide and resolves where every entry goes.
    ///
    /// `direction` is the direction the surrounding legend strip packs its
    /// blocks in. A single guide always lays its own entries out along that same
    /// direction, so both levels share one packing routine.
    ///
    /// `main_budget` is the length available along the packing direction. The
    /// guide wraps its entries to stay within it, so the returned size does not
    /// exceed the budget -- unless a single entry is longer than the whole
    /// budget, which is unavoidable and left for the caller to handle.
    pub(crate) fn measure(
        &self,
        theme: &Theme,
        direction: Direction,
        main_budget: f64,
    ) -> MeasuredGuide {
        match self.kind {
            GuideKind::ColorBar => self.measure_colorbar(theme, direction, main_budget),
            GuideKind::Legend => self.measure_legend(theme, direction, main_budget),
        }
    }

    /// Height of the block title and the gap that separates it from the content.
    ///
    /// The title is slightly larger than the labels, hence the 1.1 factor.
    fn title_box(theme: &Theme) -> (f64, f64) {
        (theme.legend_label_size * 1.1, theme.legend_title_gap)
    }

    /// Measures a discrete legend: turns its labels into entry boxes, packs them,
    /// and wraps the whole thing in a title.
    ///
    /// # What a box is here
    ///
    /// At this level a box is **one entry**, i.e. one row of the legend, and it
    /// contains exactly what gets drawn for it:
    ///
    /// ```text
    ///   |<-- cell -->|<-- marker_text_gap -->|<----- label text ----->|
    ///   |     ●      |                      |      Category 00       |
    ///   |<----------------- entry_width (main/horizontal) --------->|
    ///   |<---------- entry_width (cross/vertical) --------->|
    /// ```
    ///
    /// So a box covers the symbol cell (18px), the gap after the symbol
    /// (`legend_marker_text_gap`), and the label text. It contains **no** spacing
    /// towards its neighbours: the gaps are passed to the packer separately. It
    /// also contains no title -- the title is not an entry, it is added around the
    /// packed entries at the end of this function.
    ///
    /// # What the budget is here
    ///
    /// `main_budget` is the length of the container handed down by the caller
    /// (ultimately the plot panel's extent along the packing direction). The title
    /// is always drawn *above* the entries, so it always consumes height. When the
    /// packing direction is vertical that height lies on the packing axis and has
    /// to be taken out of the budget; when it is horizontal the packing axis is
    /// the width, which the title does not constrain at all -- there it only puts
    /// a lower bound on the block width.
    fn measure_legend(
        &self,
        theme: &Theme,
        direction: Direction,
        main_budget: f64,
    ) -> MeasuredGuide {
        let font_size = theme.legend_label_size;
        let metrics = EntryMetrics::new(font_size);
        let (title_height, title_gap) = Self::title_box(theme);
        let title_width = estimate_text_width(&self.title, title_height);

        // What is left for the entries themselves once the title has been
        // accounted for (only relevant when the title sits on the packing axis).
        let entry_budget = match direction {
            Direction::Vertical => (main_budget - title_height - title_gap).max(MIN_ENTRY_BUDGET),
            Direction::Horizontal => main_budget,
        };

        // One box per label: symbol cell + gap + text, measured in both axes so
        // that `Direction` can decide which of the two is `main`.
        let extents: Vec<flow::Extent> = self
            .get_sampling_labels()
            .iter()
            .map(|label| {
                let entry_width = metrics.cell
                    + theme.legend_marker_text_gap
                    + estimate_text_width(label, font_size);
                direction.measure(entry_width, metrics.row)
            })
            .collect();

        // Entries of one guide are packed tightly, and wrapping into a new
        // line uses the *other* gap so that rows/columns stay readable.
        let (gap, line_gap) = match direction {
            Direction::Horizontal => (theme.legend_col_h_gap, theme.legend_item_v_gap),
            Direction::Vertical => (theme.legend_item_v_gap, theme.legend_col_h_gap),
        };
        let packed = flow::pack(&extents, entry_budget, gap, line_gap);

        // Add the title on top of the packed content: it extends the block along
        // the height axis, whichever physical axis that maps to.
        let (main, cross) = match direction {
            Direction::Vertical => (
                title_height + title_gap + packed.main_total,
                packed.cross_total,
            ),
            Direction::Horizontal => (
                packed.main_total,
                title_height + title_gap + packed.cross_total,
            ),
        };
        let (mut width, height) = direction.resolve(main, cross);
        // A title wider than the entries must widen the block, otherwise it
        // would be clipped.
        width = width.max(title_width);

        let entries = packed
            .offsets
            .iter()
            .map(|&(main, cross)| {
                let (x, y) = direction.resolve(main, cross);
                EntryOffset {
                    x,
                    y: y + title_height + title_gap,
                }
            })
            .collect();

        MeasuredGuide {
            size: GuideSize { width, height },
            metrics,
            entries,
            colorbar: None,
        }
    }

    /// Measures a continuous gradient bar plus the tick labels around it.
    fn measure_colorbar(
        &self,
        theme: &Theme,
        direction: Direction,
        main_budget: f64,
    ) -> MeasuredGuide {
        let font_size = theme.legend_label_size;
        let (title_height, title_gap) = Self::title_box(theme);
        let title_width = estimate_text_width(&self.title, title_height);
        let ticks = self.colorbar_ticks();
        let max_label_width = ticks
            .iter()
            .map(|tick| estimate_text_width(&tick.label, font_size))
            .fold(0.0, f64::max);

        // The bar runs along the packing direction: a horizontal bar stretches
        // with the available width, a vertical one takes a share of the panel's
        // height so that it scales with the plot instead of being a fixed size.
        let bar_length = match direction {
            Direction::Horizontal => main_budget.clamp(COLORBAR_WIDTH_MIN, COLORBAR_WIDTH_MAX),
            Direction::Vertical => (main_budget * COLORBAR_HEIGHT_RATIO).min(COLORBAR_HEIGHT_MAX),
        };
        let geometry = ColorBarGeometry {
            length: bar_length,
            thickness: COLORBAR_THICKNESS,
        };

        // Tick labels sit under a horizontal bar and beside a vertical one.
        let (main, cross) = match direction {
            Direction::Horizontal => (
                bar_length,
                COLORBAR_THICKNESS + theme.tick_label_padding + font_size,
            ),
            Direction::Vertical => (
                bar_length,
                COLORBAR_THICKNESS + theme.legend_marker_text_gap + max_label_width,
            ),
        };
        let (mut width, height) = direction.resolve(main, cross);
        width = width.max(title_width);

        MeasuredGuide {
            // The title adds another strip of height on top of the bar.
            size: GuideSize {
                width,
                height: height + title_height + title_gap,
            },
            metrics: EntryMetrics::new(font_size),
            entries: Vec::new(),
            colorbar: Some(geometry),
        }
    }

    /// The tick marks a colour bar shows.
    ///
    /// Both the measurement (which needs the widest label) and the renderer
    /// (which draws them) use this, so the two can never disagree about how much
    /// room the labels need. Unlike [`Self::get_sampling_labels`] the labels keep
    /// the scale's own "pretty" formatting: a colour bar shows axis-like values,
    /// not data values.
    pub(crate) fn colorbar_ticks(&self) -> Vec<Tick> {
        let Some(mapping) = self.mappings.first() else {
            return Vec::new();
        };

        let mut ticks = mapping.scale_impl.suggest_ticks(COLORBAR_TICK_COUNT);
        // Fall back to plain sampling when the pretty algorithm is too sparse.
        if ticks.len() < 3 && !matches!(self.domain, ScaleDomain::Discrete(_)) {
            ticks = mapping.scale_impl.sample_n(COLORBAR_TICK_COUNT);
        }
        ticks
    }

    /// Extracts string labels from the underlying Scale implementation and
    /// enforces uniform decimal precision for visual alignment.
    ///
    /// This method ensures that all labels in a legend block share the same number
    /// of decimal places, preventing jagged text alignment (e.g., ensuring "20.0"
    /// isn't shortened to "20" when appearing alongside "16.3").
    pub(crate) fn get_sampling_labels(&self) -> Vec<String> {
        if let Some(first_mapping) = self.mappings.first() {
            // 1. Define target density (e.g., we want 5 circles for Size)
            let count = match self.kind {
                GuideKind::ColorBar => 5,
                GuideKind::Legend => {
                    if let ScaleDomain::Discrete(ref v) = self.domain {
                        v.len()
                    } else {
                        5
                    }
                }
            };

            // 2. Retrieve raw ticks from the scale (Pretty algorithm or Sample_n)
            let mut ticks = first_mapping.scale_impl.suggest_ticks(count);

            // Fallback to force-sampling if the pretty algorithm returns insufficient points
            if ticks.len() < 3 && !matches!(self.domain, ScaleDomain::Discrete(_)) {
                ticks = first_mapping.scale_impl.sample_n(count);
            }

            // A formatted scale already produced the final text; the alignment
            // pass below would overwrite it.
            if first_mapping.scale_impl.label_formatter().is_some() {
                return ticks.into_iter().map(|t| t.label).collect();
            }

            // 3. --- Uniform Precision Logic ---

            // Check if we are dealing with a numeric (non-categorical) scale
            if !matches!(self.domain, ScaleDomain::Discrete(_)) {
                // Determine the maximum precision needed across all sampled points.
                // We look for the most specific decimal place to ensure no data is lost.
                let mut max_precision = 0;
                let has_fractions = ticks
                    .iter()
                    .any(|t| (t.value - t.value.floor()).abs() > 1e-9);

                if has_fractions {
                    for tick in &ticks {
                        // Find how many decimals this specific number actually uses
                        let s = format!("{}", tick.value);
                        if let Some(pos) = s.find('.') {
                            let p = s.len() - pos - 1;
                            if p > max_precision {
                                max_precision = p;
                            }
                        }
                    }
                    // For aesthetics, we force at least 1 decimal if any fractions exist
                    max_precision = max_precision.clamp(1, 4);
                }

                // Re-format all ticks using the discovered global precision
                ticks
                    .into_iter()
                    .map(|t| format!("{:.1$}", t.value, max_precision))
                    .collect()
            } else {
                // For categorical data, use labels exactly as provided by the scale
                ticks.into_iter().map(|t| t.label).collect()
            }
        } else {
            // Fallback for empty mappings
            match &self.domain {
                ScaleDomain::Discrete(v) => v.clone(),
                _ => Vec::new(),
            }
        }
    }

    /// Returns the raw Tick objects (value + aligned label) used for sampling.
    pub(crate) fn get_sampling_ticks(&self) -> Vec<Tick> {
        if let Some(first_mapping) = self.mappings.first() {
            let count = 5; // Target density
            let mut ticks = first_mapping.scale_impl.suggest_ticks(count);

            if ticks.len() < 3 && !matches!(self.domain, ScaleDomain::Discrete(_)) {
                ticks = first_mapping.scale_impl.sample_n(count);
            }

            // A formatted scale already produced the final text.
            if first_mapping.scale_impl.label_formatter().is_some() {
                return ticks;
            }

            // Apply the precision alignment we discussed earlier
            let mut max_p = 0;
            let has_fractions = ticks
                .iter()
                .any(|t| (t.value - t.value.floor()).abs() > 1e-9);
            if has_fractions {
                for t in &ticks {
                    let s = format!("{}", t.value);
                    if let Some(pos) = s.find('.') {
                        max_p = max_p.max(s.len() - pos - 1);
                    }
                }
                max_p = max_p.clamp(1, 4);
            }

            // Update labels in the ticks themselves
            for t in &mut ticks {
                t.label = format!("{:.1$}", t.value, max_p);
            }
            ticks
        } else {
            Vec::new()
        }
    }
}

/// Core manager responsible for grouping aesthetics and generating GuideSpecs.
pub struct GuideManager;

impl GuideManager {
    /// Orchestrates the collection of global aesthetics into a consolidated set of GuideSpecs.
    ///
    /// This function implements the "Legend Merging" logic. According to the Grammar of Graphics,
    /// if multiple aesthetics (e.g., Color, Shape, and Size) are mapped to the same data field,
    /// they should be unified into a single visual guide (Legend) to avoid redundancy and
    /// improve scannability.
    ///
    /// # Logic Flow:
    /// 1. Group all active `AestheticMapping` instances by their `field` name.
    /// 2. Use a `BTreeMap` to ensure that guides are generated in a stable, alphabetical order.
    /// 3. Pass the consolidated mappings to `GuideSpec::new`, which infers the visual
    ///    type (Legend vs. ColorBar) based on the combined mapping properties.
    pub fn collect_guides(aesthetics: &GlobalAesthetics) -> Vec<GuideSpec> {
        // We group mappings by field name. The tuple contains the inferred ScaleDomain
        // and the list of mappings associated with that field.
        let mut field_map: BTreeMap<String, (ScaleDomain, Vec<AestheticMapping>)> = BTreeMap::new();

        // Helper closure to safely extract and group active mappings.
        let mut collect = |mapping: &Option<AestheticMapping>| {
            if let Some(m) = mapping {
                let entry = field_map.entry(m.field.clone()).or_insert_with(|| {
                    // We capture the domain from the first mapping encountered for this field.
                    // In a valid plot, all aesthetics sharing a field should share the same scale logic.
                    (m.scale_impl.get_domain_enum(), Vec::new())
                });
                entry.1.push(m.clone());
            }
        };

        // --- Phase 1: Aggregation ---
        // Scan standard aesthetic channels. Order of collection doesn't affect the
        // result because BTreeMap handles the final sorting.
        collect(&aesthetics.color);
        collect(&aesthetics.shape);
        collect(&aesthetics.size);

        // --- Phase 2: Specification ---
        // Convert each field group into a high-level GuideSpec.
        // The GuideSpec will later use the `sample_n` logic implemented in the scales
        // to generate the 5 visual steps (circles/colors) you requested.
        field_map
            .into_iter()
            .map(|(field, (domain, mappings))| {
                // Use the mapping's own title when it has one (set with
                // `with_color_label`, and so on), otherwise the field name.
                let title = mappings
                    .first()
                    .and_then(|m| m.title.clone())
                    .unwrap_or_else(|| field.clone());
                // GuideSpec::new performs semantic inference to decide if this
                // should be rendered as a discrete Legend or a continuous ColorBar.
                let mut spec = GuideSpec::new(field, domain, mappings);
                spec.title = title;
                spec
            })
            .collect()
    }
}

/// Defines where the legend block is placed relative to the chart.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LegendPosition {
    Top,
    Bottom,
    Left,
    #[default]
    Right,
    None,
}
