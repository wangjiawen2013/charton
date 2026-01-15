use crate::visual::shape::PointShape;
use crate::theme::Theme;
use crate::core::layer::RenderBackend;
use super::backend::svg::SvgBackend;
use crate::core::legend::{LegendSpec, LegendPosition};
use crate::core::context::SharedRenderingContext;
use crate::scale::ScaleDomain;
use std::f64::consts::PI;

/// LegendRenderer is responsible for drawing visual guides that explain the scales used in the plot.
/// 
/// Following the Grammar of Graphics, legends map visual aesthetics (colors, shapes) back to 
/// data values. This renderer handles dynamic positioning, centering for horizontal layouts, 
/// and ensures typography is consistent with the global Theme.
pub struct LegendRenderer;

impl LegendRenderer {
    /// The main entry point for the legend rendering process.
    /// 
    /// It coordinates the calculation of the drawing start point (anchor),
    /// the resolution of data-to-visual mappings, and the final SVG output.
    pub fn render_legend(
        buffer: &mut String,
        specs: &[LegendSpec],
        theme: &Theme,
        ctx: &SharedRenderingContext,
    ) {
        // Do not render anything if the legend is explicitly disabled or no specs exist.
        if specs.is_empty() || matches!(ctx.legend_position, LegendPosition::None) {
            return;
        }

        let mut backend = SvgBackend::new(buffer);
        
        // --- Theme-Driven Typography ---
        // Determine font size and family by checking legend-specific theme settings 
        // with a fallback to general axis tick styles.
        let font_size = theme.legend_font_size.unwrap_or(theme.tick_label_font_size);
        let font_family = theme.legend_font_family.as_ref().unwrap_or(&theme.tick_label_font_family);
        
        // Toggle layout mode: Top/Bottom are horizontal (row-major), Left/Right are vertical (column-major).
        let is_horizontal = matches!(ctx.legend_position, LegendPosition::Top | LegendPosition::Bottom);

        // 1. Calculate the initial anchor (x, y) where the legend group starts.
        let (mut current_x, mut current_y) = Self::calculate_anchor(ctx, specs, theme, font_size, is_horizontal);

        for spec in specs {
            // 2. Render the Legend Title (e.g., "Species" or "Price Range")
            backend.draw_text(
                &spec.title,
                current_x,
                current_y,
                font_size * 1.1, // Titles are slightly emphasized (110% of base size)
                font_family,     // Theme-driven font family
                &theme.title_color,
                "start",
                "bold",
                1.0,
            );

            // 3. Bridge abstract data values to concrete visual properties (colors/shapes).
            let (labels, colors, shapes) = Self::resolve_mappings(spec, ctx);

            // 4. Draw the actual items (geometric markers + text labels).
            // Title padding is derived from theme's tick_label_padding.
            let item_y_start = current_y + theme.tick_label_padding + 5.0;
            let (block_w, block_h) = Self::draw_spec_group(
                &mut backend,
                &labels,
                &colors,
                shapes.as_deref(),
                current_x,
                item_y_start,
                font_size,
                theme,
                is_horizontal,
            );

            // 5. Update the "cursor" position for the next legend block.
            // If multiple aesthetics are mapped (e.g., color and shape), they are drawn side-by-side or stacked.
            if is_horizontal {
                current_x += block_w + 40.0; // Standard horizontal gap between distinct guides
            } else {
                current_y += block_h + 25.0; // Vertical spacing between distinct guides
            }
        }
    }

    /// Calculates the starting (x, y) coordinate based on the selected Position.
    /// 
    /// For Top/Bottom positions, it performs a 'pre-flight' measurement of all legend blocks
    /// to ensure the group is perfectly centered relative to the Plot Panel.
    fn calculate_anchor(
        ctx: &SharedRenderingContext,
        specs: &[LegendSpec],
        theme: &Theme,
        font_size: f64,
        is_horizontal: bool,
    ) -> (f64, f64) {
        let mut x = ctx.panel.x;
        let mut y = ctx.panel.y;

        // Resolve font family for width estimation (Mono fonts require more space)
        let font_family = theme.legend_font_family.as_ref().unwrap_or(&theme.tick_label_font_family);
        let width_factor = if font_family.contains("Mono") { 0.65 } else { 0.55 };

        // Margin between the plot area and the legend, driven by theme padding.
        let legend_margin = theme.tick_label_padding * 6.0 + 10.0; 
        let block_gap = 40.0; 

        if is_horizontal {
            // --- Flow Layout Calculation (for Centering) ---
            let mut total_width = 0.0;
            for spec in specs {
                if let ScaleDomain::Categorical(values) = &spec.domain {
                    // Estimate title width
                    let title_w = spec.title.len() as f64 * font_size * width_factor;
                    
                    let mut items_w = 0.0;
                    for val in values {
                        // Combined width of Symbol (25px fixed) + Text (dynamic) + Gap (15px)
                        items_w += 25.0 + (val.len() as f64 * font_size * width_factor) + 15.0;
                    }
                    total_width += title_w.max(items_w) + block_gap;
                }
            }
            total_width -= block_gap; // Remove trailing gap

            // Center X relative to panel width
            x = ctx.panel.x + (ctx.panel.width - total_width).max(0.0) / 2.0;
            
            y = if ctx.legend_position == LegendPosition::Top {
                legend_margin // Margin from SVG top
            } else {
                ctx.panel.y + ctx.panel.height + legend_margin // Margin below X-axis
            };
        } else {
            // --- Stack Layout Calculation (Side Placement) ---
            match ctx.legend_position {
                LegendPosition::Right => {
                    x = ctx.panel.x + ctx.panel.width + legend_margin;
                    y = ctx.panel.y;
                }
                LegendPosition::Left => {
                    x = 10.0; // Anchor near left edge of canvas
                    y = ctx.panel.y;
                }
                _ => {}
            }
        }
        (x, y)
    }

    /// Queries the VisualMappers in the SharedContext to retrieve the 
    /// specific colors and shapes assigned to data domain values.
    fn resolve_mappings(
        spec: &LegendSpec,
        ctx: &SharedRenderingContext,
    ) -> (Vec<String>, Vec<String>, Option<Vec<PointShape>>) {
        let mut labels = Vec::new();
        let mut colors = Vec::new();
        let mut shapes = Vec::new();

        if let ScaleDomain::Categorical(domain_values) = &spec.domain {
            for val in domain_values {
                labels.push(val.clone());

                // Resolve color via the Color Scale and Mapper
                if let Some((scale, mapper)) = &ctx.aesthetics.color {
                    let norm = scale.normalize_string(val);
                    let l_max = scale.logical_max();
                    colors.push(mapper.map_to_color(norm, l_max));
                } else {
                    colors.push("#333333".into()); // Fallback color
                }

                // Resolve shape via the Shape Scale and Mapper
                if let Some((scale, mapper)) = &ctx.aesthetics.shape {
                    let norm = scale.normalize_string(val);
                    let l_max = scale.logical_max();
                    shapes.push(mapper.map_to_shape(norm, l_max));
                } else {
                    shapes.push(PointShape::Circle); // Fallback shape
                }
            }
        }
        (labels, colors, if spec.has_shape { Some(shapes) } else { None })
    }

    /// Renders individual items within a legend block and returns the total width/height consumed.
    fn draw_spec_group(
        backend: &mut dyn RenderBackend,
        labels: &[String],
        colors: &[String],
        shapes: Option<&[PointShape]>,
        x: f64,
        y: f64,
        font_size: f64,
        theme: &Theme,
        horizontal: bool,
    ) -> (f64, f64) {
        let mut max_w = 0.0;
        let mut total_h = 0.0;
        let item_spacing = 15.0; // Spacing between individual items in a row
        
        let font_family = theme.legend_font_family.as_ref().unwrap_or(&theme.tick_label_font_family);

        for (i, label) in labels.iter().enumerate() {
            // Rough estimate of this specific item's width
            let item_w = 25.0 + (label.len() as f64 * font_size * 0.6);
            
            // Calculate item position based on layout orientation
            let (ix, iy) = if horizontal {
                (x + max_w, y + 20.0) // Side-by-side
            } else {
                (x, y + 20.0 + (i as f64 * (font_size + 10.0))) // Stacked vertically
            };

            let color = &colors[i % colors.len()];
            let shape = shapes.and_then(|s| s.get(i % s.len())).unwrap_or(&PointShape::Circle);

            // 1. Draw the geometric symbol
            Self::draw_symbol(backend, shape, ix + 5.0, iy, 5.0, color);

            // 2. Draw the label text
            backend.draw_text(
                label,
                ix + 18.0,
                iy + (font_size * 0.3), // Vertical alignment adjustment
                font_size,
                font_family,
                &theme.legent_label_color, // Using theme field: legent_label_color
                "start",
                "normal",
                1.0,
            );

            // Update bounding box metrics
            if horizontal {
                max_w += item_w + item_spacing;
                total_h = font_size + 20.0;
            } else {
                max_w = max_w.max(item_w);
                total_h = (i as f64 + 1.0) * (font_size + 10.0) + 20.0;
            }
        }
        (max_w, total_h)
    }

    /// Renders specific geometric paths based on the PointShape variant.
    /// 
    /// Supports Circle, Square, Triangle, Diamond, Star, and various Polygons.
    fn draw_symbol(
        backend: &mut dyn RenderBackend, 
        shape: &PointShape, 
        cx: f64, 
        cy: f64, 
        r: f64, 
        color: &str
    ) {
        match shape {
            PointShape::Circle => {
                backend.draw_circle(cx, cy, r, Some(color), None, 0.0, 1.0);
            }
            PointShape::Square => {
                backend.draw_rect(cx - r, cy - r, r * 2.0, r * 2.0, Some(color), None, 0.0, 1.0);
            }
            PointShape::Triangle => {
                let pts = vec![(cx, cy - r), (cx - r, cy + r), (cx + r, cy + r)];
                backend.draw_polygon(&pts, Some(color), None, 0.0, 1.0);
            }
            PointShape::Diamond => {
                let pts = vec![(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)];
                backend.draw_polygon(&pts, Some(color), None, 0.0, 1.0);
            }
            PointShape::Star => {
                let mut pts = Vec::with_capacity(10);
                for i in 0..10 {
                    let angle = (i as f64) * PI / 5.0 - PI / 2.0;
                    let radius = if i % 2 == 0 { r } else { r * 0.45 };
                    pts.push((cx + radius * angle.cos(), cy + radius * angle.sin()));
                }
                backend.draw_polygon(&pts, Some(color), None, 0.0, 1.0);
            }
            PointShape::Pentagon | PointShape::Hexagon | PointShape::Octagon => {
                let sides = match shape {
                    PointShape::Pentagon => 5,
                    PointShape::Hexagon => 6,
                    PointShape::Octagon => 8,
                    _ => 4,
                };
                let pts: Vec<(f64, f64)> = (0..sides)
                    .map(|i| {
                        let angle = (i as f64) * 2.0 * PI / (sides as f64) - PI / 2.0;
                        (cx + r * angle.cos(), cy + r * angle.sin())
                    })
                    .collect();
                backend.draw_polygon(&pts, Some(color), None, 0.0, 1.0);
            }
        }
    }
}