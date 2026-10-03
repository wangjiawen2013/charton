//! Violin geometry: turn raw measurements into the outline of a violin plot.
//!
//! # What a violin plot is
//!
//! A violin is a smooth picture of *where the data is dense*. Tall parts of the
//! outline mean "many values here", narrow parts mean "few values here". It is
//! built in three steps:
//!
//! 1. **Estimate the density** of the values (the same spirit as a histogram,
//!    but smoothed). This module reuses [`crate::stats::kde`].
//! 2. **Mirror the curve** so the violin is symmetric around its centre line.
//! 3. **Close the two mirrored sides into a polygon** that any polygon renderer
//!    can draw.
//!
//! Nothing here is violin-specific at the rendering level: the output is a plain
//! `(x, y, path_group)` table, exactly the kind of data a polygon mark expects.
//!
//! # How this compares with the Vega-Lite / Altair recipe
//!
//! Vega-Lite draws a violin with the ordinary density transform plus an area
//! mark whose values are stacked around the centre (`stack: "center"`). Charton
//! supports that path too — see [`crate::encode::y::StackMode::Mirror`] — and it
//! is the simplest way to draw a single or faceted violin:
//!
//! ```rust,ignore
//! chart!(iris)?
//!     .transform_density(DensityTransform::new("sepal_length").with_groupby("species"))?
//!     .mark_area()?
//!     .encode((alt::x("sepal_length"),
//!              alt::y("density").with_stack("mirror"),
//!              alt::color("species")))?
//!     .facet(FacetSpec::wrap("species"))
//!     .coord_flip()
//!     .save("violin.svg")?;
//! ```
//!
//! The density transform, however, puts the *measured value* on x and can only
//! group by a single field. A **dodged** violin (several violins side by side
//! inside one category) needs the category on x and a two-field grouping
//! `(category, group)`. That case is what this transform provides, mirroring
//! ggplot2's `stat_ydensity`. It reuses the same KDE core as the density
//! transform, so no statistics are duplicated.
//!
//! # Grouping
//!
//! Two different ideas are often both called "grouping":
//!
//! * **Facets** split the picture into separate panels. One violin per panel,
//!   all centred. Achieve this by grouping the density estimate (so each facet
//!   gets its own curve) and then faceting on the same field.
//! * **Dodging** places several violins *side by side* inside one category.
//!   Achieve this by giving the transform a `category` field and a side-by-side
//!   [`Position`].
//!
//! The mirroring and dodging are pure data-space arithmetic, so the same polygon
//! geometry serves every layout.

use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset, get_quantile};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::position::Position;
use crate::stats::kde::{BandwidthType, Kde, KernelType};
use ahash::AHashMap;

/// How the width of a violin is scaled relative to the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolinScale {
    /// Every violin gets the same maximum width. Great when you care about the
    /// *shape* more than the amount of data behind it.
    Width,
    /// Every violin encloses the same area (the default). A violin with more
    /// samples looks slightly wider, but the total area stays comparable.
    Area,
    /// Width grows with the number of samples, so a violin made from twice as
    /// many rows is visibly larger.
    Count,
}

impl From<&str> for ViolinScale {
    fn from(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "count" => Self::Count,
            "area" => Self::Area,
            _ => Self::Width,
        }
    }
}

/// Configuration for [`Chart::transform_violin`].
///
/// The defaults describe a single, centred violin: one density curve mirrored
/// around `x = 0`.
#[derive(Debug, Clone)]
pub struct ViolinTransform {
    /// Numeric column holding the measurements the violin should describe.
    pub(crate) value: String,

    /// Optional column whose values become the x position (the violin's centre).
    ///
    /// Leave it empty for a single violin, or for the faceted layout where each
    /// panel shows one violin at the centre.
    pub(crate) category: Option<String>,

    /// Optional column that splits the data into several density curves.
    ///
    /// With a [`Position::Dodge`] the curves sit side by side; with
    /// [`Position::Identity`] they overlap, which is what you want when they are
    /// later separated by facets.
    pub(crate) groupby: Option<String>,

    /// Names of the three produced columns: `[x, y, path_group]`.
    pub(crate) as_: [String; 3],

    /// Smoothing rule for the density estimate.
    pub(crate) bandwidth: BandwidthType,

    /// Shape of the smoothing bump.
    pub(crate) kernel: KernelType,

    /// How several curves inside one category are arranged.
    pub(crate) position: Position,

    /// When `true`, draw two groups as the left and right halves of one violin
    /// instead of dodging them side by side.
    pub(crate) split: bool,

    /// The widest a single violin may be, in category steps. This is the mark's
    /// own cap, matching the box plot and point marks; the whole group may be
    /// wider when several violins are dodged side by side. See `span`.
    pub(crate) width: f64,

    /// Total width of one category's violin group, in category steps. A single
    /// violin fills `min(width, span)`; a dodged group fills at most `span`.
    /// Defaults to `0.7`, matching the box plot and point marks.
    pub(crate) span: f64,

    /// Width of the inner box, as a fraction of the violin slot.
    pub(crate) box_width: f64,

    /// Width scaling rule (see [`ViolinScale`]).
    pub(crate) scale: ViolinScale,

    /// How many points describe one side of the outline. Larger is smoother.
    pub(crate) steps: usize,

    /// When `true`, the outline stops at the smallest and largest observation.
    /// When `false`, the tails are extended a little so the curve closes softly.
    pub(crate) trim: bool,

    /// Whether to also emit `q1`, `median` and `q3` columns (useful for drawing
    /// an inner box on top of the violin).
    pub(crate) quantiles: bool,
}

impl ViolinTransform {
    /// Creates a violin transform for `value_field`.
    ///
    /// Defaults: one centred violin, `Area` scaling, 200 points per side,
    /// trimmed tails, quantiles enabled, Gaussian kernel, Scott's bandwidth.
    pub fn new(value_field: impl Into<String>) -> Self {
        Self {
            value: value_field.into(),
            category: None,
            groupby: None,
            as_: ["x".to_string(), "y".to_string(), "violin_id".to_string()],
            bandwidth: BandwidthType::Scott,
            kernel: KernelType::Normal,
            position: Position::Identity,
            split: false,
            width: 0.5,
            span: 0.7,
            box_width: 0.2,
            scale: ViolinScale::Area,
            steps: 200,
            trim: true,
            quantiles: true,
        }
    }

    /// Sets the column that provides each violin's x position.
    pub fn with_category(mut self, field: impl Into<String>) -> Self {
        self.category = Some(field.into());
        self
    }

    /// Sets the column that splits the data into several density curves.
    pub fn with_group(mut self, field: impl Into<String>) -> Self {
        self.groupby = Some(field.into());
        self
    }

    /// Renames the produced `[x, y, path_group]` columns.
    pub fn with_as(
        mut self,
        x: impl Into<String>,
        y: impl Into<String>,
        path_group: impl Into<String>,
    ) -> Self {
        self.as_ = [x.into(), y.into(), path_group.into()];
        self
    }

    /// Chooses the smoothing rule.
    pub const fn with_bandwidth(mut self, bandwidth: BandwidthType) -> Self {
        self.bandwidth = bandwidth;
        self
    }

    /// Chooses the smoothing kernel.
    pub const fn with_kernel(mut self, kernel: KernelType) -> Self {
        self.kernel = kernel;
        self
    }

    /// Chooses how several violins inside one category are arranged.
    pub const fn with_position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Draws two groups as the left and right halves of a single violin.
    ///
    /// The first group grows to the right of the category centre and the second
    /// to the left. Colour the result by group to make both halves visible.
    /// This ignores the [`Position`], because the halves share one centre.
    pub const fn with_split(mut self, split: bool) -> Self {
        self.split = split;
        self
    }

    /// Sets the maximum width of a single violin, in category steps.
    pub fn with_width(mut self, width: f64) -> Self {
        self.width = width.clamp(0.0, 1.0);
        self
    }

    /// Sets the total width of a category's violin group, in category steps.
    ///
    /// Defaults to `0.7`, matching the box plot and point marks, so a dodged
    /// violin group lines up with a dodged box or scatter.
    pub fn with_span(mut self, span: f64) -> Self {
        self.span = span.clamp(0.0, 1.0);
        self
    }

    /// Sets the inner box width as a fraction of the violin slot (0.0–1.0).
    pub fn with_box_width(mut self, width: f64) -> Self {
        self.box_width = width.clamp(0.0, 1.0);
        self
    }

    /// Chooses the width scaling rule.
    pub const fn with_scale(mut self, scale: ViolinScale) -> Self {
        self.scale = scale;
        self
    }

    /// Sets how many points describe one side of the outline.
    pub fn with_steps(mut self, steps: usize) -> Self {
        self.steps = steps.max(8);
        self
    }

    /// Keeps (`true`) or extends (`false`) the tails of the outline.
    pub const fn with_trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }

    /// Enables or disables the `q1` / `median` / `q3` output columns.
    pub const fn with_quantiles(mut self, quantiles: bool) -> Self {
        self.quantiles = quantiles;
        self
    }
}

/// One estimated curve plus the summary statistics that travel with it.
struct CurveGeometry {
    /// Index of the category this curve belongs to.
    category_index: usize,
    /// Index of the group (lane) inside the category.
    group_index: usize,
    /// Human-readable labels, echoed into the output for faceting/colouring.
    category_label: String,
    group_label: String,
    /// Evaluation points (the value axis).
    points: Vec<f64>,
    /// Estimated density at each point.
    density: Vec<f64>,
    /// How many observations produced this curve.
    count: f64,
    /// Largest density on this curve (used for width scaling).
    peak: f64,
    /// Summary statistics, repeated on every output row.
    q1: f64,
    median: f64,
    q3: f64,
}

/// The full layout of a violin plot: every curve plus the shared lane geometry.
struct ViolinGeometry {
    curves: Vec<CurveGeometry>,
    /// Centre shift of each lane, in category steps.
    lane_offsets: Vec<f64>,
    /// Width one violin may use, in category steps.
    slot: f64,
    /// Largest density across all curves (for `Area` scaling).
    global_peak: f64,
    /// Largest `density * count` across all curves (for `Count` scaling).
    global_count_peak: f64,
}

impl ViolinGeometry {
    /// The x centre of one curve: its category plus its dodge lane.
    fn centre(&self, curve: &CurveGeometry) -> f64 {
        curve.category_index as f64 + self.lane_offsets[curve.group_index]
    }

    /// The stable id shared by all vertices of one violin.
    fn id(&self, curve: &CurveGeometry) -> String {
        format!("{}::{}", curve.category_label, curve.group_label)
    }

    /// Converts a density reading into a half-width, honouring the chosen scale.
    fn half_width(&self, curve: &CurveGeometry, density: f64, scale: ViolinScale) -> f64 {
        let half_slot = (self.slot / 2.0).max(0.0);
        let factor = match scale {
            ViolinScale::Width => {
                if curve.peak > 0.0 {
                    half_slot / curve.peak
                } else {
                    0.0
                }
            }
            ViolinScale::Area => {
                if self.global_peak > 0.0 {
                    half_slot / self.global_peak
                } else {
                    0.0
                }
            }
            ViolinScale::Count => half_slot / self.global_count_peak * curve.count,
        };
        density * factor
    }
}

/// Collects the observations, estimates every curve and solves the lane layout.
///
/// This is the shared "stat" of the violin plot. Both the outline and the inner
/// box are derived from the same geometry, so they always line up.
fn build_geometry(
    data: &Dataset,
    params: &ViolinTransform,
) -> Result<ViolinGeometry, ChartonError> {
    let row_count = data.height();
    let value_col = data.column(&params.value)?;

    // The order of categories and groups is the order they first appear, which
    // keeps colours and facets stable from run to run.
    let category_order: Vec<Option<String>> = match &params.category {
        Some(field) => data
            .column(field)?
            .unique_values()
            .into_iter()
            .map(Some)
            .collect(),
        None => vec![None],
    };

    let group_order: Vec<Option<String>> = match &params.groupby {
        Some(field) => data
            .column(field)?
            .unique_values()
            .into_iter()
            .map(Some)
            .collect(),
        None => vec![None],
    };

    // A key is `(category, group)`; `None` means "there is only one".
    let mut cells: AHashMap<(Option<String>, Option<String>), Vec<f64>> = AHashMap::new();
    for i in 0..row_count {
        let Some(value) = value_col.get(i).to_f64() else {
            continue; // Skip nulls; they carry no density information.
        };
        if !value.is_finite() {
            continue;
        }

        let category = params.category.as_ref().map(|field| {
            data.get(field, i)
                .to_string()
                .unwrap_or_else(|| "null".to_string())
        });
        let group = params.groupby.as_ref().map(|field| {
            data.get(field, i)
                .to_string()
                .unwrap_or_else(|| "null".to_string())
        });

        cells.entry((category, group)).or_default().push(value);
    }

    // Estimate one density curve per cell.
    let mut curves: Vec<CurveGeometry> = Vec::new();
    for (category_index, category) in category_order.iter().enumerate() {
        for (group_index, group) in group_order.iter().enumerate() {
            let Some(values) = cells.get(&(category.clone(), group.clone())) else {
                continue;
            };
            if values.is_empty() {
                continue;
            }

            curves.push(estimate_curve(
                values,
                category_index,
                group_index,
                category.as_deref().unwrap_or("all"),
                group.as_deref().unwrap_or("all"),
                params,
            ));
        }
    }

    let global_peak = curves.iter().map(|c| c.peak).fold(0.0_f64, f64::max);
    let global_count_peak = curves
        .iter()
        .map(|c| c.peak * c.count)
        .fold(0.0_f64, f64::max)
        .max(f64::MIN_POSITIVE);

    // Lane positions are solved once: every category uses the same lanes, so
    // dodged violins line up across categories. A split violin instead places
    // every group on the same centre line.
    // Solve the lanes exactly as the box plot and point marks do, so a dodged
    // violin group has the same width and spacing as a dodged box or scatter.
    let lane_count = group_order.len().max(1);
    let (lane_offsets, slot) = if params.split {
        // Both halves share the centre line, so there is no dodge to solve.
        (vec![0.0; lane_count], params.width)
    } else {
        let item_width = params
            .position
            .item_width(lane_count as f64, params.span, params.width);
        let offsets = (0..lane_count)
            .map(|i| {
                params
                    .position
                    .offset(i as f64, lane_count as f64, item_width)
            })
            .collect();
        (offsets, item_width)
    };

    Ok(ViolinGeometry {
        curves,
        lane_offsets,
        slot,
        global_peak,
        global_count_peak,
    })
}

impl<T: Mark> Chart<T> {
    /// Rewrites the dataset into a violin-polygon table.
    ///
    /// After the call the dataset contains one outline per `(category, group)`
    /// combination, ready to be drawn by a polygon mark:
    ///
    /// ```rust,ignore
    /// chart!(ds)?
    ///     .transform_violin(ViolinTransform::new("sepal_length"))?
    ///     .mark_polygon()?
    ///     .encode((alt::x("x"), alt::y("y"), alt::path_group("violin_id")))?
    ///     .save("violin.svg")?;
    /// ```
    pub fn transform_violin(mut self, params: ViolinTransform) -> Result<Self, ChartonError> {
        let geometry = build_geometry(&self.data, &params)?;

        let mut out_x = Vec::new();
        let mut out_y = Vec::new();
        let mut out_id = Vec::new();
        let mut out_category = Vec::new();
        let mut out_group = Vec::new();
        let mut out_q1 = Vec::new();
        let mut out_median = Vec::new();
        let mut out_q3 = Vec::new();

        for curve in &geometry.curves {
            let centre = geometry.centre(curve);
            let id = geometry.id(curve);

            // One closure keeps the eight output columns in lock-step.
            let mut push = |x: f64, y: f64| {
                out_x.push(x);
                out_y.push(y);
                out_id.push(id.clone());
                out_category.push(curve.category_label.clone());
                out_group.push(curve.group_label.clone());
                out_q1.push(curve.q1);
                out_median.push(curve.median);
                out_q3.push(curve.q3);
            };

            if params.split {
                // A split violin draws one half per group, both sharing the
                // category centre: the first group grows to the right, the
                // second to the left. Coloured by group, the halves form one
                // violin split down the middle.
                let side = if curve.group_index % 2 == 0 {
                    1.0
                } else {
                    -1.0
                };
                let (first, last) = match (curve.points.first(), curve.points.last()) {
                    (Some(first), Some(last)) => (*first, *last),
                    _ => continue,
                };

                push(centre, first);
                for (point, density) in curve.points.iter().zip(curve.density.iter()) {
                    let half = geometry.half_width(curve, *density, params.scale);
                    push(centre + side * half, *point);
                }
                push(centre, last);
            } else {
                // Full violin: the right side runs up, the left side walks back
                // down, closing the polygon symmetrically.
                for (point, density) in curve.points.iter().zip(curve.density.iter()) {
                    let half = geometry.half_width(curve, *density, params.scale);
                    push(centre + half, *point);
                }
                for (point, density) in curve.points.iter().zip(curve.density.iter()).rev() {
                    let half = geometry.half_width(curve, *density, params.scale);
                    push(centre - half, *point);
                }
            }
        }

        let mut new_data = Dataset::new();
        new_data.add_column(
            &params.as_[0],
            ColumnVector::Float64 {
                data: out_x,
                validity: None,
            },
        )?;
        new_data.add_column(
            &params.as_[1],
            ColumnVector::Float64 {
                data: out_y,
                validity: None,
            },
        )?;
        new_data.add_column(
            &params.as_[2],
            ColumnVector::String {
                data: out_id,
                validity: None,
            },
        )?;
        if let Some(field) = &params.category {
            new_data.add_column(
                field,
                ColumnVector::String {
                    data: out_category,
                    validity: None,
                },
            )?;
        }
        if let Some(field) = &params.groupby {
            new_data.add_column(
                field,
                ColumnVector::String {
                    data: out_group,
                    validity: None,
                },
            )?;
        }

        if params.quantiles {
            let y = &params.as_[1];
            for (suffix, column) in [("q1", out_q1), ("median", out_median), ("q3", out_q3)] {
                new_data.add_column(
                    format!("{y}_{suffix}"),
                    ColumnVector::Float64 {
                        data: column,
                        validity: None,
                    },
                )?;
            }
        }

        self.data = new_data;
        Ok(self)
    }

    /// Rewrites the dataset into the **inner box** of a violin plot.
    ///
    /// Draw this as a second polygon layer on top of [`Chart::transform_violin`].
    /// Both transforms run the same statistics and the same lane layout, so the
    /// box always sits exactly over its violin:
    ///
    /// ```rust,ignore
    /// let outline = chart!(ds)?.transform_violin(params.clone())?.mark_polygon()?
    ///     .configure_geoshape(|m| m.with_fill("#cfe3f3"))?
    ///     .encode((alt::x("x"), alt::y("y"), alt::path_group("violin_id")))?;
    ///
    /// let box = chart!(ds)?.transform_violin_box(params)?.mark_polygon()?
    ///     .configure_geoshape(|m| m.with_fill("white").with_stroke("black"))?
    ///     .encode((alt::x("x"), alt::y("y"), alt::path_group("violin_id")))?;
    ///
    /// outline.and(box).save("violin_with_box.svg")?;
    /// ```
    ///
    /// The output uses the same `[x, y, path_group]` column names, plus a
    /// `violin_part` column whose values are `"box"` and `"median"`.
    pub fn transform_violin_box(mut self, params: ViolinTransform) -> Result<Self, ChartonError> {
        let geometry = build_geometry(&self.data, &params)?;

        let box_half = (geometry.slot * params.box_width / 2.0).max(0.0);
        let median_half = (geometry.slot * 0.02 / 2.0).max(0.0);

        let mut out_x = Vec::new();
        let mut out_y = Vec::new();
        let mut out_id = Vec::new();
        let mut out_part = Vec::new();
        let mut out_category = Vec::new();
        let mut out_group = Vec::new();

        for curve in &geometry.curves {
            let centre = geometry.centre(curve);
            let id = geometry.id(curve);

            let mut push = |x: f64, y: f64, part: &str, id: &str| {
                out_x.push(x);
                out_y.push(y);
                out_id.push(id.to_string());
                out_part.push(part.to_string());
                out_category.push(curve.category_label.clone());
                out_group.push(curve.group_label.clone());
            };

            // The inter-quartile box: a thin rectangle from q1 to q3.
            let box_id = format!("{id}::box");
            push(centre - box_half, curve.q1, "box", &box_id);
            push(centre + box_half, curve.q1, "box", &box_id);
            push(centre + box_half, curve.q3, "box", &box_id);
            push(centre - box_half, curve.q3, "box", &box_id);
            push(centre - box_half, curve.q1, "box", &box_id);

            // The median: a very thin rectangle so it can be a polygon too.
            let median_id = format!("{id}::median");
            push(
                centre - box_half,
                curve.median - median_half,
                "median",
                &median_id,
            );
            push(
                centre + box_half,
                curve.median - median_half,
                "median",
                &median_id,
            );
            push(
                centre + box_half,
                curve.median + median_half,
                "median",
                &median_id,
            );
            push(
                centre - box_half,
                curve.median + median_half,
                "median",
                &median_id,
            );
            push(
                centre - box_half,
                curve.median - median_half,
                "median",
                &median_id,
            );
        }

        let mut new_data = Dataset::new();
        new_data.add_column(
            &params.as_[0],
            ColumnVector::Float64 {
                data: out_x,
                validity: None,
            },
        )?;
        new_data.add_column(
            &params.as_[1],
            ColumnVector::Float64 {
                data: out_y,
                validity: None,
            },
        )?;
        new_data.add_column(
            &params.as_[2],
            ColumnVector::String {
                data: out_id,
                validity: None,
            },
        )?;
        new_data.add_column(
            "violin_part",
            ColumnVector::String {
                data: out_part,
                validity: None,
            },
        )?;
        if let Some(field) = &params.category {
            new_data.add_column(
                field,
                ColumnVector::String {
                    data: out_category,
                    validity: None,
                },
            )?;
        }
        if let Some(field) = &params.groupby {
            new_data.add_column(
                field,
                ColumnVector::String {
                    data: out_group,
                    validity: None,
                },
            )?;
        }

        self.data = new_data;
        Ok(self)
    }
}

/// Estimates one density curve and reads off its summary statistics.
///
/// The evaluation grid is the span of the observations. When `trim` is false the
/// span is widened a little on both sides so the outline tapers to (almost) zero
/// instead of ending with a blunt cut.
fn estimate_curve(
    values: &[f64],
    category_index: usize,
    group_index: usize,
    category_label: &str,
    group_label: &str,
    params: &ViolinTransform,
) -> CurveGeometry {
    let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
    for &value in values {
        min = min.min(value);
        max = max.max(value);
    }

    // A constant column has no spread; give it a tiny symmetric window so the
    // density estimate and the polygon remain finite.
    if (max - min).abs() < 1e-12 {
        let pad = if min == 0.0 { 1.0 } else { min.abs() * 0.1 };
        min -= pad;
        max += pad;
    }

    if !params.trim {
        // Reuse the classic 30 % tail extension used by the density transform.
        let lo = 1.3 * min - 0.3 * max;
        let hi = 1.3 * max - 0.3 * min;
        min = lo;
        max = hi;
    }

    let steps = params.steps.max(8);
    let step_size = (max - min) / (steps.saturating_sub(1)) as f64;
    let points: Vec<f64> = (0..steps).map(|i| min + i as f64 * step_size).collect();

    let kde = Kde::new(values.to_vec(), params.bandwidth, params.kernel);
    let density = kde.pdf(&points);
    let peak = density.iter().copied().fold(0.0_f64, f64::max);

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    CurveGeometry {
        category_index,
        group_index,
        category_label: category_label.to_string(),
        group_label: group_label.to_string(),
        points,
        density,
        count: values.len() as f64,
        peak,
        q1: get_quantile(&sorted, 0.25),
        median: get_quantile(&sorted, 0.5),
        q3: get_quantile(&sorted, 0.75),
    }
}
