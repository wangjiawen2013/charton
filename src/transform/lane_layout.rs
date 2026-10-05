//! Shared categorical lane layout for the composable band and box geometry.
//!
//! `transform_band` and `transform_quantile_box` both group rows by an optional
//! `(category, group)` pair and place one lane per group inside each category.
//! This module owns that machinery — the first-appearance ordering, the cell
//! lookup and the [`Position`] lane solve — so the two transforms cannot drift
//! apart. Callers only contribute what is genuinely different: how to pull a
//! numeric payload out of a row, and what shape to draw from the result.
//!
//! # Missing values
//!
//! The `category` is a *position*, so a row whose category is missing is
//! dropped — there is no lane to put it in. The `group` only selects a lane
//! inside an otherwise valid category, so a missing group is kept under the
//! reserved [`MISSING_CATEGORY`] label and drawn as one extra lane. Because the
//! band and the box share this function, a violin outline and its inner box
//! always agree on which rows exist and where they sit, holes included.
//!
//! [`MISSING_CATEGORY`]: crate::core::data::MISSING_CATEGORY

use crate::core::data::{Dataset, MISSING_CATEGORY};
use crate::error::ChartonError;
use crate::position::Position;
use ahash::AHashMap;

/// One non-empty `(category, group)` cell, in category-major emission order.
pub(crate) struct LaneCell<T> {
    pub category_index: usize,
    pub group_index: usize,
    pub category_label: String,
    pub group_label: String,
    /// The per-row payload collected from the cell, in row order.
    pub values: Vec<T>,
}

/// A resolved categorical axis: every cell, plus the lane geometry they share.
pub(crate) struct LaneLayout<T> {
    pub cells: Vec<LaneCell<T>>,
    /// Width one lane may use, in category steps.
    pub slot: f64,
    /// Centre shift of each lane, in category steps.
    lane_offsets: Vec<f64>,
}

impl<T> LaneLayout<T> {
    /// The x centre of a cell: its category index plus its lane offset.
    pub fn center(&self, cell: &LaneCell<T>) -> f64 {
        cell.category_index as f64 + self.lane_offsets[cell.group_index]
    }

    /// How many lanes a centre may have (the number of distinct groups).
    ///
    /// `1` means there is nothing to sit side by side or to split, so a
    /// `with_split` request has no effect.
    pub fn lane_count(&self) -> usize {
        self.lane_offsets.len()
    }
}

/// The lane-placement options shared by the band and box geometries.
#[derive(Clone, Copy)]
pub(crate) struct LaneLayoutOptions<'a> {
    /// How several lanes inside one centre are arranged (`Identity` / `Dodge`).
    pub position: &'a Position,
    /// Total width of one centre's lane group, in category steps.
    pub span: f64,
    /// The widest a single lane may be, in category steps.
    pub max_width: f64,
    /// Share the centre line (a split violin) instead of sitting side by side.
    pub split: bool,
}

/// Groups `data` by `(category, group)` and solves the lane layout.
///
/// `extract` returns the payload for one row, or `None` to skip it (a null or
/// non-finite value). When `category`/`group` are `None`, every row lands in a
/// single unnamed cell.
///
/// With `split`, the lanes share the centre line (a split violin); otherwise
/// `position` distributes them across `span`, each at most `max_width` wide.
pub(crate) fn build_lane_layout<T, F>(
    data: &Dataset,
    category: Option<&str>,
    group: Option<&str>,
    options: LaneLayoutOptions<'_>,
    mut extract: F,
) -> Result<LaneLayout<T>, ChartonError>
where
    F: FnMut(usize) -> Option<T>,
{
    let LaneLayoutOptions {
        position,
        span,
        max_width,
        split,
    } = options;
    // The order the categories and lanes first appear keeps colours and facets
    // stable from run to run. Categories use the positional list (no missing
    // level); groups use the non-positional list, which hides any missing group
    // behind the reserved label placed last.
    let category_order: Vec<Option<String>> = match category {
        Some(field) => data
            .column(field)?
            .unique_values()
            .into_iter()
            .map(Some)
            .collect(),
        None => vec![None],
    };
    let group_order: Vec<Option<String>> = match group {
        Some(field) => data
            .column(field)?
            .labels_with_missing()
            .into_iter()
            .map(Some)
            .collect(),
        None => vec![None],
    };

    let category_index: AHashMap<Option<String>, usize> = category_order
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, label)| (label, i))
        .collect();
    let group_index: AHashMap<Option<String>, usize> = group_order
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, label)| (label, i))
        .collect();

    // Gather the payload per `(category, lane)` cell, skipping rows that carry
    // no usable value.
    let mut cells: AHashMap<(usize, usize), Vec<T>> = AHashMap::new();
    let mut present: Vec<(usize, usize)> = Vec::new();
    for i in 0..data.height() {
        let Some(value) = extract(i) else {
            continue;
        };
        // `category` is positional: a missing value drops the row. `group` is
        // non-positional (a lane/colour): a missing value becomes the reserved
        // "NA" lane, so the observation is kept.
        let category_label = match category {
            Some(field) => {
                let label = data
                    .get(field, i)
                    .to_string()
                    .unwrap_or_else(|| MISSING_CATEGORY.to_string());
                if label == MISSING_CATEGORY {
                    continue;
                }
                Some(label)
            }
            None => None,
        };
        let group_label = group.map(|field| {
            data.get(field, i)
                .to_string()
                .unwrap_or_else(|| MISSING_CATEGORY.to_string())
        });
        let (Some(&ci), Some(&gi)) = (
            category_index.get(&category_label),
            group_index.get(&group_label),
        ) else {
            continue;
        };
        let held = cells.entry((ci, gi)).or_default();
        if held.is_empty() {
            present.push((ci, gi));
        }
        held.push(value);
    }

    // A stable, category-major emission order.
    present.sort_unstable();

    let cells = present
        .into_iter()
        .filter_map(|(ci, gi)| {
            let values = cells.remove(&(ci, gi))?;
            if values.is_empty() {
                return None;
            }
            Some(LaneCell {
                category_index: ci,
                group_index: gi,
                category_label: category_order[ci]
                    .clone()
                    .unwrap_or_else(|| "all".to_string()),
                group_label: group_order[gi].clone().unwrap_or_else(|| "all".to_string()),
                values,
            })
        })
        .collect();

    // Solve the lanes exactly as the box plot and point marks do, so a dodged
    // band/box has the same width and spacing as a dodged box or scatter.
    let lane_count = group_order.len().max(1);
    let (lane_offsets, slot) = if split {
        (vec![0.0; lane_count], max_width)
    } else {
        let item_width = position.item_width(lane_count as f64, span, max_width);
        let offsets = (0..lane_count)
            .map(|i| position.offset(i as f64, lane_count as f64, item_width))
            .collect();
        (offsets, item_width)
    };

    Ok(LaneLayout {
        cells,
        slot,
        lane_offsets,
    })
}
