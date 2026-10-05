//! The violin mark's recipe (internal).
//!
//! Runs `transform_density` and `transform_band` for `mark_violin`, then points
//! the encoding at the columns they produce.
//!
//! The input column names are cached on
//! [`MarkViolin`](crate::mark::violin::MarkViolin) on the first run, because a
//! faceted re-run reaches this code after the encoding already names the
//! generated `x`/`y`/`path_group` columns.

use crate::TEMP_SUFFIX;
use crate::chart::Chart;
use crate::error::ChartonError;
use crate::mark::Mark;
use crate::position::Position;
use crate::transform::band_transform::BandTransform;
use crate::transform::density_transform::DensityTransform;

impl<T: Mark> Chart<T> {
    /// Runs the density estimate and band geometry, and rewrites the encoding
    /// to the generated columns.
    pub(crate) fn transform_violin_data(mut self) -> Result<Self, ChartonError> {
        let missing = || ChartonError::Mark("violin mark configuration is missing".to_string());

        // --- Step 1: Resolve the input columns exactly once -----------------
        // On a faceted re-run `inputs_resolved` is already `true`, so the
        // cached names are reused verbatim.
        let resolved = self
            .mark
            .as_ref()
            .and_then(Mark::as_violin)
            .map(|v| v.inputs_resolved)
            .ok_or_else(missing)?;

        if !resolved {
            // On the first pass `y` is the value column. `x` (category) and
            // `color` (group) are optional.
            let value = self
                .encoding
                .y
                .as_ref()
                .map(|e| e.field.clone())
                .ok_or_else(|| {
                    ChartonError::Encoding(
                        "violin requires a y encoding holding the value column".to_string(),
                    )
                })?;
            let center = self.encoding.x.as_ref().map(|e| e.field.clone());
            let group = self.encoding.color.as_ref().map(|e| e.field.clone());

            let v = self
                .mark
                .as_mut()
                .and_then(Mark::as_violin_mut)
                .ok_or_else(missing)?;
            v.value_field = Some(value);
            v.center_field = center;
            v.group_field = group;
            v.inputs_resolved = true;
        }

        // --- Step 2: Read back the resolved parameters ----------------------
        let (value, center, group, bandwidth, trim, split, side, band_scale, width, span, overlap) = {
            let v = self
                .mark
                .as_ref()
                .and_then(Mark::as_violin)
                .ok_or_else(missing)?;
            (
                v.value_field.clone().ok_or_else(missing)?,
                v.center_field.clone(),
                v.group_field.clone(),
                v.bandwidth,
                v.trim,
                v.split,
                v.side,
                v.scale,
                v.width,
                v.span,
                v.overlap,
            )
        };

        // Generated columns are prefixed so they can never collide with the
        // user's own columns; the axes are titled from the original names with
        // `with_label` below.
        let value_out = format!("{TEMP_SUFFIX}_violin_value");
        let density_out = format!("{TEMP_SUFFIX}_violin_density");
        let x_out = format!("{TEMP_SUFFIX}_violin_x");
        let y_out = format!("{TEMP_SUFFIX}_violin_y");
        let pg_out = format!("{TEMP_SUFFIX}_violin_path_group");

        // One density curve per (category, group) cell. Either may be absent.
        let mut groupbys: Vec<String> = Vec::new();
        if let Some(c) = &center {
            groupbys.push(c.clone());
        }
        if let Some(g) = &group {
            groupbys.push(g.clone());
        }

        // --- Step 3: The statistic (public, reused) -------------------------
        let density = DensityTransform::new(value.clone())
            .with_as(value_out.clone(), density_out.clone())
            .with_groupbys(groupbys)
            .with_bandwidth(bandwidth)
            .with_trim(trim);
        self = self.transform_density(density)?;

        // --- Step 4: The geometry (public, reused) --------------------------
        let mut band = BandTransform::new(value_out, density_out)
            .with_as(x_out.clone(), y_out.clone(), pg_out.clone())
            .with_scale(band_scale)
            .with_width(width)
            .with_span(span)
            .with_overlap(overlap)
            .with_side(side)
            .with_split(split);
        if let Some(c) = &center {
            band = band.with_center(c.clone());
        }
        if let Some(g) = &group {
            band = band.with_group(g.clone());
        }
        // Several groups inside one category sit side by side, unless they are
        // the two halves of a split violin.
        if group.is_some() && !split {
            band = band.with_position(Position::dodge());
        }
        self = self.transform_band(band)?;

        // --- Step 5: Point the encoding at the generated columns ------------
        // The display labels keep the original column names on the axes even
        // though the encoded fields are now the generated `x`/`y`.
        let mut x_enc = crate::encode::x::x(&x_out);
        match &center {
            Some(c) => {
                // Numeric band positions with categorical labels form a discrete
                // position axis.
                x_enc = x_enc.with_category_labels(c.clone()).with_label(c.clone());
            }
            // A single violin has no category; the axis is the width direction.
            None => x_enc = x_enc.with_label("x"),
        }
        self.encoding.x = Some(x_enc);
        self.encoding.y = Some(crate::encode::y::y(&y_out).with_label(value.clone()));
        self.encoding.path_group = Some(crate::encode::path_group::path_group(&pg_out));
        // `color` is left untouched: the band copies the group column back, so
        // the existing colour encoding keeps pointing at a valid column.

        Ok(self)
    }
}
