use crate::chart::Chart;
use crate::coordinate::{CoordSystem, CoordinateTrait, Rect};
use crate::core::aesthetics::AestheticMapping;
use crate::core::aesthetics::GlobalAesthetics;
use crate::core::context::{ChartSpec, PanelContext};
use crate::core::guide::GuideSpec;
use crate::core::layer::Layer;
use crate::encode::Channel;
use crate::error::ChartonError;
use crate::scale::{
    Expansion, ExplicitTick, Scale, ScaleDomain, create_scale, mapper::VisualMapper,
};
use crate::theme::Theme;
use html_escape::encode_safe;
use std::fmt::Write;
use std::sync::Arc;

/// A complete specification for a visual channel before the final Scale object is created.
pub struct ResolvedSpec {
    pub field: String,
    pub scale_type: Scale,
    pub domain: ScaleDomain,
    pub expand: Expansion,
}

/// `LayeredChart` is the central orchestrator of the visualization.
///
/// It follows the "Specification" pattern:
/// 1. It holds the structural intent (layers, coordinates, data labels).
/// 2. It stores "Overrides" (User-defined domains or layout tweaks) that
///    take precedence over the [Theme] defaults.
#[derive(Clone)]
pub struct LayeredChart {
    // --- Physical Canvas Dimensions ---
    /// The target width of the rendered output in pixels.
    pub(crate) width: u32,
    /// The target height of the rendered output in pixels.
    pub(crate) height: u32,

    // --- Aesthetic Context ---
    /// The active visual theme. Used as the source for all visual constants
    /// and default layout ratios unless overridden below.
    pub(crate) theme: Theme,

    // --- Content ---
    /// The text content of the chart title. Styled by `theme.title_size/color`.
    pub(crate) title: Option<String>,
    /// The collection of plot layers (points, lines, bars, etc.).
    pub(crate) layers: Vec<Arc<dyn Layer>>,
    /// The logical coordinate system (e.g., Cartesian, Polar).
    pub(crate) coord_system: CoordSystem,

    // --- Layout Overrides (The "Override" Pattern) ---
    /// Manual override for chart margins [top, right, bottom, left].
    /// If `None`, `theme.default_margins` will be used during layout resolution.
    pub(crate) top_margin: Option<f64>,
    pub(crate) right_margin: Option<f64>,
    pub(crate) bottom_margin: Option<f64>,
    pub(crate) left_margin: Option<f64>,

    // --- Axis & Scale Overrides (The "Brain") ---
    // These fields define how data is mapped and labeled, overriding automatic inference.
    /// User-defined range for the X-axis.
    pub(crate) x_domain: Option<ScaleDomain>,
    /// Explicit title for the X-axis (e.g., "Time", "GDP").
    pub(crate) x_label: Option<String>,
    /// Custom expansion/padding rules for the X-axis.
    pub(crate) x_expand: Option<Expansion>,
    /// Explicit ticks for the X-axis.
    pub(crate) x_ticks: Option<Vec<ExplicitTick>>,

    /// User-defined range for the Y-axis.
    pub(crate) y_domain: Option<ScaleDomain>,
    /// Explicit title for the Y-axis.
    pub(crate) y_label: Option<String>,
    /// Custom expansion/padding rules for the Y-axis.
    pub(crate) y_expand: Option<Expansion>,
    /// Explicit ticks for the Y-axis.
    pub(crate) y_ticks: Option<Vec<ExplicitTick>>,

    /// User-defined domain for the Color channel (legend).
    pub(crate) color_domain: Option<ScaleDomain>,
    /// Explicit title for the Color legend.
    pub(crate) color_label: Option<String>,
    pub(crate) color_expand: Option<Expansion>,

    pub(crate) shape_domain: Option<ScaleDomain>,
    pub(crate) shape_label: Option<String>,
    pub(crate) shape_expand: Option<Expansion>,

    pub(crate) size_domain: Option<ScaleDomain>,
    pub(crate) size_label: Option<String>,
    pub(crate) size_expand: Option<Expansion>,

    // --- Structural Modifiers ---
    /// Whether to swap the X and Y axes (common for horizontal bar charts).
    pub(crate) flipped: bool,

    // --- Polar Context Overrides ---
    // These override the defaults in `theme.polar_xxx` for specific chart needs.
    pub(crate) polar_start_angle: Option<f64>,
    pub(crate) polar_end_angle: Option<f64>,
    pub(crate) polar_inner_radius: Option<f64>,
}

impl Default for LayeredChart {
    fn default() -> Self {
        Self::new()
    }
}

impl LayeredChart {
    pub fn new() -> Self {
        Self {
            width: 500,
            height: 400,

            theme: Theme::default(),
            title: None,

            layers: Vec::new(),
            coord_system: CoordSystem::default(),

            top_margin: None,
            right_margin: None,
            bottom_margin: None,
            left_margin: None,

            // Initializing all overrides as None (defer to automatic inference)
            x_domain: None,
            x_label: None,
            x_expand: None,
            x_ticks: None,

            y_domain: None,
            y_label: None,
            y_expand: None,
            y_ticks: None,

            color_domain: None,
            color_label: None,
            color_expand: None,

            shape_domain: None,
            shape_label: None,
            shape_expand: None,

            size_domain: None,
            size_label: None,
            size_expand: None,

            flipped: false,

            polar_start_angle: None,
            polar_end_angle: None,
            polar_inner_radius: None,
        }
    }

    /// Resolves the unified visual specification for a given channel across all layers.
    ///
    /// This method performs the critical "Scale Arbitration" process. It ensures that
    /// multiple layers can coexist on the same coordinate axis or aesthetic mapping.
    ///
    /// # The Resolution Pipeline:
    /// 1. **Discovery**: Iterates through all layers to find the common field and scale type.
    /// 2. **Constraint Check**: Validates that all layers agree on the mathematical interpretation (Scale).
    /// 3. **Consolidation**: Merges domains (min/max or unique labels) and expands requirements.
    /// 4. **Override**: Applies explicit user settings (highest priority).
    /// 5. **Finalization**: Applies smart defaults for edge cases (e.g., zero-range data).
    pub fn resolve_scale_spec(
        &self,
        channel: Channel,
    ) -> Result<Option<ResolvedSpec>, ChartonError> {
        // --- Accumulators for Data Inference ---
        let mut inferred_field: Option<String> = None;
        let mut inferred_type: Option<Scale> = None;

        // Domain accumulators
        let mut cont_min = f64::INFINITY;
        let mut cont_max = f64::NEG_INFINITY;
        let mut all_labels: Vec<String> = Vec::new();
        let mut temp_min: i64 = i64::MAX;
        let mut temp_max: i64 = i64::MIN;

        // Expansion accumulators (Finding the "Maximum Room" requested by layers)
        let mut max_mult = (0.0f64, 0.0f64);
        let mut max_add = (0.0f64, 0.0f64);
        let mut has_expansion_info = false;

        // --- Step 1: Scan Layers ---
        for (i, layer) in self.layers.iter().enumerate() {
            let (field, current_type) = match (layer.get_field(channel), layer.get_scale(channel)) {
                (Some(f), Some(t)) => (f, t),
                _ => continue, // Layer does not participate in this channel
            };

            // Scale Type Consistency Check
            if let Some(ref existing_type) = inferred_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "{:?} scale conflict: Layer 0 is {:?}, but layer {} is {:?}",
                        channel, existing_type, i, current_type
                    )));
                }
            } else {
                inferred_field = Some(field);
                inferred_type = Some(current_type);
            }

            // Consolidate Domain Data
            match layer.get_data_bounds(channel)? {
                ScaleDomain::Continuous(min, max) => {
                    cont_min = cont_min.min(min);
                    cont_max = cont_max.max(max);
                }
                ScaleDomain::Discrete(labels) => {
                    for label in labels {
                        if !all_labels.contains(&label) {
                            all_labels.push(label);
                        }
                    }
                }
                ScaleDomain::Temporal(min_ns, max_ns) => {
                    temp_min = temp_min.min(min_ns);
                    temp_max = temp_max.max(max_ns);
                }
            }

            // Consolidate Expansion Requirements
            if let Some(layer_expand) = layer.get_expand(channel) {
                max_mult.0 = max_mult.0.max(layer_expand.mult.0);
                max_mult.1 = max_mult.1.max(layer_expand.mult.1);
                max_add.0 = max_add.0.max(layer_expand.add.0);
                max_add.1 = max_add.1.max(layer_expand.add.1);
                has_expansion_info = true;
            }
        }

        // --- Step 2: Retrieve User Overrides ---
        let (manual_domain, manual_label, manual_expand) = match channel {
            Channel::X => (self.x_domain.clone(), self.x_label.clone(), self.x_expand),
            Channel::Y => (self.y_domain.clone(), self.y_label.clone(), self.y_expand),
            Channel::Color => (
                self.color_domain.clone(),
                self.color_label.clone(),
                self.color_expand,
            ),
            Channel::Shape => (self.shape_domain.clone(), None, self.shape_expand),
            Channel::Size => (self.size_domain.clone(), None, self.size_expand),
        };

        // --- Step 3: Final Reconciliation ---

        // A. Resolve Scale Type
        let scale_type = match (&inferred_type, &manual_domain) {
            (Some(t), _) => *t,
            (None, Some(d)) => match d {
                ScaleDomain::Discrete(_) => Scale::Discrete,
                ScaleDomain::Temporal(_, _) => Scale::Temporal,
                _ => Scale::Linear,
            },
            _ => return Ok(None), // No data and no override
        };

        // B. Resolve Field Label
        let field = manual_label
            .or(inferred_field)
            .unwrap_or_else(|| format!("{:?}", channel));

        // C. Resolve Domain (Priority: Manual > Consolidated)
        let domain = if let Some(d) = manual_domain {
            d
        } else {
            match scale_type {
                Scale::Discrete => {
                    if all_labels.is_empty() {
                        return Ok(None);
                    }
                    ScaleDomain::Discrete(all_labels)
                }
                Scale::Temporal => {
                    if temp_min == i64::MAX || temp_max == i64::MIN {
                        return Ok(None);
                    }
                    ScaleDomain::Temporal(temp_min, temp_max)
                }
                _ => {
                    if cont_min.is_infinite() {
                        ScaleDomain::Continuous(0.0, 1.0)
                    } else {
                        // Zero-range Protection
                        let (mut min, mut max) = (cont_min, cont_max);
                        if (max - min).abs() < 1e-12 {
                            min -= 0.5;
                            max += 0.5;
                        }
                        ScaleDomain::Continuous(min, max)
                    }
                }
            }
        };

        // D. Resolve Expansion (Priority: Manual > Consolidated Max > Type Defaults)
        let expand = if let Some(me) = manual_expand {
            me
        } else if has_expansion_info {
            Expansion {
                mult: max_mult,
                add: max_add,
            }
        } else {
            // Expansion logic depends on both the Scale type and the target Channel.
            match channel {
                // Non-positional channels (Color, Size, Shape) map data points directly
                // to visual identities and typically require zero padding to maintain
                // mathematical limits (e.g., full color scale range).
                Channel::Color | Channel::Size | Channel::Shape => Expansion {
                    mult: (0.0, 0.0),
                    add: (0.0, 0.0),
                },
                // Positional channels (X, Y) require expansion to prevent marks
                // from clipping at the coordinate system boundaries.
                _ => match scale_type {
                    // Discrete scales use an additive constant (0.4) to provide
                    // consistent spacing for bars or categories within their slots.
                    Scale::Discrete => Expansion {
                        mult: (0.0, 0.0),
                        add: (0.4, 0.4),
                    },
                    // Continuous scales apply a 5% multiplicative factor by default
                    // to provide a visual buffer around data points.
                    _ => Expansion::default(),
                },
            }
        };

        Ok(Some(ResolvedSpec {
            field,
            scale_type,
            domain,
            expand,
        }))
    }

    /// Add a layer to the chart
    ///
    /// Adds a new chart layer to create a multi-layered visualization. Each layer can represent
    /// a different data series or chart type, allowing for complex composite visualizations like
    /// line charts overlaid on bar charts.
    ///
    /// Layers are rendered in the order they are added, with the first layer at the bottom
    /// and subsequent layers stacked on top.
    ///
    /// # Arguments
    ///
    /// * `layer` - A Chart instance representing the layer to be added
    ///
    /// # Returns
    ///
    /// Returns the LayeredChart instance for method chaining
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df1 = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
    /// let df2 = df!["x" => [1, 2, 3], "y" => [5, 15, 25]]?;
    ///
    /// let base_layer = Chart::<MarkBar>::build(&df1)?
    ///     .mark_bar()?
    ///     .encode(x("x"), y("y"))?;
    ///     
    /// let overlay_layer = Chart::<MarkLine>::build(&df2)?
    ///     .mark_line()?
    ///     .encode(x("x"), y("y"))?;
    ///
    /// let chart = LayeredChart::new()
    ///     .add_layer(base_layer)
    ///     .add_layer(overlay_layer);
    /// ```
    pub(crate) fn add_layer<T: crate::mark::Mark + 'static>(mut self, layer: Chart<T>) -> Self
    where
        Chart<T>: Layer,
    {
        // Check if the layer has data before adding it
        if layer.data.height() > 0 {
            self.layers.push(Arc::new(layer));
        }
        // If layer is empty, silently ignore it
        self
    }

    /// Consolidates all metadata, data domains, and physical constraints into a final rendering Scene.
    ///
    /// This implementation follows the "Industrial Defense" pipeline:
    /// 1. **Aesthetic Resolution**: Consolidates Color, Shape, and Size scales using unified specs.
    /// 2. **Coordinate Resolution**: Resolves X and Y scales and constructs the Coordinate system.
    /// 3. **Guide Generation**: Collects legend specifications based on merged fields.
    /// 4. **Layout Measurement**: Calculates the physical pixel Rect for the plot panel.
    ///
    /// The output is the "Final Blueprint" required to begin the actual drawing phase.
    #[allow(clippy::type_complexity)] // This is the core resolution result; a type alias isn't needed for a single usage.
    pub fn resolve_scene(
        &self,
    ) -> Result<
        (
            Arc<dyn CoordinateTrait>,
            Rect,
            GlobalAesthetics,
            Vec<GuideSpec>,
        ),
        ChartonError,
    > {
        // --- STEP 1: RESOLVE GLOBAL AESTHETIC MAPPINGS ---
        // We resolve non-positional encodings (Color, Shape, Size) across all layers.

        let color_mapping = if let Some(spec) = self.resolve_scale_spec(Channel::Color)? {
            let mapper = VisualMapper::new_color_default(&spec.scale_type, &self.theme);
            let scale_impl = create_scale(
                &spec.scale_type,
                spec.domain,
                spec.expand,
                Some(mapper.clone()),
            )?;
            Some(AestheticMapping {
                field: spec.field,
                scale_impl,
            })
        } else {
            None
        };

        let shape_mapping = if let Some(spec) = self.resolve_scale_spec(Channel::Shape)? {
            let mapper = VisualMapper::new_shape_default();
            let scale_impl = create_scale(
                &spec.scale_type,
                spec.domain,
                spec.expand,
                Some(mapper.clone()),
            )?;
            Some(AestheticMapping {
                field: spec.field,
                scale_impl,
            })
        } else {
            None
        };

        let size_mapping = if let Some(spec) = self.resolve_scale_spec(Channel::Size)? {
            let mapper = VisualMapper::new_size_default(2.0, 9.0);
            let scale_impl = create_scale(
                &spec.scale_type,
                spec.domain,
                spec.expand,
                Some(mapper.clone()),
            )?;
            Some(AestheticMapping {
                field: spec.field,
                scale_impl,
            })
        } else {
            None
        };

        let aesthetics = GlobalAesthetics::new(color_mapping, shape_mapping, size_mapping);

        // Create the global ChartSpec (Blueprint) early so it can be used for measurement.
        let chart_spec = ChartSpec {
            aesthetics: &aesthetics,
            theme: &self.theme,
        };

        // --- STEP 2: RESOLVE COORDINATE SCALES (X & Y) ---
        let x_spec = self.resolve_scale_spec(Channel::X)?.unwrap();
        let y_spec = self.resolve_scale_spec(Channel::Y)?.unwrap();

        let x_scale = create_scale(&x_spec.scale_type, x_spec.domain, x_spec.expand, None)?;
        let y_scale = create_scale(&y_spec.scale_type, y_spec.domain, y_spec.expand, None)?;

        let final_coord: Arc<dyn CoordinateTrait> = match self.coord_system {
            CoordSystem::Cartesian2D => Arc::new(crate::coordinate::cartesian::Cartesian2D::new(
                x_scale,
                y_scale,
                x_spec.field.clone(),
                y_spec.field.clone(),
                self.flipped,
            )),
            CoordSystem::Polar => {
                // 1. Resolve parameters by prioritizing User Overrides > Theme Defaults.
                // This 'Late Binding' ensures the chart remains responsive to theme changes
                // unless the user explicitly locks a value.
                let start_angle = self
                    .polar_start_angle
                    .unwrap_or(self.theme.polar_start_angle);
                let end_angle = self.polar_end_angle.unwrap_or(self.theme.polar_end_angle);
                let inner_radius = self
                    .polar_inner_radius
                    .unwrap_or(self.theme.polar_inner_radius);

                // 2. Initialize the Polar coordinate system with resolved scales and data fields.
                let mut polar = crate::coordinate::polar::Polar::new(
                    x_scale,
                    y_scale,
                    x_spec.field.clone(),
                    y_spec.field.clone(),
                );

                // 3. Inject the finalized geometric parameters into the execution instance.
                polar.start_angle = start_angle;
                polar.end_angle = end_angle;
                polar.inner_radius = inner_radius;

                Arc::new(polar)
            }
        };

        // --- STEP 3: GUIDE GENERATION ---
        let guide_specs = crate::core::guide::GuideManager::collect_guides(&aesthetics);

        // --- STEP 4: PHYSICAL MEASUREMENT (LAYOUT ENGINE) ---
        let w = self.width as f64;
        let h = self.height as f64;

        let initial_plot_w = w
            * (1.0
                - self.left_margin.unwrap_or(self.theme.left_margin)
                - self.right_margin.unwrap_or(self.theme.right_margin));
        let initial_plot_h = h
            * (1.0
                - self.top_margin.unwrap_or(self.theme.top_margin)
                - self.bottom_margin.unwrap_or(self.theme.bottom_margin));

        // A. Measure Legend Constraints.
        let legend_box = crate::core::layout::LayoutEngine::calculate_legend_constraints(
            &guide_specs,
            self.theme.legend_position,
            w,
            h,
            initial_plot_w,
            initial_plot_h,
            self.theme.legend_margin,
            &self.theme,
        );

        // B. Measure Axis Constraints using a temporary PanelContext.
        // We calculate a 'rough' panel area first to allow the engine to estimate
        // tick density and label overlap.
        let temp_panel = Rect::new(
            (self.left_margin.unwrap_or(self.theme.left_margin) * w) + legend_box.left,
            (self.top_margin.unwrap_or(self.theme.top_margin) * h) + legend_box.top,
            (initial_plot_w - legend_box.left - legend_box.right).max(10.0),
            (initial_plot_h - legend_box.top - legend_box.bottom).max(10.0),
        );

        // Create the temporary context required for layout measurement.
        let temp_ctx = PanelContext::new(&chart_spec, final_coord.clone(), temp_panel);

        let axis_box = crate::core::layout::LayoutEngine::calculate_axis_constraints(
            &temp_ctx,
            &self.theme,
            temp_panel.width,
            temp_panel.height,
        );

        // --- STEP 5: FINAL PANEL RESOLUTION ---
        let final_left = (self.left_margin.unwrap_or(self.theme.left_margin) * w)
            + legend_box.left
            + axis_box.left;
        let final_right =
            (self.right_margin.unwrap_or(self.theme.right_margin) * w) + legend_box.right;
        let final_top = (self.top_margin.unwrap_or(self.theme.top_margin) * h) + legend_box.top;
        let final_bottom = (self.bottom_margin.unwrap_or(self.theme.bottom_margin) * h)
            + legend_box.bottom
            + axis_box.bottom;

        // Apply final dimensions with a safety floor (min_panel_size).
        let plot_w = (w - final_left - final_right).max(self.theme.min_panel_size);
        let plot_h = (h - final_top - final_bottom).max(self.theme.min_panel_size);

        let final_panel_rect = Rect::new(final_left, final_top, plot_w, plot_h);

        Ok((final_coord, final_panel_rect, aesthetics, guide_specs))
    }

    /// Renders the chart title at the top-center of the SVG canvas.
    ///
    /// In this revised implementation, the title position is no longer a fixed offset.
    /// Instead, it dynamically calculates its vertical position to be centered within
    /// the space defined by `top_margin`. This ensures the title remains visually
    /// balanced even as the chart scales or if large margins are specified.
    fn render_title(&self, svg: &mut String, panel: &Rect) -> Result<(), ChartonError> {
        // 1. Guard: Check if a title exists.
        let title_text = match &self.title {
            Some(t) => t,
            None => return Ok(()),
        };

        // 2. Horizontal Positioning:
        // Use the full canvas width to find the absolute horizontal center.
        let center_x = self.width as f64 / 2.0;

        // 3. Vertical Positioning Logic:
        // Instead of a hardcoded '25.0', we calculate the available vertical space
        // above the plot panel (panel.y).
        // We place the text's baseline in the middle of this area.
        let title_area_height = panel.y;
        let font_size = self.theme.title_size;

        // Calculate the vertical midpoint.
        // Note: Using 'dominant-baseline="middle"' allows us to use the exact midpoint as the Y coordinate.
        let center_y = title_area_height / 4.0;

        // 4. Style Metadata Extraction:
        let font_family = &self.theme.title_family;
        let font_color = &self.theme.title_color;

        // 5. SVG Generation:
        // - x: Absolute horizontal center.
        // - y: Midpoint of the top margin area.
        // - text-anchor="middle": Centers the text horizontally.
        // - dominant-baseline="middle": Centers the text vertically around the Y coordinate.
        writeln!(
            svg,
            r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" dominant-baseline="middle" font-family="{}" font-size="{}" fill="{}" font-weight="bold">{}</text>"#,
            center_x,
            center_y,
            font_family,
            font_size,
            font_color.to_css_string(),
            encode_safe(title_text)
        )?;

        Ok(())
    }

    /// Renders the entire layered chart to the provided SVG string.
    ///
    /// This implementation coordinates the final rendering pipeline with a clear separation
    /// between global specifications (ChartSpec) and local drawing environments (PanelContext).
    pub fn render(&mut self, svg: &mut String) -> Result<(), ChartonError> {
        // 0. Guard: Ensure there's something to render.
        if self.layers.is_empty() {
            return Ok(());
        }

        // --- STEP 1: SCENE RESOLUTION ---
        // Resolve scale training, unified aesthetic mapping, and physical layout measurement.
        // Returns the final unified coordinate system, the plotting rect,
        // global aesthetics, and the calculated legend specifications.
        let (coord, panel, aesthetics, guide_specs) = self.resolve_scene()?;

        // --- STEP 2: GLOBAL SPECIFICATION SETUP ---
        // We initialize the ChartSpec, which serves as the "Global Source of Truth".
        // This spec is immutable and shared across all potential panels (facets).
        let spec = ChartSpec {
            aesthetics: &aesthetics,
            theme: &self.theme,
        };

        // --- STEP 3: LAYER SYNCHRONIZATION (The "Back-fill") ---
        // Inject the resolved global state into each layer. This allows layers to
        // prepare for rendering (e.g., pre-calculating aesthetic mappings).
        for layer in self.layers.iter() {
            layer.inject_resolved_scales(coord.clone(), &aesthetics);
        }

        // --- STEP 4: ORCHESTRATED DRAWING ---
        // NOTE: In the future, for Faceted plots, this section will wrap in a loop
        // that iterates over multiple PanelContexts created by a 'FacetEngine'.

        // 4a. Initialize the Primary Panel Context.
        let primary_panel_ctx = PanelContext::new(&spec, coord.clone(), panel);

        // 4b. Render Chart Title.
        // Title is typically global to the entire chart canvas.
        self.render_title(svg, &primary_panel_ctx.panel)?;

        // 4c. Render Axes (X and Y).
        // Only render axes if the theme allows and at least one layer requires them.
        if self.theme.show_axes && self.layers.iter().any(|l| l.requires_axes()) {
            let x_label = coord.get_x_label();
            let y_label = coord.get_y_label();

            let x_explicit = self.x_ticks.as_deref();
            let y_explicit = self.y_ticks.as_deref();

            primary_panel_ctx.coord.render_axes(
                svg,
                &self.theme,
                &primary_panel_ctx.panel,
                x_label,
                x_explicit,
                y_label,
                y_explicit,
            )?;
        }

        // 4d. Render Marks (Data Geometries).
        // We create a backend with a clipping region defined by the current panel.
        let mut backend =
            crate::render::backend::svg::SvgBackend::new(svg, Some(&primary_panel_ctx.panel));

        for layer in &self.layers {
            // Each layer renders its marks within the provided PanelContext.
            layer.render_marks(&mut backend, &primary_panel_ctx)?;
        }

        // 4e. Render Unified Legends & Guides.
        // Legends are rendered globally, using the ChartSpec for visual rules.
        crate::render::legend_renderer::LegendRenderer::render_legend(
            svg,
            &guide_specs,
            &self.theme,
            &primary_panel_ctx,
        );

        Ok(())
    }

    /// Generates the complete SVG string for the chart.
    ///
    /// This method serves as the core rendering entry point. To maintain the original
    /// chart state (the "recipe") for potential multiple exports (e.g., saving as
    /// both SVG and PNG), it performs the following:
    /// 1. Creates a decoupled clone of itself.
    /// 2. Executes the stateful "Training Phase" and drawing logic on the clone.
    /// 3. Wraps the result in standard SVG XML headers and background elements.
    ///
    /// # Returns
    /// A Result containing the full SVG markup string or a ChartonError.
    fn generate_svg(&self) -> Result<String, ChartonError> {
        let mut svg_content = String::new();

        // 1. SVG Header & ViewBox Setup
        // We define the width, height, and viewBox to ensure the chart scales
        // correctly across different screen resolutions and aspect ratios.
        svg_content.push_str(&format!(
            r#"<svg width="{}" height="{}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">"#,
            self.width, self.height, self.width, self.height
        ));

        // 2. Background Layer
        // The background color is now managed by the global theme.
        // We render a full-size rectangle using the theme's background_color.
        svg_content.push_str(&format!(
            r#"<rect width="100%" height="100%" fill="{}" />"#,
            self.theme.background_color.to_css_string()
        ));

        // 3. Local State Training & Rendering
        // Because the rendering process (specifically the Sync/Back-fill phase)
        // mutates internal scales and domains to ensure visual consistency,
        // we operate on a mutable clone. This preserves 'self' for future calls.
        let mut chart_instance = self.clone();

        // Pass the mutable reference of the clone to the rendering pipeline.
        chart_instance.render(&mut svg_content)?;

        // 4. Finalize SVG Document
        // Close the root SVG tag to complete the XML structure.
        svg_content.push_str("</svg>");

        Ok(svg_content)
    }

    /// Generates and returns the SVG representation of the chart.
    ///
    /// This method renders the entire chart as an SVG (Scalable Vector Graphics) string,
    /// including all layers, axes, labels, legends, and other visual elements. The
    /// generated SVG can be embedded directly in HTML documents.
    ///
    /// # Returns
    /// A Result containing either:
    /// - Ok(String) with the complete SVG markup of the chart
    /// - Err(ChartonError) if there was an error during rendering
    ///
    /// # Example
    /// ```rust,ignore
    /// let svg_string = chart.to_svg()?;
    /// std::fs::write("chart.svg", svg_string)?;
    /// ```
    pub fn to_svg(&self) -> Result<String, ChartonError> {
        self.generate_svg()
    }

    /// Generate the chart and display in Jupyter
    ///
    /// Renders the chart as an SVG and displays it directly in a Jupyter notebook
    /// environment using the EVCXR kernel. This method is specifically designed
    /// for interactive data exploration in Jupyter notebooks.
    ///
    /// The method automatically detects if it's running in an EVCXR environment
    /// and will only display the chart in that context. In other environments,
    /// this method will successfully execute but won't produce any visible output.
    ///
    /// # Returns
    ///
    /// Returns a Result indicating success or a ChartonError if SVG generation fails
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
    /// let chart = Chart::build(&df)?
    ///     .mark_point()?
    ///     .encode(X::new("x"), Y::new("y"))?;
    ///
    /// chart.show()?; // Displays in Jupyter notebook
    /// ```
    pub fn show(&self) -> Result<(), ChartonError> {
        let svg_content = self.generate_svg()?;

        // Check if we're in EVCXR Jupyter environment
        if std::env::var("EVCXR_IS_RUNTIME").is_ok() {
            println!(
                "EVCXR_BEGIN_CONTENT text/html\n{}\nEVCXR_END_CONTENT",
                svg_content
            );
        }

        Ok(())
    }

    /// Generate the chart and save to file
    ///
    /// Renders the chart and saves it to the specified file path. The format is determined
    /// by the file extension in the path. Currently, only SVG and PNG format are supported.
    ///
    /// # Arguments
    ///
    /// * `path` - A path-like object specifying where to save the chart file
    ///
    /// # Returns
    ///
    /// Returns a Result indicating success or a ChartonError if SVG/PNG generation or file writing fails
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
    /// let chart = Chart::build(&df)?
    ///     .mark_point()?
    ///     .encode(alt::x("x"), alt::y("y"))?;
    ///
    /// chart.save("my_chart.svg")?; // Save as SVG file
    /// ```
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ChartonError> {
        let svg_content = self.generate_svg()?;

        // Convert to Path for file operations
        let path_obj = path.as_ref();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path_obj.parent().filter(|p| !p.exists()) {
            std::fs::create_dir_all(parent).map_err(|e| {
                ChartonError::Io(std::io::Error::other(format!(
                    "Failed to create directory: {}",
                    e
                )))
            })?;
        }

        let ext = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        match ext.as_deref() {
            Some("svg") => {
                std::fs::write(path_obj, svg_content).map_err(ChartonError::Io)?;
            }
            Some("png") => {
                #[cfg(feature = "png")]
                {
                    // Load system fonts
                    let mut opts = resvg::usvg::Options::default();

                    // 1. Create a new fontdb instead of cloning the default one
                    let mut fontdb = resvg::usvg::fontdb::Database::new();

                    // 2. Load system fonts (utilizing resources from various OS)
                    fontdb.load_system_fonts();

                    // 3. Load built-in "emergency" font to ensure display even in extreme environments
                    let default_font_data = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
                    fontdb.load_font_data(default_font_data.to_vec());

                    // 4. Set explicit family mappings (Fallback logic)
                    // When users specify "sans-serif" but the system doesn't have mappings configured,
                    // resvg will try this font as a fallback.
                    fontdb.set_sans_serif_family("Inter");

                    opts.fontdb = std::sync::Arc::new(fontdb);

                    // Parse svg string
                    let tree = resvg::usvg::Tree::from_str(&svg_content, &opts)
                        .map_err(|e| ChartonError::Render(format!("SVG parsing error: {:?}", e)))?;

                    // Scale the image size to higher resolution
                    let pixmap_size = tree.size();
                    let scale = 2.0;
                    let width = (pixmap_size.width() * scale) as u32;
                    let height = (pixmap_size.height() * scale) as u32;

                    // Create pixmap
                    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
                        .ok_or(ChartonError::Render("Failed to create pixmap".into()))?;

                    // Render and save
                    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
                    resvg::render(&tree, transform, &mut pixmap.as_mut());
                    pixmap
                        .save_png(path_obj)
                        .map_err(|e| ChartonError::Render(format!("PNG saving error: {:?}", e)))?;
                }

                #[cfg(not(feature = "png"))]
                {
                    return Err(ChartonError::Unimplemented(
                        "PNG support is disabled. Please enable the 'png' feature".to_string(),
                    ));
                }
            }
            Some(format) => {
                return Err(ChartonError::Unimplemented(format!(
                    "Output format '{}' is not supported",
                    format
                )));
            }
            None => {
                return Err(ChartonError::Unimplemented(
                    "Output format could not be determined from file extension".to_string(),
                ));
            }
        }

        Ok(())
    }
}
