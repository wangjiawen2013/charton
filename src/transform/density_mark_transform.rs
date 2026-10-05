//! The density mark's recipe (internal).
//!
//! Runs `transform_density` and the area preparation for `mark_density`. The
//! input `x`/`color` columns are cached on the mark on the first run so the
//! recipe can be re-run per facet panel.

use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::transform::density_transform::DensityTransform;

impl<T: Mark> Chart<T> {
    /// Expands the density mark into the density transform + area geometry.
    pub(crate) fn transform_density_data(mut self) -> Result<Self, ChartonError> {
        let missing = || ChartonError::Mark("density mark configuration is missing".to_string());

        // --- Step 1: Resolve the input columns exactly once -----------------
        let resolved = self
            .mark
            .as_ref()
            .and_then(Mark::as_density)
            .map(|d| d.inputs_resolved)
            .ok_or_else(missing)?;

        if !resolved {
            let value = self
                .encoding
                .x
                .as_ref()
                .map(|e| e.field.clone())
                .ok_or_else(|| {
                    ChartonError::Encoding(
                        "density requires an x encoding holding the value column".to_string(),
                    )
                })?;
            let group = self.encoding.color.as_ref().map(|e| e.field.clone());

            let d = self
                .mark
                .as_mut()
                .and_then(Mark::as_density_mut)
                .ok_or_else(missing)?;
            d.value_field = Some(value);
            d.group_field = group;
            d.inputs_resolved = true;
        }

        // --- Step 2: Read back the resolved parameters ----------------------
        let (value, group, bandwidth, kernel, trim, counts, cumulative) = {
            let d = self
                .mark
                .as_ref()
                .and_then(Mark::as_density)
                .ok_or_else(missing)?;
            (
                d.value_field.clone().ok_or_else(missing)?,
                d.group_field.clone(),
                d.bandwidth,
                d.kernel,
                d.trim,
                d.counts,
                d.cumulative,
            )
        };

        // --- Step 3: The statistic (public, reused) -------------------------
        // The curve is written to a private column, so a user column named
        // `density` cannot collide with it. The `x` encoding still names the
        // value column and does not need rewriting.
        let density_out = format!("{TEMP_SUFFIX}_density");
        let mut density = DensityTransform::new(value.clone())
            .with_as(value, density_out.clone())
            .with_bandwidth(bandwidth)
            .with_kernel(kernel)
            .with_trim(trim)
            .with_counts(counts)
            .with_cumulative(cumulative);
        if let Some(g) = &group {
            density = density.with_groupbys([g.clone()]);
        }
        self = self.transform_density(density)?;

        // The estimated curve is the y axis.
        self.encoding.y = Some(crate::encode::y::y(&density_out).with_label("density"));

        // --- Step 4: The geometry (the same prep `mark_area` runs) ----------
        self = self.transform_area_data()?;

        Ok(self)
    }
}
