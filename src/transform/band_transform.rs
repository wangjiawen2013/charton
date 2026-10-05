//! Band geometry: turn a centre, a value and a width into a symmetric polygon.
//!
//! A *band* is a shape that is mirror-symmetric around a centre line. For every
//! point `v` along the value axis it spans `centre - width(v)` to
//! `centre + width(v)`; joining the right edge upward and the left edge back
//! down closes it into a polygon any polygon renderer can fill:
//!
//! ```text
//!     centre + width(v)   ─┐
//!                          │  one closed polygon per (centre, lane)
//!     centre - width(v)   ─┘
//! ```
//!
//! A violin, a confidence ribbon and a funnel are all bands; they differ only
//! in where the `width` comes from. That statistic lives in another transform,
//! so this one stays purely geometric. The standard violin recipe chains the
//! two:
//!
//! ```rust,ignore
//! chart!(ds)?
//!     .transform_density(
//!         DensityTransform::new("score")
//!             .with_as("score", "density")
//!             .with_groupbys(["category", "group"]),
//!     )?
//!     .transform_band(
//!         BandTransform::new("score", "density")
//!             .with_center("category")
//!             .with_group("group")
//!             .with_position(Position::dodge()),
//!     )?
//!     .mark_polygon()?
//!     .encode((
//!         alt::x("x").with_category_labels("category"),
//!         alt::y("y"),
//!         alt::path_group("path_group"),
//!         alt::color("group"),
//!     ))?;
//! ```
//!
//! # Placement
//!
//! When several bands share one category they sit side by side. That layout is
//! owned by [`Position`], the same data-space adjustment the box, bar, error
//! bar and point marks use, so a dodged violin lines up with a dodged box or
//! scatter. `transform_band` resolves the lanes once and bakes the offsets into
//! the polygon, so the renderer needs no special case.
//!
//! # Side and split
//!
//! [`with_side`](BandTransform::with_side) chooses which bank of the river a
//! band occupies: `Both` (the symmetric violin), or `Left` / `Right` for a
//! one-sided band whose flat edge is the centre line. This is a geometry flag,
//! independent of lanes and scale.
//!
//! [`with_split(true)`](BandTransform::with_split) is the two-group split
//! violin: the first group takes the right bank, the second the left, sharing
//! the centre line. It only applies when two groups exist; a single group stays
//! symmetric, so use `with_side` for a genuinely one-sided band.
//!
//! # Overlap
//!
//! By default a band stays inside its lane.
//! [`with_overlap`](BandTransform::with_overlap) multiplies its width so it can
//! grow past the lane and overlap its neighbour — overlapping violins, and the
//! stacked look of a ridgeline when combined with a one-sided `with_side`.

use super::ensure_distinct_columns;
use super::lane_layout::{LaneLayoutOptions, build_lane_layout};
use crate::chart::Chart;
use crate::core::data::{ColumnVector, Dataset};
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::position::Position;

/// How the raw `width` values are scaled to fill the lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandScale {
    /// Scale each band by its own peak, so every band is equally wide. This is
    /// the usual violin look when the *shape* matters more than the sample
    /// size (ggplot2's default `scale = "width"`-ish). It is the default.
    PerGroup,
    /// Scale every band by the largest peak across all bands, so the areas are
    /// comparable (the "area" rule; a denser group looks wider).
    Global,
    /// Use the `width` values unchanged, in category steps. Useful when the
    /// half-width is already meaningful, for example a confidence interval.
    Raw,
}

impl From<&str> for BandScale {
    fn from(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "global" | "area" => Self::Global,
            "raw" => Self::Raw,
            _ => Self::PerGroup,
        }
    }
}

/// Which bank(s) of the river a band is drawn on.
///
/// A band is mirror-symmetric around its centre, so the natural shape is
/// [`Both`](BandSide::Both) — a violin. Keeping only one side gives a
/// one-sided shape; with one band per category on a flipped axis this is the
/// ridgeline (`with_side(BandSide::Right)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BandSide {
    /// Symmetric: `centre - width` to `centre + width`. The default.
    #[default]
    Both,
    /// One-sided, growing to the right: `centre` to `centre + width`.
    Right,
    /// One-sided, growing to the left: `centre - width` to `centre`.
    Left,
}

/// Configuration for [`Chart::transform_band`].
///
/// A band reads a `value` column (the position along the value axis) and a
/// `width` column (the half-width there). An optional `center` column gives the
/// band its position on a categorical axis, and an optional `group` column
/// turns several bands into side-by-side lanes within that centre.
#[derive(Debug, Clone)]
pub struct BandTransform {
    /// Numeric column with the position along the value axis (the density
    /// evaluation points, for a violin).
    pub(crate) value: String,

    /// Numeric column with the half-width at each value.
    pub(crate) width: String,

    /// Optional categorical column giving each band's x position (its centre).
    pub(crate) center: Option<String>,

    /// Optional categorical column splitting the centre into side-by-side lanes.
    pub(crate) group: Option<String>,

    /// Names of the produced columns: `[x, y, path_group]`.
    pub(crate) as_: [String; 3],

    /// How several lanes inside one centre are arranged.
    pub(crate) position: Position,

    /// Total width of one centre's lane group, in category steps.
    pub(crate) span: f64,

    /// The widest a single band may be, in category steps.
    pub(crate) max_width: f64,

    /// Extra width factor. `1.0` keeps a band inside its lane; larger values let
    /// it grow past the lane so neighbouring bands overlap (a ridgeline).
    pub(crate) overlap: f64,

    /// Which side(s) of the centre line a band occupies (see [`BandSide`]).
    pub(crate) side: BandSide,

    /// Width scaling rule (see [`BandScale`]).
    pub(crate) scale: BandScale,

    /// When `true`, draw two groups as the left and right halves of one band
    /// instead of as two side-by-side bands.
    pub(crate) split: bool,
}

impl BandTransform {
    /// Creates a band from a `value` and a `width` column.
    ///
    /// Defaults: one centred band, `PerGroup` scaling, no split, a span of
    /// `0.7` and a maximum width of `0.5` (the box plot and point marks).
    pub fn new(value: impl Into<String>, width: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            width: width.into(),
            center: None,
            group: None,
            as_: ["x".to_string(), "y".to_string(), "path_group".to_string()],
            position: Position::Identity,
            span: 0.7,
            max_width: 0.5,
            overlap: 1.0,
            side: BandSide::Both,
            scale: BandScale::PerGroup,
            split: false,
        }
    }

    /// Sets the categorical column that gives each band its x position.
    pub fn with_center(mut self, field: impl Into<String>) -> Self {
        self.center = Some(field.into());
        self
    }

    /// Sets the categorical column that splits a centre into lanes.
    pub fn with_group(mut self, field: impl Into<String>) -> Self {
        self.group = Some(field.into());
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

    /// Chooses how several bands inside one centre are arranged.
    pub const fn with_position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Sets the total width of a centre's lane group, in category steps.
    ///
    /// Defaults to `0.7`, matching the box plot and point marks.
    pub const fn with_span(mut self, span: f64) -> Self {
        self.span = span.clamp(0.0, 1.0);
        self
    }

    /// Sets the maximum width of a single band, in category steps.
    pub const fn with_width(mut self, width: f64) -> Self {
        self.max_width = width.clamp(0.0, 1.0);
        self
    }

    /// Grows a band past its lane, so neighbouring bands overlap.
    ///
    /// The value multiplies the band's width after scaling. `1.0` (the default)
    /// keeps every band inside its lane; values above `1.0` let a band spill
    /// into the category above it — overlapping violins, or the stacked look of
    /// a ridgeline when combined with [`with_side`](Self::with_side).
    pub const fn with_overlap(mut self, overlap: f64) -> Self {
        self.overlap = if overlap > 0.0 { overlap } else { 1.0 };
        self
    }

    /// Chooses which bank(s) of the river the band occupies.
    ///
    /// [`BandSide::Both`] (the default) is the symmetric violin;
    /// [`BandSide::Right`] / [`BandSide::Left`] keep one side, so the flat edge
    /// is the centre line. One of these on the value axis of a flipped chart is
    /// the one-sided shape a ridgeline is built from.
    pub const fn with_side(mut self, side: BandSide) -> Self {
        self.side = side;
        self
    }

    /// Chooses the width scaling rule.
    pub const fn with_scale(mut self, scale: BandScale) -> Self {
        self.scale = scale;
        self
    }

    /// Draws two groups as the left and right halves of one band.
    ///
    /// The first group grows to the right of the centre and the second to the
    /// left; colour by group to see both halves. This needs **two** groups: with
    /// a single group both banks are kept and the band stays symmetric, so use
    /// [`with_side`](Self::with_side) for a genuinely one-sided band.
    pub const fn with_split(mut self, split: bool) -> Self {
        self.split = split;
        self
    }
}

impl<T: Mark> Chart<T> {
    /// Rewrites the dataset into a band-polygon table.
    ///
    /// The statistics must have run first. The usual violin recipe is:
    ///
    /// ```rust,ignore
    /// chart!(ds)?
    ///     .transform_density(
    ///         DensityTransform::new("score")
    ///             .with_as("score", "density")
    ///             .with_groupbys(["category", "group"]),
    ///     )?
    ///     .transform_band(
    ///         BandTransform::new("score", "density")
    ///             .with_center("category")
    ///             .with_group("group")
    ///             .with_position(Position::dodge()),
    ///     )?
    ///     .mark_polygon()?
    ///     .encode((
    ///         alt::x("x").with_category_labels("category"),
    ///         alt::y("y"),
    ///         alt::path_group("path_group"),
    ///         alt::color("group"),
    ///     ))?
    ///     .save("violin.svg")?;
    /// ```
    pub fn transform_band(mut self, params: BandTransform) -> Result<Self, ChartonError> {
        let value_col = self.data.column(&params.value)?;
        let width_col = self.data.column(&params.width)?;

        // The generated polygon columns must not clash with the centre/group
        // columns that are copied back into the output table.
        let mut output_names: Vec<&str> = params.as_.iter().map(String::as_str).collect();
        if let Some(center) = &params.center {
            output_names.push(center);
        }
        if let Some(group) = &params.group {
            output_names.push(group);
        }
        ensure_distinct_columns(&output_names)?;

        // Group by `(centre, lane)` and solve the lanes once, exactly as the
        // box plot and point marks do, so a dodged band lines up with them.
        let layout = build_lane_layout(
            &self.data,
            params.center.as_deref(),
            params.group.as_deref(),
            LaneLayoutOptions {
                position: &params.position,
                span: params.span,
                max_width: params.max_width,
                split: params.split,
            },
            |i| {
                let value = value_col.get(i).to_f64()?;
                let width = width_col.get(i).to_f64()?;
                (value.is_finite() && width.is_finite()).then_some((value, width))
            },
        )?;

        let global_peak = layout
            .cells
            .iter()
            .flat_map(|cell| cell.values.iter().map(|&(_, width)| width))
            .fold(0.0_f64, f64::max);

        let mut out_x = Vec::new();
        let mut out_y = Vec::new();
        let mut out_id = Vec::new();
        let mut out_center = Vec::new();
        let mut out_group = Vec::new();

        for cell in &layout.cells {
            let center = layout.center(cell);
            let id = format!("{}::{}", cell.category_label, cell.group_label);
            let peak = cell
                .values
                .iter()
                .map(|&(_, width)| width)
                .fold(0.0_f64, f64::max);
            let factor = params.overlap
                * match params.scale {
                    BandScale::PerGroup => {
                        if peak > 0.0 {
                            layout.slot / 2.0 / peak
                        } else {
                            0.0
                        }
                    }
                    BandScale::Global => {
                        if global_peak > 0.0 {
                            layout.slot / 2.0 / global_peak
                        } else {
                            0.0
                        }
                    }
                    BandScale::Raw => 1.0,
                };

            let mut push = |x: f64, y: f64| {
                out_x.push(x);
                out_y.push(y);
                out_id.push(id.clone());
                out_center.push(cell.category_label.clone());
                out_group.push(cell.group_label.clone());
            };

            // A split band only means something when two groups share the
            // centre; a lone group has nothing to split against, so it falls
            // back to the requested side (both, by default).
            let side = if params.split && layout.lane_count() > 1 {
                if cell.group_index % 2 == 0 {
                    BandSide::Right
                } else {
                    BandSide::Left
                }
            } else {
                params.side
            };

            match side {
                // Symmetric: the right side runs up, the left side walks back
                // down, closing the polygon around the centre.
                BandSide::Both => {
                    for &(value, width) in &cell.values {
                        push(center + width * factor, value);
                    }
                    for &(value, width) in cell.values.iter().rev() {
                        push(center - width * factor, value);
                    }
                }
                // One-sided: the flat edge runs along the centre line and the
                // other side bulges out by `width`.
                BandSide::Right | BandSide::Left => {
                    let sign = if side == BandSide::Right { 1.0 } else { -1.0 };
                    let (Some(first), Some(last)) = (cell.values.first(), cell.values.last())
                    else {
                        continue;
                    };
                    push(center, first.0);
                    for &(value, width) in &cell.values {
                        push(center + sign * width * factor, value);
                    }
                    push(center, last.0);
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
        if let Some(field) = &params.center {
            new_data.add_column(
                field,
                ColumnVector::String {
                    data: out_center,
                    validity: None,
                },
            )?;
        }
        if let Some(field) = &params.group {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::Chart;
    use crate::core::data::{ColumnVector, Dataset};
    use crate::mark::no_mark::NoMark;
    use ahash::AHashMap;

    fn chart(value: Vec<f64>, width: Vec<f64>) -> Chart<NoMark> {
        let mut ds = Dataset::new();
        ds.add_column(
            "value",
            ColumnVector::Float64 {
                data: value,
                validity: None,
            },
        )
        .unwrap();
        ds.add_column(
            "width",
            ColumnVector::Float64 {
                data: width,
                validity: None,
            },
        )
        .unwrap();
        Chart::<NoMark>::build(ds).unwrap()
    }

    /// Two categories × two groups, each with a few observations.
    fn grouped_chart() -> Chart<NoMark> {
        let mut ds = Dataset::new();
        let value = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let width = vec![0.5, 1.0, 1.5, 1.0, 0.8, 1.2, 0.6, 1.4];
        let cat = ["A", "A", "B", "B", "A", "A", "B", "B"];
        let grp = ["g1", "g1", "g1", "g1", "g2", "g2", "g2", "g2"];
        ds.add_column(
            "value",
            ColumnVector::Float64 {
                data: value,
                validity: None,
            },
        )
        .unwrap();
        ds.add_column(
            "width",
            ColumnVector::Float64 {
                data: width,
                validity: None,
            },
        )
        .unwrap();
        ds.add_column(
            "cat",
            ColumnVector::String {
                data: cat.iter().map(|s| s.to_string()).collect(),
                validity: None,
            },
        )
        .unwrap();
        ds.add_column(
            "grp",
            ColumnVector::String {
                data: grp.iter().map(|s| s.to_string()).collect(),
                validity: None,
            },
        )
        .unwrap();
        Chart::<NoMark>::build(ds).unwrap()
    }

    /// The x centre of every `(category, group)` cell: the midpoint of the
    /// cell's extreme x values. Both a symmetric band and a box straddle their
    /// centre, so this recovers the lane centre the layout chose.
    fn centres(ds: &Dataset, category: &str, group: &str) -> AHashMap<(String, String), f64> {
        let xs = ds.column("x").unwrap().to_f64_vec();
        let mut bounds: AHashMap<(String, String), (f64, f64)> = AHashMap::new();
        for (i, &x) in xs.iter().enumerate() {
            let key = (
                ds.get(category, i).to_string().unwrap_or_default(),
                ds.get(group, i).to_string().unwrap_or_default(),
            );
            let entry = bounds
                .entry(key)
                .or_insert((f64::INFINITY, f64::NEG_INFINITY));
            entry.0 = entry.0.min(x);
            entry.1 = entry.1.max(x);
        }
        bounds
            .into_iter()
            .map(|(key, (lo, hi))| (key, (lo + hi) / 2.0))
            .collect()
    }

    #[test]
    fn band_is_symmetric_around_its_centre() {
        let banded = chart(vec![0.0, 1.0, 2.0], vec![1.0, 2.0, 1.0])
            .transform_band(BandTransform::new("value", "width"))
            .unwrap();

        let xs = banded.data.column("x").unwrap().to_f64_vec();
        let ys = banded.data.column("y").unwrap().to_f64_vec();
        assert_eq!(xs.len(), 6);
        assert_eq!(ys.len(), 6);

        // The right side runs up, the left side mirrors it back down.
        for i in 0..xs.len() / 2 {
            let mirror = xs.len() - 1 - i;
            assert!((xs[i] + xs[mirror]).abs() < 1e-12, "x[{i}] + x[{mirror}]");
            assert!((ys[i] - ys[mirror]).abs() < 1e-12, "y[{i}] != y[{mirror}]");
        }
    }

    #[test]
    fn peak_width_fills_half_the_lane() {
        // Default max_width is 0.5, so the half-lane is 0.25 and the largest
        // width on the band maps exactly there.
        let banded = chart(vec![0.0, 1.0, 2.0], vec![1.0, 2.0, 1.0])
            .transform_band(BandTransform::new("value", "width"))
            .unwrap();

        let xs = banded.data.column("x").unwrap().to_f64_vec();
        let max = xs.iter().copied().fold(f64::MIN, f64::max);
        assert!((max - 0.25).abs() < 1e-12, "max half-width {max}");
    }

    #[test]
    fn overlap_grows_the_band_past_its_lane() {
        // The default half-lane is 0.25; an overlap of 2.0 doubles it to 0.5,
        // which is what lets a ridgeline spill into the category above.
        let banded = chart(vec![0.0, 1.0, 2.0], vec![1.0, 2.0, 1.0])
            .transform_band(BandTransform::new("value", "width").with_overlap(2.0))
            .unwrap();

        let xs = banded.data.column("x").unwrap().to_f64_vec();
        let max = xs.iter().copied().fold(f64::MIN, f64::max);
        assert!((max - 0.5).abs() < 1e-12, "max half-width {max}");
    }

    #[test]
    fn side_right_keeps_only_one_bank() {
        // A one-sided band keeps the centre line flat and bulges to the right,
        // so every vertex is at or past the centre.
        let banded = chart(vec![0.0, 1.0, 2.0], vec![1.0, 2.0, 1.0])
            .transform_band(BandTransform::new("value", "width").with_side(BandSide::Right))
            .unwrap();

        let xs = banded.data.column("x").unwrap().to_f64_vec();
        assert!(
            xs.iter().all(|&x| x >= 0.0),
            "right-side x went negative: {xs:?}"
        );
        let max = xs.iter().copied().fold(f64::MIN, f64::max);
        assert!((max - 0.25).abs() < 1e-12, "max half-width {max}");
    }

    #[test]
    fn split_without_a_group_stays_symmetric() {
        // `split` needs two groups to split against; a lone group must not
        // silently become one-sided — that is what `with_side` is for.
        let banded = chart(vec![0.0, 1.0, 2.0], vec![1.0, 2.0, 1.0])
            .transform_band(BandTransform::new("value", "width").with_split(true))
            .unwrap();

        let xs = banded.data.column("x").unwrap().to_f64_vec();
        let min = xs.iter().copied().fold(f64::MAX, f64::min);
        let max = xs.iter().copied().fold(f64::MIN, f64::max);
        assert!((min + 0.25).abs() < 1e-12, "min half-width {min}");
        assert!((max - 0.25).abs() < 1e-12, "max half-width {max}");
    }

    #[test]
    fn raw_scale_uses_the_width_column_verbatim() {
        let banded = chart(vec![0.0, 1.0], vec![0.1, 0.2])
            .transform_band(BandTransform::new("value", "width").with_scale(BandScale::Raw))
            .unwrap();
        let xs = banded.data.column("x").unwrap().to_f64_vec();
        // Right side up then left side down, with no lane scaling applied.
        assert!((xs[0] - 0.1).abs() < 1e-12, "{}", xs[0]);
        assert!((xs[1] - 0.2).abs() < 1e-12, "{}", xs[1]);
        assert!((xs[2] + 0.2).abs() < 1e-12, "{}", xs[2]);
        assert!((xs[3] + 0.1).abs() < 1e-12, "{}", xs[3]);
    }

    /// The box geometry must reuse the *same* lane solve as the band, otherwise
    /// a raincloud's box would float off its violin.
    #[test]
    fn band_and_box_agree_on_every_lane_centre() {
        let banded = grouped_chart()
            .transform_band(
                BandTransform::new("value", "width")
                    .with_center("cat")
                    .with_group("grp")
                    .with_position(Position::dodge()),
            )
            .unwrap();
        let boxed = grouped_chart()
            .transform_quantile_box(
                crate::transform::box_transform::QuantileBoxTransform::new("value")
                    .with_category("cat")
                    .with_group("grp")
                    .with_position(Position::dodge()),
            )
            .unwrap();

        let band_centres = centres(&banded.data, "cat", "grp");
        let box_centres = centres(&boxed.data, "cat", "grp");
        assert_eq!(band_centres.len(), 4);
        assert_eq!(band_centres.len(), box_centres.len());
        for (key, band_centre) in &band_centres {
            let box_centre = box_centres.get(key).expect("box cell missing");
            assert!(
                (band_centre - box_centre).abs() < 1e-12,
                "{key:?}: band centre {band_centre} vs box centre {box_centre}"
            );
        }
    }

    /// A null *category* (position) is dropped, while a null *group*
    /// (non-positional lane) becomes the reserved `"NA"` lane. This is the
    /// Plan-A split: position drops, non-position keeps.
    #[test]
    fn null_category_drops_but_null_group_becomes_missing_lane() {
        let mut ds = Dataset::new();
        ds.add_column(
            "value",
            ColumnVector::Float64 {
                data: vec![1.0, 2.0, 3.0, 4.0],
                validity: None,
            },
        )
        .unwrap();
        // cat: ["A", null, "A", null]
        ds.add_column(
            "cat",
            ColumnVector::String {
                data: vec!["A".into(), "ignored".into(), "A".into(), "ignored".into()],
                validity: Some(vec![0b0101]),
            },
        )
        .unwrap();
        // grp: [null, "g", null, "g"]
        ds.add_column(
            "grp",
            ColumnVector::String {
                data: vec!["ignored".into(), "g".into(), "ignored".into(), "g".into()],
                validity: Some(vec![0b1010]),
            },
        )
        .unwrap();

        let layout = build_lane_layout(
            &ds,
            Some("cat"),
            Some("grp"),
            LaneLayoutOptions {
                position: &Position::Identity,
                span: 0.7,
                max_width: 0.5,
                split: false,
            },
            |i| ds.column("value").unwrap().get(i).to_f64(),
        )
        .unwrap();

        // Rows (A, null), (null, g), (A, null), (null, g): the two position-null
        // rows are dropped, the two group-null rows share one (A, "NA") cell.
        assert_eq!(layout.cells.len(), 1, "unexpected cells");
        let cell = &layout.cells[0];
        assert_eq!(cell.category_label, "A");
        assert_eq!(cell.group_label, "NA");
        assert_eq!(cell.values, vec![1.0, 3.0]);
    }

    /// A category column that already carries a generated output name must be
    /// rejected, not silently overwritten.
    #[test]
    fn rejects_a_generated_name_clash() {
        let mut ds = Dataset::new();
        ds.add_column(
            "value",
            ColumnVector::Float64 {
                data: vec![1.0, 2.0, 3.0],
                validity: None,
            },
        )
        .unwrap();
        ds.add_column(
            "width",
            ColumnVector::Float64 {
                data: vec![1.0, 1.0, 1.0],
                validity: None,
            },
        )
        .unwrap();
        // A category column literally called `x`, which the band also emits.
        ds.add_column(
            "x",
            ColumnVector::String {
                data: vec!["a".into(), "a".into(), "a".into()],
                validity: None,
            },
        )
        .unwrap();
        let chart = Chart::<NoMark>::build(ds).unwrap();

        let result = chart.transform_band(BandTransform::new("value", "width").with_center("x"));
        assert!(result.is_err(), "a generated-name clash must be rejected");
    }
}
