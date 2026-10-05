//! The 2-D density mark's recipe (internal).
//!
//! Runs `transform_density_2d` and the rectangle preparation for
//! `mark_density_2d`. The rect bin count is set to the grid size so each cell
//! holds exactly one grid node.

use crate::chart::Chart;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::scale::Scale;
use crate::transform::density_2d_transform::Density2DTransform;

impl<T: Mark> Chart<T> {
    /// Expands the 2-D density mark into density + rectangle geometry.
    pub(crate) fn transform_density_2d_data(mut self) -> Result<Self, ChartonError> {
        let missing = || ChartonError::Mark("density_2d mark configuration is missing".to_string());

        // --- Step 1: Resolve the input columns exactly once -----------------
        let resolved = self
            .mark
            .as_ref()
            .and_then(Mark::as_density_2d)
            .map(|d| d.inputs_resolved)
            .ok_or_else(missing)?;

        if !resolved {
            let x = self
                .encoding
                .x
                .as_ref()
                .map(|e| e.field.clone())
                .ok_or_else(|| {
                    ChartonError::Encoding(
                        "density_2d requires an x encoding (the first axis)".to_string(),
                    )
                })?;
            let y = self
                .encoding
                .y
                .as_ref()
                .map(|e| e.field.clone())
                .ok_or_else(|| {
                    ChartonError::Encoding(
                        "density_2d requires a y encoding (the second axis)".to_string(),
                    )
                })?;

            let d = self
                .mark
                .as_mut()
                .and_then(Mark::as_density_2d_mut)
                .ok_or_else(missing)?;
            d.x_field = Some(x);
            d.y_field = Some(y);
            d.inputs_resolved = true;
        }

        // --- Step 2: Read back the resolved parameters ----------------------
        let (x, y, bandwidth, grid, padding) = {
            let d = self
                .mark
                .as_ref()
                .and_then(Mark::as_density_2d)
                .ok_or_else(missing)?;
            (
                d.x_field.clone().ok_or_else(missing)?,
                d.y_field.clone().ok_or_else(missing)?,
                d.bandwidth,
                d.grid,
                d.padding,
            )
        };

        // --- Step 3: The statistic (public, reused) -------------------------
        // The grid columns are named `x`, `y` and `density` (the transform's
        // defaults).
        let density = Density2DTransform::new(x.clone(), y.clone())
            .with_grid_size(grid)
            .with_padding(padding)
            .with_bandwidth(bandwidth);
        self = self.transform_density_2d(density)?;

        // --- Step 4: The geometry (prepare heatmap cells) -------------------
        // Point the encoding at the grid columns and set the rect bin count to
        // the grid size, so every cell holds exactly one node and nothing is
        // merged. Scale types are known here (the grid is always numeric), so
        // set them directly rather than re-running semantic resolution.
        self.encoding.x = Some(
            crate::encode::x::x("x")
                .with_bins(grid)
                .with_scale(Scale::Linear)
                .with_label(x),
        );
        self.encoding.y = Some(
            crate::encode::y::y("y")
                .with_bins(grid)
                .with_scale(Scale::Linear)
                .with_label(y),
        );
        self.encoding.color = Some(crate::encode::color::color("density").with_label("density"));

        self = self.transform_rect_data()?;

        Ok(self)
    }
}
