//! The contour mark's recipe (internal).
//!
//! Runs `transform_contour` for `mark_contour` and points the encoding at the
//! generated columns. `x`/`y` are cached on
//! [`MarkContour`](crate::mark::contour::MarkContour) on the first run so a
//! faceted re-run can find the original columns.

use crate::chart::Chart;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::transform::contour_transform::{ContourLevels, ContourTransform};

impl<T: Mark> Chart<T> {
    /// Expands the contour mark into the iso-line transform + path geometry.
    pub(crate) fn transform_contour_data(mut self) -> Result<Self, ChartonError> {
        let missing = || ChartonError::Mark("contour mark configuration is missing".to_string());

        // --- Step 1: Resolve the input columns exactly once -----------------
        let resolved = self
            .mark
            .as_ref()
            .and_then(Mark::as_contour)
            .map(|c| c.inputs_resolved)
            .ok_or_else(missing)?;

        if !resolved {
            let x = self
                .encoding
                .x
                .as_ref()
                .map(|e| e.field.clone())
                .ok_or_else(|| {
                    ChartonError::Encoding(
                        "contour requires an x encoding (the grid's first axis)".to_string(),
                    )
                })?;
            let y = self
                .encoding
                .y
                .as_ref()
                .map(|e| e.field.clone())
                .ok_or_else(|| {
                    ChartonError::Encoding(
                        "contour requires a y encoding (the grid's second axis)".to_string(),
                    )
                })?;

            let c = self
                .mark
                .as_mut()
                .and_then(Mark::as_contour_mut)
                .ok_or_else(missing)?;
            c.x_field = Some(x);
            c.y_field = Some(y);
            c.inputs_resolved = true;
        }

        // --- Step 2: Read back the resolved parameters ----------------------
        let (x, y, z, levels) = {
            let c = self
                .mark
                .as_ref()
                .and_then(Mark::as_contour)
                .ok_or_else(missing)?;
            (
                c.x_field.clone().ok_or_else(missing)?,
                c.y_field.clone().ok_or_else(missing)?,
                c.z_field.clone().ok_or_else(missing)?,
                c.levels.clone(),
            )
        };

        // --- Step 3: The statistic (public, reused) -------------------------
        // Use the transform's default output names (`x`, `y`, `path_group`,
        // `level`).
        let contour = match &levels {
            ContourLevels::Count(n) => {
                ContourTransform::new(x.clone(), y.clone(), z.clone()).with_levels(*n)
            }
            ContourLevels::Values(values) => ContourTransform::new(x.clone(), y.clone(), z.clone())
                .with_levels_values(values.clone()),
        };
        self = self.transform_contour(contour)?;

        // --- Step 4: Point the encoding at the generated columns ------------
        // The display labels keep the original grid column names on the axes.
        self.encoding.x = Some(crate::encode::x::x("x").with_label(x));
        self.encoding.y = Some(crate::encode::y::y("y").with_label(y));
        self.encoding.path_group = Some(crate::encode::path_group::path_group("path_group"));

        // Colour by level by default. An explicit colour encoding wins, so a
        // caller can map the lines to anything (including nothing).
        let color_by_level = self
            .mark
            .as_ref()
            .and_then(Mark::as_contour)
            .map(|c| c.color_by_level)
            .unwrap_or(false);
        if color_by_level && self.encoding.color.is_none() {
            // The colour represents the scalar field, so title the legend with
            // the original `z` column rather than the generated `level`.
            self.encoding.color = Some(crate::encode::color::color("level").with_label(z));
        }

        Ok(self)
    }
}
