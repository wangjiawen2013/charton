use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, RenderBackend, TextConfig};
use crate::error::ChartonError;
use crate::mark::text::MarkText;
use crate::visual::color::SingleColor;

// ============================================================================
// MARK RENDERING (Text implementation)
// ============================================================================

impl MarkRenderer for Chart<MarkText> {
    /// Renders text marks by mapping data points to text elements in the backend.
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let ds = &self.data;

        // Early return if no data rows exist.
        if ds.row_count == 0 {
            return Ok(());
        }

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkText configuration is missing".to_string()))?;

        // --- STEP 1: PRE-NORMALIZE POSITION COLUMNS ---
        // Access encodings for X and Y axes.
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("X is missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or_else(|| ChartonError::Encoding("Y is missing".into()))?;

        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        // Perform vectorized normalization directly on our internal ColumnVectors.
        let x_norms = x_scale
            .scale_type()
            .normalize_column(x_scale, ds.column(&x_enc.field)?);
        let y_norms = y_scale
            .scale_type()
            .normalize_column(y_scale, ds.column(&y_enc.field)?);

        // --- STEP 2: RESOLVE TEXT CONTENT ---
        // If a text encoding exists, use the column values; otherwise, repeat the static mark text.
        let text_values: Vec<String> = if let Some(ref text_enc) = self.encoding.text {
            let col = ds.column(&text_enc.field)?;
            (0..ds.row_count)
                .map(|i| col.get_as_string(i).unwrap_or_default())
                .collect()
        } else {
            vec![mark_config.text.clone(); ds.row_count]
        };

        // --- STEP 3: RESOLVE COLORS ---
        // Pre-normalize the color column if a mapping exists.
        let color_norms = if let Some(ref mapping) = context.spec.aesthetics.color {
            let s_trait = mapping.scale_impl.as_ref();
            Some(
                s_trait
                    .scale_type()
                    .normalize_column(s_trait, ds.column(&mapping.field)?),
            )
        } else {
            None
        };

        // --- STEP 4: EMIT TEXT ELEMENTS ---
        for i in 0..ds.row_count {
            // Skip points where X or Y are null.
            let (Some(xn), Some(yn)) = (x_norms[i], y_norms[i]) else {
                continue;
            };

            // 4.1 Coordinate Projection: [0, 1] -> Pixels
            let (px, py) = context.coord.transform(xn, yn, &context.panel);

            // 4.2 Color Resolution
            let fill_color = if let Some(ref norms) = color_norms {
                self.resolve_color_from_value(norms[i], context, &mark_config.color)
            } else {
                mark_config.color
            };

            // 4.3 Drawing
            backend.draw_text(TextConfig {
                x: px as Precision,
                y: py as Precision,
                text: text_values[i].clone(),
                font_size: mark_config.font_size as Precision,
                font_family: mark_config.font_family.clone(),
                color: fill_color,
                text_anchor: mark_config.text_anchor.to_string(),
                font_weight: mark_config.font_weight.to_string(),
                opacity: mark_config.opacity as Precision,
            });
        }

        Ok(())
    }
}

impl Chart<MarkText> {
    /// Re-using the optimized color resolution logic.
    fn resolve_color_from_value(
        &self,
        val: Option<f64>,
        context: &PanelContext,
        fallback: &SingleColor,
    ) -> SingleColor {
        if let (Some(v), Some(mapping)) = (val, &context.spec.aesthetics.color) {
            let s_trait = mapping.scale_impl.as_ref();
            s_trait
                .mapper()
                .as_ref()
                .map(|m| m.map_to_color(v, s_trait.logical_max()))
                .unwrap_or(*fallback)
        } else {
            *fallback
        }
    }
}
