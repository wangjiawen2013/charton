//! Transforms: the data-processing steps that run before a mark is drawn.
//!
//! A transform reads the current `Dataset`, computes something new from it and
//! replaces it. Because every transform has the same shape, they chain freely:
//! the output of one is the input of the next, and the final table is what the
//! mark encodes.
//!
//! They come in two families, and keeping them apart is the whole trick behind
//! charton's composition model:
//!
//! * **Statistics** summarise the rows: `transform_density` estimates a smooth
//!   curve, `transform_density_2d` a density grid, `transform_contour`
//!   iso-lines of any grid, and `transform_quantile_box` the quartiles of each
//!   group. The per-mark statistics (histogram bins, box plot quartiles, error
//!   bars) live with their marks.
//! * **Geometry** turns a summary into the columns a mark draws:
//!   `transform_band` is the general symmetric `centre ± width` polygon, and
//!   `transform_quantile_box` emits the box and median polygons alongside its
//!   quartiles.
//!
//! A violin, for instance, is `transform_density` (statistics) followed by
//! `transform_band` (geometry) and then `mark_polygon`. No step knows the word
//! "violin"; each only does its own job. See
//! `docs/src/concepts/grammar_pipeline.md` for the full stat → position →
//! geometry model.

pub(crate) mod area_transform;
pub(crate) mod band_transform;
pub(crate) mod bar_transform;
pub(crate) mod box_transform;
pub(crate) mod boxplot_transform;
pub(crate) mod calculate_transform;
pub(crate) mod contour_mark_transform;
pub(crate) mod contour_transform;
pub(crate) mod density_2d_mark_transform;
pub(crate) mod density_2d_transform;
pub(crate) mod density_mark_transform;
pub(crate) mod density_transform;
pub(crate) mod errorbar_transform;
pub(crate) mod hist_transform;
pub(crate) mod lane_layout;
pub(crate) mod point_transform;
pub(crate) mod rect_transform;
pub(crate) mod violin_transform;
pub(crate) mod window_transform;

use crate::error::ChartonError;
use ahash::AHashSet;

/// Fails if a transform would write the same output column name more than once.
///
/// Column names are how transforms talk to the encoding, and `add_column`
/// overwrites on a clash. A transform that emits the same name twice would
/// therefore drop one of the two columns with no warning at all — most easily
/// when a caller's category or group column already happens to be named `x`,
/// `y` or `path_group`. There is never a legitimate reason to overwrite one of
/// a transform's own outputs, so this turns that mistake into a clear error
/// instead of a quietly wrong chart.
///
/// The composable transforms (`transform_density`, `transform_density_2d`,
/// `transform_contour`, `transform_band`, `transform_quantile_box`) collect
/// their full output-name list and check it here before touching the dataset.
pub(crate) fn ensure_distinct_columns(names: &[&str]) -> Result<(), ChartonError> {
    let mut seen: AHashSet<&str> = AHashSet::new();
    for name in names {
        if !seen.insert(name) {
            return Err(ChartonError::Data(format!(
                "transform writes the column '{name}' more than once; rename one of its \
                 outputs with `with_as(...)`, or choose a different input column"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ensure_distinct_columns;

    #[test]
    fn distinct_output_names_are_accepted() {
        assert!(ensure_distinct_columns(&["x", "y", "path_group"]).is_ok());
    }

    #[test]
    fn a_repeated_output_name_is_rejected() {
        assert!(ensure_distinct_columns(&["x", "y", "x"]).is_err());
    }
}
