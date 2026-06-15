use crate::Precision;
use crate::chart::Chart;
use crate::coordinate::{CoordSystem, CoordinateTrait, Rect};
use crate::core::aesthetics::AestheticMapping;
use crate::core::aesthetics::GlobalAesthetics;
use crate::core::context::{ChartSpec, PanelContext};
use crate::core::guide::GuideSpec;
use crate::core::layer::{Layer, RectConfig, RenderBackend, TextConfig};
use crate::encode::Channel;
use crate::error::ChartonError;
use crate::scale::{
    Expansion, ExplicitTick, Scale, ScaleDomain, create_scale, mapper::VisualMapper,
};
use crate::theme::Theme;
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

    // The device pixel ratio for raster rendering. Defaults to 2.0.
    pub(crate) scale_factor: f32,
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

            scale_factor: 2.0,
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
    fn render_title<B: RenderBackend>(
        &self,
        backend: &mut B,
        panel: &Rect,
    ) -> Result<(), ChartonError> {
        // 1. Guard: Check if a title exists.
        let title_text = match &self.title {
            Some(t) => t,
            None => return Ok(()),
        };

        // 2. Horizontal Positioning:
        // Use the full canvas width to find the absolute horizontal center.
        let center_x = self.width as f64 / 2.0;

        // 3. Vertical Positioning Logic:
        // We calculate the available vertical space above the plot panel (panel.y).
        // We place the text's baseline in the middle of this area.
        let title_area_height = panel.y;
        let font_size = self.theme.title_size;

        // Calculate the vertical midpoint.
        // Note: Using 'dominant-baseline="middle"' allows us to use the exact midpoint as the Y coordinate.
        let center_y = title_area_height / 3.0;

        // 4. Style Metadata Extraction:
        let font_family = &self.theme.title_family;
        let font_color = &self.theme.title_color;

        // 5. Construct TextConfig and Draw
        let config = TextConfig {
            x: center_x as Precision,
            y: center_y as Precision,
            text: title_text.clone(),
            font_size: font_size as Precision,
            font_family: font_family.clone(),
            color: *font_color,
            text_anchor: "middle".to_string(),
            dominant_baseline: "middle".into(),
            font_weight: "bold".to_string(),
            opacity: 1.0,
            angle: 0.0,
        };

        backend.draw_text(config);

        Ok(())
    }

    /// Renders the entire layered chart to the provided SVG string.
    ///
    /// This implementation coordinates the final rendering pipeline with a clear separation
    /// between global specifications (ChartSpec) and local drawing environments (PanelContext).
    pub fn render<B: RenderBackend>(&mut self, backend: &mut B) -> Result<(), ChartonError> {
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

        // 4b. Render grid lines

        // 4c. Render Chart Title.
        // Title is typically global to the entire chart canvas.
        self.render_title(backend, &primary_panel_ctx.panel)?;

        // 4d. Render Marks (Data Geometries) wrapped in a State Machine Scope.
        // We activate clipping using the resolved physical panel bounds to lock
        // chart marks inside the data viewport across all supported backends.
        backend.begin_clip_scope(&primary_panel_ctx.panel);

        for layer in &self.layers {
            // Each layer renders its marks within the isolated PanelContext.
            layer.render_marks(backend, &primary_panel_ctx)?;
        }

        // Deactivate the clipping scope to restore the global drawing canvas.
        backend.end_clip_scope();

        // 4e. Render Axes (X and Y).
        // Only render axes if the theme allows and at least one layer requires them.
        if self.theme.show_axes && self.layers.iter().any(|l| l.requires_axes()) {
            let x_label = coord.get_x_label();
            let y_label = coord.get_y_label();

            let x_explicit = self.x_ticks.as_deref();
            let y_explicit = self.y_ticks.as_deref();

            primary_panel_ctx.coord.render_axes(
                backend,
                &self.theme,
                &primary_panel_ctx.panel,
                x_label,
                x_explicit,
                y_label,
                y_explicit,
            )?;
        }

        // 4f. Render Unified Legends & Guides.
        // Legends are rendered globally, using the ChartSpec for visual rules.
        crate::render::legend_renderer::LegendRenderer::render_legend(
            backend,
            &guide_specs,
            &self.theme,
            &primary_panel_ctx,
        );

        Ok(())
    }

    /// Generates and returns the SVG representation of the chart.
    ///
    /// This method renders the entire chart as an SVG string. It creates a mutable
    /// clone of the chart to perform the stateful training phase (syncing scales
    /// and aesthetics) without mutating the original chart instance.
    ///
    /// # Returns
    /// A Result containing the complete SVG markup or a ChartonError.
    pub fn to_svg(&self) -> Result<String, ChartonError> {
        let mut chart_instance = self.clone();
        let mut svg_content = String::new();

        // 1. SVG Header & ViewBox Setup
        // Define dimensions and coordinate system to ensure proper scaling.
        svg_content.push_str(&format!(
            r#"<svg width="{}" height="{}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">"#,
            self.width, self.height, self.width, self.height
        ));

        // 2. Localized Backend Scope
        // We initialize the SvgBackend. The background is rendered through the
        // backend interface to ensure consistency across different output formats.
        {
            let mut backend = crate::render::backend::svg::SvgBackend::new(&mut svg_content);

            // Render Background
            backend.draw_rect(RectConfig {
                x: 0.0,
                y: 0.0,
                width: self.width as Precision,
                height: self.height as Precision,
                fill: self.theme.background_color,
                stroke: "none".into(),
                stroke_width: 0.0,
                opacity: 1.0,
            });

            // Orchestrate the full rendering pipeline
            chart_instance.render(&mut backend)?;
        }

        // 3. Finalize SVG Document
        svg_content.push_str("</svg>");

        Ok(svg_content)
    }

    /// Generates and returns a PNG representation of the chart as a byte vector.
    ///
    /// This method renders the entire chart into a pixel buffer using the `tiny-skia`
    /// engine. It creates a mutable clone of the chart to perform the stateful
    /// "Training Phase" (synchronizing scales and aesthetics) while preserving
    /// the original chart as an immutable recipe.
    ///
    /// # Returns
    /// A Result containing the PNG encoded bytes or a ChartonError.
    #[cfg(feature = "png")]
    pub fn to_png(&self) -> Result<Vec<u8>, ChartonError> {
        // 1. Create a mutable clone for the stateful rendering phase.
        // This ensures the training phase doesn't mutate the original chart instance.
        let mut chart_instance = self.clone();

        // 2. Initialize the Pixmap (pixel buffer).
        // If dimensions are invalid or memory allocation fails, we return a descriptive Render error.
        let mut pixmap = tiny_skia::Pixmap::new(
            (self.width as f32 * self.scale_factor) as u32,
            (self.height as f32 * self.scale_factor) as u32,
        )
        .ok_or_else(|| {
            ChartonError::Render("Invalid chart dimensions or out of memory for Pixmap".to_string())
        })?;

        // 3. Localized Backend Scope.
        {
            // Renders the chart using the configured scale_factor (defaulting to 2.0x for High-DPI quality).
            // The backend automatically retrieves the globally cached 'static font from utils.
            let mut backend =
                crate::render::backend::raster::RasterBackend::new(&mut pixmap, self.scale_factor);

            // Render Background.
            // We use the backend's draw_rect to ensure the background is the first
            // layer in the drawing stack.
            backend.draw_rect(RectConfig {
                x: 0.0,
                y: 0.0,
                width: self.width as Precision,
                height: self.height as Precision,
                fill: self.theme.background_color,
                stroke: "none".into(),
                stroke_width: 0.0,
                opacity: 1.0,
            });

            // Execute the unified rendering pipeline.
            // This is the core logic shared between all export formats (SVG, PNG, etc.).
            chart_instance.render(&mut backend)?;
        }

        // 4. Finalize PNG Document.
        // Encode the raw pixel buffer into a standard PNG byte stream.
        let png_bytes = pixmap
            .encode_png()
            .map_err(|e| ChartonError::Render(format!("Failed to encode PNG: {}", e)))?;

        Ok(png_bytes)
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
        let svg_content = self.to_svg()?;

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
    ///
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ChartonError> {
        let path_obj = path.as_ref();

        // Create parent directories if they do not exist
        if let Some(parent) = path_obj.parent().filter(|p| !p.exists()) {
            std::fs::create_dir_all(parent).map_err(ChartonError::Io)?;
        }

        // Extract and normalize the file extension
        let ext = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        match ext.as_deref() {
            Some("svg") => {
                let svg_content = self.to_svg()?;
                std::fs::write(path_obj, svg_content).map_err(ChartonError::Io)?;
            }
            Some("pdf") => {
                #[cfg(feature = "pdf")]
                {
                    let svg_content = self.to_svg()?;
                    let mut opts = svg2pdf::usvg::Options::default();
                    opts.fontdb = crate::core::utils::get_font_db();

                    // Parse the raw SVG string into a usvg render tree
                    let tree = svg2pdf::usvg::Tree::from_str(&svg_content, &opts)
                        .map_err(|e| ChartonError::Render(format!("SVG parsing error: {:?}", e)))?;

                    // Compile the tree into standard binary PDF bytes
                    let pdf_data = svg2pdf::to_pdf(
                        &tree,
                        svg2pdf::ConversionOptions::default(),
                        svg2pdf::PageOptions::default(),
                    )
                    .map_err(|e| ChartonError::Render(format!("PDF generation error: {:?}", e)))?;

                    std::fs::write(path_obj, pdf_data).map_err(ChartonError::Io)?;
                }
                #[cfg(not(feature = "pdf"))]
                {
                    return Err(ChartonError::Unimplemented(
                        "PDF support is disabled. Please enable the 'pdf' feature".to_string(),
                    ));
                }
            }
            Some("png") => {
                // Branch 1: High-performance GPU-accelerated rendering via wgpu
                #[cfg(all(feature = "wgpu", feature = "png"))]
                {
                    // Block on the async GPU pipeline to execute synchronously within this thread context
                    pollster::block_on(self.save_wgpu_png(path_obj))?;
                }

                // Branch 2: Standard CPU-bound fallback rendering via tiny-skia
                #[cfg(all(feature = "png", not(feature = "wgpu")))]
                {
                    let png_data = self.to_png()?;
                    std::fs::write(path_obj, png_data).map_err(ChartonError::Io)?;
                }

                // Branch 3: Guard rail triggered if no raster backends are active
                #[cfg(not(feature = "png"))]
                {
                    return Err(ChartonError::Unimplemented(
                        "To save PNG images onto the local file system, you must also enable the 'png' feature."
                        .to_string()
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

    /// Shared core rendering path: render only GPU primitives, return text ledger for external compositing.
    /// Architecture: WGPU = geometry only, text = deferred to caller
    #[cfg(feature = "wgpu")]
    async fn render_primitive_only(
        &self,
        backend: &mut crate::render::backend::wgpu::WgpuBackend,
        target_view: &wgpu::TextureView,
    ) -> Result<Vec<TextConfig>, ChartonError> {
        // Reset backend state for a clean frame pass
        backend.reset();
        backend.collected_texts.clear();

        let scaled_width = self.width as f32 * self.scale_factor;
        let scaled_height = self.height as f32 * self.scale_factor;

        // Base layer: Draw solid white background
        backend.draw_rect(RectConfig {
            x: 0.0,
            y: 0.0,
            width: scaled_width,
            height: scaled_height,
            fill: "#FFFFFF".into(),
            stroke: "none".into(),
            stroke_width: 0.0,
            opacity: 1.0,
        });

        // Render all chart geometry (text marks are intercepted and deferred to backend.collected_texts)
        let mut chart_clone = self.clone();
        chart_clone.render(backend)?;

        // Create or maintain a container.
        // In highly interactive loops, you would pass this frame_ledger from the outer app context.
        let mut frame_ledger = Vec::with_capacity(backend.collected_texts.len());

        // Flush and populate the reusable frame_ledger
        backend.flush_and_render(target_view, &mut frame_ledger);

        Ok(frame_ledger)
    }

    /// Future-proof entry: render chart into an external WGPU texture view.
    /// Expects an active backend instance passed from the host application (e.g., Bevy/egui system).
    #[cfg(feature = "wgpu")]
    pub async fn render_to_surface(
        &self,
        backend: &mut crate::render::backend::wgpu::WgpuBackend,
        target_view: &wgpu::TextureView,
    ) -> Result<Vec<TextConfig>, ChartonError> {
        // Re-use external backend to avoid resource reallocation overhead in interactive loops
        self.render_primitive_only(backend, target_view).await
    }

    /// Desktop/Headless: Render via WGPU and save to PNG with tiny-skia text compositing.
    #[cfg(all(feature = "wgpu", feature = "png"))]
    pub async fn save_wgpu_png<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<(), ChartonError> {
        use crate::render::backend::wgpu::WgpuBackend;

        // 1. Initialize headless wgpu instance
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .map_err(|_| ChartonError::Render("Failed to request wgpu adapter".to_string()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|e| ChartonError::Render(format!("wgpu device error: {}", e)))?;

        // 2. Create an off-screen texture with HiDPI scaling
        let scaled_width = (self.width as f32 * self.scale_factor) as u32;
        let scaled_height = (self.height as f32 * self.scale_factor) as u32;

        let texture_size = wgpu::Extent3d {
            width: scaled_width,
            height: scaled_height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Chart Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 3. Initialize backend & render core primitives
        let mut backend = WgpuBackend::new(
            device.clone(),
            queue.clone(),
            self.width,
            self.height,
            self.scale_factor,
        )
        .await;

        // Single pass generation: yields geometry on GPU and text ledger on CPU
        let saved_texts = self.render_primitive_only(&mut backend, &view).await?;

        // 4. Readback buffer setup
        let bytes_per_pixel = 4;
        let unpadded_bytes_per_row = scaled_width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padding;
        let buffer_size = padded_bytes_per_row * scaled_height;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback Buffer"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(scaled_height),
                },
            },
            texture_size,
        );
        queue.submit(std::iter::once(encoder.finish()));

        // 5. Map GPU buffer asynchronously and await synchronization fence
        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        // Block host thread until GPU fence execution guarantees data availability
        let _ = device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        rx.recv()
            .unwrap()
            .map_err(|e| ChartonError::Render(format!("Buffer mapping failed: {:?}", e)))?;

        // 6. Convert raw texture pixels to tiny_skia compatible layout (BGRA Premultiplied)
        let mut skia_pixels = Vec::with_capacity((scaled_width * scaled_height * 4) as usize);

        {
            // Explicit scope restricts raw pointer lifetime before unmapping the buffer
            let raw_padded_data = buffer_slice.get_mapped_range();
            for row in 0..scaled_height {
                let start = (row * padded_bytes_per_row) as usize;
                for col in 0..scaled_width as usize {
                    let pixel_offset = start + (col * 4);
                    let r = raw_padded_data[pixel_offset];
                    let g = raw_padded_data[pixel_offset + 1];
                    let b = raw_padded_data[pixel_offset + 2];
                    let a = raw_padded_data[pixel_offset + 3];

                    // Performance Fast-Path: Bypass floating-point arithmetic for fully opaque pixels
                    if a == 255 {
                        skia_pixels.push(r);
                        skia_pixels.push(g);
                        skia_pixels.push(b);
                        skia_pixels.push(255);
                    } else {
                        let alpha_factor = a as f32 / 255.0;
                        let premultiplied_r =
                            (r as f32 * alpha_factor).round().clamp(0.0, 255.0) as u8;
                        let premultiplied_g =
                            (g as f32 * alpha_factor).round().clamp(0.0, 255.0) as u8;
                        let premultiplied_b =
                            (b as f32 * alpha_factor).round().clamp(0.0, 255.0) as u8;

                        skia_pixels.push(premultiplied_r);
                        skia_pixels.push(premultiplied_g);
                        skia_pixels.push(premultiplied_b);
                        skia_pixels.push(a);
                    }
                }
            }
        }
        buffer.unmap();

        // 7. Initialize tiny_skia CPU canvas wrapped over the WGPU raw pixel payload
        let mut pixmap = tiny_skia::Pixmap::from_vec(
            skia_pixels,
            tiny_skia::IntSize::from_wh(scaled_width, scaled_height)
                .ok_or_else(|| ChartonError::Render("Invalid dimensions for Pixmap".to_string()))?,
        )
        .ok_or_else(|| ChartonError::Render("Failed to create Pixmap from GPU data".to_string()))?;

        // 8. Compositing layer: Draw ALL DEFERRED TEXT with tiny-skia (CPU)
        {
            let mut skia_backend =
                crate::render::backend::raster::RasterBackend::new(&mut pixmap, self.scale_factor);

            // Render text collected from the deferred ledger
            for text_config in saved_texts {
                skia_backend.draw_text(text_config);
            }
        }

        // 9. Encode image and serialize to filesystem target
        let png_bytes = pixmap
            .encode_png()
            .map_err(|e| ChartonError::Render(format!("PNG encoding failed: {}", e)))?;
        std::fs::write(path, png_bytes).map_err(ChartonError::Io)?;

        Ok(())
    }

    /// WASM/Web: Render to HTML Canvas using WGPU via cached virtual surface + native Canvas2D text compositing.
    #[cfg(feature = "wgpu")]
    pub async fn render_to_canvas(&self, canvas_id: &str) -> Result<(), ChartonError> {
        #[cfg(target_arch = "wasm32")]
        {
            use crate::render::backend::wgpu::WgpuBackend;
            use std::cell::RefCell;
            use std::collections::HashMap;
            use std::rc::Rc;
            use wasm_bindgen::JsCast;
            use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

            thread_local! {
                /// Global thread-local cache to persist WebGPU contexts and prevent expensive re-initialization overhead on hot-reloads
                static RENDER_CACHE: RefCell<HashMap<String, Rc<RenderState>>> = RefCell::new(HashMap::new());
            }

            struct RenderState {
                surface: wgpu::Surface<'static>,
                adapter: wgpu::Adapter,
                device: wgpu::Device,
                /// Allow dead_code warning. The queue reference must be explicitly held by the struct
                /// to preserve its lifespan and prevent premature dropping of the WebGPU command queue.
                #[allow(dead_code)]
                queue: wgpu::Queue,
                text_canvas: HtmlCanvasElement,
                /// Persistent rendering backend pipeline cache to handle dynamic hot-reloads seamlessly
                backend: RefCell<WgpuBackend>,
            }

            let window =
                web_sys::window().ok_or_else(|| ChartonError::Render("No window found".into()))?;
            let document = window
                .document()
                .ok_or_else(|| ChartonError::Render("No document found".into()))?;

            let host_canvas = document
                .get_element_by_id(canvas_id)
                .ok_or_else(|| ChartonError::Render(format!("Canvas {} not found", canvas_id)))?
                .dyn_into::<HtmlCanvasElement>()
                .map_err(|_| ChartonError::Render("Element is not a canvas".into()))?;

            let dpr = window.device_pixel_ratio();
            let display_width = (self.width as f64 * dpr).round() as u32;
            let display_height = (self.height as f64 * dpr).round() as u32;

            // Enforce synchronized matching dimensions between HTML canvas bounds and underlying WebGPU buffer
            host_canvas.set_width(display_width);
            host_canvas.set_height(display_height);

            let state = if let Some(cached) =
                RENDER_CACHE.with(|c| c.borrow().get(canvas_id).cloned())
            {
                cached
            } else {
                // Initialize text overlay canvas if a cache miss occurs
                let text_canvas = document
                    .create_element("canvas")
                    .map_err(|_| ChartonError::Render("Failed to create text canvas".into()))?
                    .dyn_into::<HtmlCanvasElement>()
                    .map_err(|_| ChartonError::Render("Text element is not a canvas".into()))?;

                text_canvas.set_id(&format!("{}_text_layer", canvas_id));

                let html_element = text_canvas
                    .dyn_ref::<web_sys::HtmlElement>()
                    .ok_or_else(|| ChartonError::Render("Failed to cast to HtmlElement".into()))?;

                // Style configurations for matching overlay alignment and blending
                html_element
                    .style()
                    .set_property("position", "absolute")
                    .unwrap();
                html_element.style().set_property("top", "0").unwrap();
                html_element.style().set_property("left", "0").unwrap();
                html_element.style().set_property("width", "100%").unwrap();
                html_element.style().set_property("height", "100%").unwrap();
                html_element
                    .style()
                    .set_property("pointer-events", "none")
                    .unwrap();
                html_element
                    .style()
                    .set_property("background", "transparent")
                    .unwrap();

                if let Some(parent) = host_canvas.parent_node() {
                    parent.append_child(&text_canvas).unwrap();
                }

                let instance = wgpu::Instance::default();
                let surface_target = wgpu::SurfaceTarget::Canvas(host_canvas.clone());
                let surface = instance.create_surface(surface_target).map_err(|e| {
                    ChartonError::Render(format!("Failed to create Web surface: {}", e))
                })?;

                let adapter = instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        compatible_surface: Some(&surface),
                        power_preference: wgpu::PowerPreference::HighPerformance,
                        force_fallback_adapter: false,
                    })
                    .await
                    .map_err(|e| ChartonError::Render(format!("GPU adapter err: {:?}", e)))?;

                let (device, queue) = adapter
                    .request_device(&wgpu::DeviceDescriptor::default())
                    .await
                    .map_err(|e| ChartonError::Render(format!("Device err: {}", e)))?;

                // DPI Mapping Alignment: Explicitly supply both logical dimensions (self.width) and the
                // device_pixel_ratio to avoid projection matrices compressing geometry into the top-left corner.
                let backend = WgpuBackend::new(
                    device.clone(),
                    queue.clone(),
                    self.width,
                    self.height,
                    dpr as f32,
                )
                .await;

                let s = Rc::new(RenderState {
                    surface,
                    adapter,
                    device,
                    queue,
                    text_canvas,
                    backend: RefCell::new(backend),
                });
                RENDER_CACHE.with(|c| c.borrow_mut().insert(canvas_id.to_string(), s.clone()));
                s
            };

            state.text_canvas.set_width(display_width);
            state.text_canvas.set_height(display_height);

            let caps = state.surface.get_capabilities(&state.adapter);

            // Strict Pipeline Target Lock: Enforce a hardcoded Rgba8Unorm format constraint.
            // Do not use the dynamic browser format fallback (caps.formats[0]) because the core
            // wgpu.rs pipelines are heavily optimized and pre-compiled against Rgba8Unorm.
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Rgba8Unorm,
                width: display_width,
                height: display_height,
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            state.surface.configure(&state.device, &config);

            let surface_texture = match state.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(tex)
                | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
                other => {
                    return Err(ChartonError::Render(format!(
                        "Surface texture error: {:?}",
                        other
                    )));
                }
            };

            let view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Borrow the fully warm and cached pipeline backend instance
            let mut backend = state.backend.borrow_mut();

            // Render primitive geometry directly to the GPU core buffer
            let text_ledger = self.render_primitive_only(&mut backend, &view).await?;
            surface_texture.present();

            let ctx = state
                .text_canvas
                .get_context("2d")
                .map_err(|e| ChartonError::Render(format!("Could not get 2D context: {:?}", e)))?
                .ok_or_else(|| ChartonError::Render("2D context unavailable".into()))?
                .dyn_into::<CanvasRenderingContext2d>()
                .map_err(|_| ChartonError::Render("Failed to cast 2D context".into()))?;

            ctx.clear_rect(0.0, 0.0, display_width as f64, display_height as f64);
            ctx.save();
            let _ = ctx.scale(dpr, dpr);

            // Execute the native high-performance Canvas2D HTML text rendering pass
            for config in text_ledger {
                let font = format!(
                    "{} {}px {}",
                    config.font_weight, config.font_size, config.font_family
                );
                ctx.set_font(&font);

                let color_str = config.color.to_css_string();
                #[allow(deprecated)]
                ctx.set_fill_style(&color_str.into());

                match config.text_anchor.as_str() {
                    "start" | "left" => ctx.set_text_align("left"),
                    "end" | "right" => ctx.set_text_align("right"),
                    _ => ctx.set_text_align("center"),
                }

                match config.dominant_baseline.as_str() {
                    "hanging" | "top" => ctx.set_text_baseline("top"),
                    "alphabetic" | "bottom" => ctx.set_text_baseline("bottom"),
                    _ => ctx.set_text_baseline("middle"),
                }

                ctx.fill_text(&config.text, config.x as f64, config.y as f64)
                    .map_err(|e| ChartonError::Render(format!("fill_text failed: {:?}", e)))?;
            }

            ctx.restore();

            Ok(())
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = canvas_id;
            Err(ChartonError::Render(
                "render_to_canvas is only supported on WebAssembly platforms".into(),
            ))
        }
    }
}
