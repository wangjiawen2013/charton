use crate::coordinate::{CoordinateTrait, CoordSystem, Rect};
use crate::chart::Chart;
use crate::encode::Channel;
use crate::core::layer::Layer;
use crate::core::guide::{GuideSpec, LegendPosition};
use crate::core::context::SharedRenderingContext;
use crate::core::aesthetics::GlobalAesthetics;
use crate::scale::{Scale, ScaleDomain, Expansion, create_scale, mapper::VisualMapper};
use crate::core::aesthetics::AestheticMapping;
use crate::theme::Theme;
use crate::visual::color::{ColorMap, ColorPalette};
use crate::error::ChartonError;
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
/// It manages:
/// 1. The list of plot layers ([Layer]).
/// 2. The global coordinate system ([CoordSystem]).
/// 3. Scale Overrides (User-defined domains, expansions, and color schemes).
/// 4. The visual [Theme].
#[derive(Clone)]
pub struct LayeredChart {
    // --- Physical Dimensions ---
    pub(crate) width: u32,
    pub(crate) height: u32,

    // --- Layout Margins (Proportions 0.0 to 1.0) ---
    pub(crate) left_margin: f32,
    pub(crate) right_margin: f32,
    pub(crate) top_margin: f32,
    pub(crate) bottom_margin: f32,

    // --- Aesthetic Styling ---
    pub(crate) theme: Theme,
    pub(crate) title: Option<String>,

    // --- Chart Components ---
    /// Using Arc (Atomic Reference Counter) allows the chart to be cloned 
    /// cheaply without deep-copying the underlying data layers.
    pub(crate) layers: Vec<Arc<dyn Layer>>,
    pub(crate) coord_system: CoordSystem,

    // --- Axis & Scale Overrides (The "Brain") ---
    // These fields store explicit user intents that override automatic data inference.
    pub(crate) x_domain: Option<ScaleDomain>,
    pub(crate) x_label: Option<String>,
    pub(crate) x_expand: Option<Expansion>,

    pub(crate) y_domain: Option<ScaleDomain>,
    pub(crate) y_label: Option<String>,
    pub(crate) y_expand: Option<Expansion>,

    /// Override the theme's default color map for continuous data.
    pub(crate) color_map_override: Option<ColorMap>,
    /// Override the theme's default palette for categorical data.
    pub(crate) palette_override: Option<ColorPalette>,
    pub(crate) color_domain: Option<ScaleDomain>,
    pub(crate) color_expand: Option<Expansion>,

    pub(crate) shape_domain: Option<ScaleDomain>,
    pub(crate) shape_expand: Option<Expansion>,

    pub(crate) size_domain: Option<ScaleDomain>,
    pub(crate) size_expand: Option<Expansion>,

    pub(crate) flipped: bool,

    // --- Legend Logic ---
    pub(crate) legend_title: Option<String>,
    pub(crate) legend_position: LegendPosition,
    pub(crate) legend_margin: f32,
}

impl LayeredChart {
    pub fn new() -> Self {
        Self {
            width: 500,
            height: 400,

            left_margin: 0.06,
            right_margin: 0.03,
            top_margin: 0.10,
            bottom_margin: 0.08,

            theme: Theme::default(),
            title: None,

            layers: Vec::new(),
            coord_system: CoordSystem::default(),

            // Initializing all overrides as None (defer to automatic inference)
            x_domain: None,
            x_label: None,
            x_expand: None,

            y_domain: None,
            y_label: None,
            y_expand: None,

            color_map_override: None,
            palette_override: None,
            color_domain: None,
            color_expand: None,

            shape_domain: None,
            shape_expand: None,
            size_domain: None,
            size_expand: None,

            flipped: false,

            legend_title: None,
            legend_position: LegendPosition::Right,
            legend_margin: 15.0,
        }
    }

    // --- Physical Dimensions ---

    /// Convenience method to set both width and height.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    // --- Layout Margins ---

    pub fn with_left_margin(mut self, margin: f32) -> Self {
        self.left_margin = margin;
        self
    }

    pub fn with_right_margin(mut self, margin: f32) -> Self {
        self.right_margin = margin;
        self
    }

    pub fn with_top_margin(mut self, margin: f32) -> Self {
        self.top_margin = margin;
        self
    }

    pub fn with_bottom_margin(mut self, margin: f32) -> Self {
        self.bottom_margin = margin;
        self
    }

    /// Convenience method to set all margins at once.
    pub fn with_margins(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.top_margin = top;
        self.right_margin = right;
        self.bottom_margin = bottom;
        self.left_margin = left;
        self
    }

    // --- Aesthetic Styling ---

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set a custom ColorMap for continuous color scales.
    pub fn with_color_continuous(mut self, map: ColorMap) -> Self {
        self.color_map_override = Some(map);
        self
    }

    /// Set a custom Palette for categorical color scales.
    pub fn with_color_discrete(mut self, palette: ColorPalette) -> Self {
        self.palette_override = Some(palette);
        self
    }

    /// Provides a closure to modify the existing theme fluently.
    pub fn configure_theme<F>(mut self, f: F) -> Self 
    where 
        F: FnOnce(Theme) -> Theme 
    {
        self.theme = f(self.theme);
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    // --- Axis Data & Scale Configuration ---

    /// Set the global X-axis domain, overriding automatic data range calculation.
    pub fn with_x_domain(mut self, min: f32, max: f32) -> Self {
        self.x_domain = Some(ScaleDomain::Continuous(min, max));
        self
    }

    /// Set the X-axis expansion (padding). 
    /// If None, the resolution logic will use Theme defaults or Scale-specific defaults.
    pub fn with_x_expand(mut self, expand: Expansion) -> Self {
        self.x_expand = Some(expand);
        self
    }

    /// Set the global Y-axis domain.
    pub fn with_y_domain(mut self, min: f32, max: f32) -> Self {
        self.y_domain = Some(ScaleDomain::Continuous(min, max));
        self
    }

    /// Set the Y-axis expansion (padding). 
    /// If None, the resolution logic will use Theme defaults or Scale-specific defaults.
    pub fn with_y_expand(mut self, expand: Expansion) -> Self {
        self.y_expand = Some(expand);
        self
    }

    pub fn with_x_label(mut self, label: impl Into<String>) -> Self {
        self.x_label = Some(label.into());
        self
    }

    pub fn with_y_label(mut self, label: impl Into<String>) -> Self {
        self.y_label = Some(label.into());
        self
    }

    pub fn coord_flip(mut self) -> Self {
        self.flipped = true;
        self
    }

    // --- Legend Logic ---

    pub fn with_legend_title(mut self, title: impl Into<String>) -> Self {
        self.legend_title = Some(title.into());
        self
    }

    pub fn with_legend_position(mut self, position: LegendPosition) -> Self {
        self.legend_position = position;
        self
    }

    pub fn with_legend_margin(mut self, margin: f32) -> Self {
        self.legend_margin = margin;
        self
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
    pub fn resolve_scale_spec(&self, channel: Channel) -> Result<Option<ResolvedSpec>, ChartonError> {
        // --- Accumulators for Data Inference ---
        let mut inferred_field: Option<String> = None;
        let mut inferred_type: Option<Scale> = None;
        
        // Domain accumulators
        let mut cont_min = f32::INFINITY;
        let mut cont_max = f32::NEG_INFINITY;
        let mut all_labels: Vec<String> = Vec::new();
        let mut temp_start: Option<time::OffsetDateTime> = None;
        let mut temp_end: Option<time::OffsetDateTime> = None;

        // Expansion accumulators (Finding the "Maximum Room" requested by layers)
        let mut max_mult = (0.0f32, 0.0f32);
        let mut max_add = (0.0f32, 0.0f32);
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
                        if !all_labels.contains(&label) { all_labels.push(label); }
                    }
                }
                ScaleDomain::Temporal(start, end) => {
                    temp_start = Some(temp_start.map_or(start, |s| s.min(start)));
                    temp_end = Some(temp_end.map_or(end, |e| e.max(end)));
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
            Channel::Color => (self.color_domain.clone(), self.legend_title.clone(), self.color_expand),
            Channel::Shape => (self.shape_domain.clone(), None, self.shape_expand),
            Channel::Size => (self.size_domain.clone(), None, self.size_expand),
        };

        // --- Step 3: Final Reconciliation ---
        
        // A. Resolve Scale Type
        let scale_type = match (&inferred_type, &manual_domain) {
            (Some(t), _) => t.clone(),
            (None, Some(d)) => match d {
                ScaleDomain::Discrete(_) => Scale::Discrete,
                ScaleDomain::Temporal(_, _) => Scale::Temporal,
                _ => Scale::Linear,
            },
            _ => return Ok(None), // No data and no override
        };

        // B. Resolve Field Label
        let field = manual_label.or(inferred_field).unwrap_or_else(|| format!("{:?}", channel));

        // C. Resolve Domain (Priority: Manual > Consolidated)
        let domain = if let Some(d) = manual_domain {
            d
        } else {
            match scale_type {
                Scale::Discrete => {
                    if all_labels.is_empty() { return Ok(None); }
                    ScaleDomain::Discrete(all_labels)
                }
                Scale::Temporal => {
                    match (temp_start, temp_end) {
                        (Some(s), Some(e)) => ScaleDomain::Temporal(s, e),
                        _ => return Ok(None),
                    }
                }
                _ => {
                    if cont_min.is_infinite() {
                        ScaleDomain::Continuous(0.0, 1.0)
                    } else {
                        // Zero-range Protection
                        let (mut min, mut max) = (cont_min, cont_max);
                        if (max - min).abs() < 1e-12 { min -= 0.5; max += 0.5; }
                        ScaleDomain::Continuous(min, max)
                    }
                }
            }
        };

        // D. Resolve Expansion (Priority: Manual > Consolidated Max > Type Defaults)
        let expand = if let Some(me) = manual_expand {
            me
        } else if has_expansion_info {
            Expansion { mult: max_mult, add: max_add }
        } else {
            match scale_type {
                Scale::Discrete => Expansion { mult: (0.0, 0.0), add: (0.4, 0.4) },
                _ => Expansion::default(), // Standard 5% padding
            }
        };

        Ok(Some(ResolvedSpec { field, scale_type, domain, expand }))
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
    /// ```
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df1 = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
    /// let df2 = df!["x" => [1, 2, 3], "y" => [5, 15, 25]]?;
    ///
    /// let base_layer = Chart::<MarkBar>::build(&df1)?
    ///     .mark_bar()
    ///     .encode(x("x"), y("y"))?;
    ///     
    /// let overlay_layer = Chart::<MarkLine>::build(&df2)?
    ///     .mark_line()
    ///     .encode(x("x"), y("y"))?;
    ///
    /// let chart = LayeredChart::new()
    ///     .add_layer(base_layer)
    ///     .add_layer(overlay_layer);
    /// ```
    pub fn add_layer<T: crate::mark::Mark + 'static>(mut self, layer: Chart<T>) -> Self
    where
        Chart<T>: Layer,
    {
        // Check if the layer has data before adding it
        if layer.data.df.height() > 0 {
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
    /// Using `Arc` for Traits ensures that scales and coordinates can be shared safely 
    /// across multiple layers and threads without expensive deep-copying of data.
    pub fn resolve_scene(&self) -> Result<(Arc<dyn CoordinateTrait>, Rect, GlobalAesthetics, Vec<GuideSpec>), ChartonError> {
        
        // --- STEP 1: RESOLVE GLOBAL AESTHETIC MAPPINGS ---
        // We resolve non-positional encodings (Color, Shape, Size) across all layers.
        // The `resolve_scale_spec` handles domain merging and expansion automatically.

        // 1a. Color Mapping
        let color_mapping = if let Some(spec) = self.resolve_scale_spec(Channel::Color)? {
            let mapper = VisualMapper::new_color_default(&spec.scale_type, &self.theme);
            let scale_impl = create_scale(&spec.scale_type, spec.domain, spec.expand, Some(mapper.clone()))?;
            Some(AestheticMapping {
                field: spec.field,
                scale_type: spec.scale_type,
                scale_impl, // This is now an Arc<dyn ScaleTrait>
                mapper,
            })
        } else { None };

        // 1b. Shape Mapping
        let shape_mapping = if let Some(spec) = self.resolve_scale_spec(Channel::Shape)? {
            let mapper = VisualMapper::new_shape_default();
            let scale_impl = create_scale(&spec.scale_type, spec.domain, spec.expand, Some(mapper.clone()))?;
            Some(AestheticMapping {
                field: spec.field,
                scale_type: spec.scale_type,
                scale_impl,
                mapper,
            })
        } else { None };

        // 1c. Size Mapping
        let size_mapping = if let Some(spec) = self.resolve_scale_spec(Channel::Size)? {
            let mapper = VisualMapper::new_size_default(2.0, 9.0);
            let scale_impl = create_scale(&spec.scale_type, spec.domain, spec.expand, Some(mapper.clone()))?;
            Some(AestheticMapping {
                field: spec.field,
                scale_type: spec.scale_type,
                scale_impl,
                mapper,
            })
        } else { None };

        // Wrap mappings into GlobalAesthetics for unified access.
        let aesthetics = GlobalAesthetics::new(color_mapping, shape_mapping, size_mapping);

        // --- STEP 2: RESOLVE COORDINATE SCALES (X & Y) ---
        // Position scales map data to the [0, 1] normalized range.
        // We expect a valid spec for X/Y, falling back to a default unit scale if empty.
        let x_spec = self.resolve_scale_spec(Channel::X)?.unwrap();
        let y_spec = self.resolve_scale_spec(Channel::Y)?.unwrap();

        let x_scale = create_scale(&x_spec.scale_type, x_spec.domain, x_spec.expand, None)?;
        let y_scale = create_scale(&y_spec.scale_type, y_spec.domain, y_spec.expand, None)?;

        // Construct the coordinate system as an Arc to allow shared access during rendering.
        let final_coord: Arc<dyn CoordinateTrait> = match self.coord_system {
            CoordSystem::Cartesian2D => Arc::new(crate::coordinate::cartesian::Cartesian2D::new(
                x_scale, 
                y_scale, 
                self.flipped
            )),
            CoordSystem::Polar => todo!("Polar coordinate resolution is planned for the next release"),
        };

        // --- STEP 3: GUIDE GENERATION ---
        // Group aesthetic mappings by field name to create unified legends/colorbars.
        let guide_specs = crate::core::guide::GuideManager::collect_guides(&aesthetics);

        // --- STEP 4: PHYSICAL MEASUREMENT (LAYOUT ENGINE) ---
        let w = self.width as f32;
        let h = self.height as f32;

        // A. Theoretical Maximum Plot Area (Total size minus static chart margins).
        let initial_plot_w = w * (1.0 - self.left_margin - self.right_margin);
        let initial_plot_h = h * (1.0 - self.top_margin - self.bottom_margin);

        // B. Measure Legend Constraints based on guide specifications.
        let legend_box = crate::core::layout::LayoutEngine::calculate_legend_constraints(
            &guide_specs,
            self.legend_position,
            w, h,
            initial_plot_w,
            initial_plot_h,
            self.legend_margin,
            &self.theme
        );

        // C. Measure Axis Constraints using a temporary SharedRenderingContext.
        let temp_panel = Rect::new(
            (self.left_margin * w) + legend_box.left,
            (self.top_margin * h) + legend_box.top,
            (initial_plot_w - legend_box.left - legend_box.right).max(10.0),
            (initial_plot_h - legend_box.top - legend_box.bottom).max(10.0)
        );

        let temp_ctx = SharedRenderingContext::new(
            &*final_coord, // Deref Arc to &dyn CoordinateTrait
            temp_panel,
            self.legend_position,
            self.legend_margin,
            &aesthetics
        );

        let axis_box = crate::core::layout::LayoutEngine::calculate_axis_constraints(
            &temp_ctx,
            &self.theme,
            &x_spec.field,
            &y_spec.field
        );

        // --- STEP 5: FINAL PANEL RESOLUTION & DEFENSE ---
        let final_left = (self.left_margin * w) + legend_box.left + axis_box.left;
        let final_right = (self.right_margin * w) + legend_box.right;
        let final_top = (self.top_margin * h) + legend_box.top;
        let final_bottom = (self.bottom_margin * h) + legend_box.bottom + axis_box.bottom;

        // Apply "Defense" rule: Ensure the panel never shrinks below the theme's minimum size.
        let plot_w = (w - final_left - final_right).max(self.theme.min_panel_size);
        let plot_h = (h - final_top - final_bottom).max(self.theme.min_panel_size);

        let panel = Rect::new(final_left, final_top, plot_w, plot_h);

        Ok((final_coord, panel, aesthetics, guide_specs))
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
        let center_x = self.width as f32 / 2.0;
        
        // 3. Vertical Positioning Logic:
        // Instead of a hardcoded '25.0', we calculate the available vertical space 
        // above the plot panel (panel.y). 
        // We place the text's baseline in the middle of this area.
        let title_area_height = panel.y;
        let font_size = self.theme.title_size;
        
        // Calculate the vertical midpoint. 
        // Note: Using 'dominant-baseline="middle"' allows us to use the exact midpoint as the Y coordinate.
        let center_y = title_area_height / 5.0;

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
            font_color.as_str(),
            title_text
        )?;

        Ok(())
    }

    /// Renders the entire layered chart to the provided SVG string.
    ///
    /// This implementation follows a strictly ordered pipeline:
    /// 1. **Resolution**: Consolidate scales, domains, and physical layout in one pass.
    /// 2. **Back-fill**: Synchronize individual layers with the resolved global scales.
    /// 3. **Draw**: Orchestrate the rendering of axes, data marks, and guides.
    pub fn render(&mut self, svg: &mut String) -> Result<(), ChartonError> {
        // 0. Guard: If no layers exist, there is nothing to visualize.
        if self.layers.is_empty() { 
            return Ok(()); 
        }

        // --- STEP 1: RESOLUTION PHASE ---
        // We call resolve_rendering_layout which performs the following:
        // - Aggregates data domains (color, shape, size, x, y) across all layers.
        // - Creates unified Scales and VisualMappers.
        // - Measures Legend and Axis constraints to determine the final data Panel (Rect).
        // - Generates GuideSpecs (legend instructions).
        let (coord_box, panel, aesthetics, guide_specs) = self.resolve_rendering_layout()?;

        // --- STEP 2: LAYER SYNCHRONIZATION (BACK-FILL) ---
        // To ensure "Visual Consistency," every layer must use the same scales and domains.
        // We update the layers with the global metadata resolved in Step 1.
        for layer in self.layers.iter_mut() {
            
            // 2a. Sync Color Metadata
            if let Some(ref mapping) = aesthetics.color {
                // mapping.scale_type is the Scale enum (Linear, Discrete, etc.)
                layer.set_scale_type("color", mapping.scale_type.clone());
                
                // Use get_domain_enum() to retrieve the full ScaleDomain (Categorical or Continuous)
                // mapping.scale_impl is the Box<dyn ScaleTrait>
                layer.set_domain("color", mapping.scale_impl.get_domain_enum());
            }

            // 2b. Sync Shape Metadata
            if let Some(ref mapping) = aesthetics.shape {
                layer.set_scale_type("shape", mapping.scale_type.clone());
                layer.set_domain("shape", mapping.scale_impl.get_domain_enum());
            }

            // 2c. Sync Size Metadata
            if let Some(ref mapping) = aesthetics.size {
                layer.set_scale_type("size", mapping.scale_type.clone());
                layer.set_domain("size", mapping.scale_impl.get_domain_enum());
            }
        }

        // --- STEP 3: CONTEXT INITIALIZATION ---
        // Construct the SharedRenderingContext, which acts as the "Source of Truth" 
        // for all downstream renderers (Axes, Marks, Legends).
        let context = SharedRenderingContext::new(
            &*coord_box, 
            panel,
            self.legend_position,
            self.legend_margin,
            &aesthetics
        );

        // --- STEP 4: DRAWING PHASE ---

        // 4a. Render Chart Title
        // Positions the title based on the overall canvas and resolved panel.
        self.render_title(svg, &context.panel)?;

        // 4b. Render Axes (X and Y)
        // Determine if axes are appropriate based on theme settings and layer types.
        let should_render_axes = if !self.theme.show_axes {
            false // Global theme override
        } else {
            // Only render axes if at least one layer requires them (e.g., bypass for Pie charts).
            self.layers.iter().any(|layer| layer.requires_axes())
        };

        if should_render_axes {
            // Retrieve labels (aggregated from layers or defaulted from chart settings).
            let x_label = self.resolve_x_label();
            let y_label = self.resolve_y_label();

            crate::render::axis_renderer::render_axes(
                svg, 
                &self.theme, 
                &context, 
                &x_label, 
                &y_label
            )?;
        }

        // 4c. Render Marks (Data Geometries)
        // Each layer iterates over its data and renders its specific geometry (points, lines, etc.)
        // using the coordinate system and visual mappers provided by the context.
        let mut backend = crate::render::backend::svg::SvgBackend::new(svg, Some(&context.panel));
        for layer in &self.layers {
            layer.render_marks(&mut backend, &context)?;
        }

        // 4d. Render Unified Legends & Guides
        // Uses the GuideSpecs generated during resolution to draw legend blocks
        // in the margins calculated by the LayoutEngine.
        crate::render::legend_renderer::LegendRenderer::render_legend(
            svg, 
            &guide_specs, 
            &self.theme, 
            &context
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
            self.theme.background_color.as_str()
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
    /// ```
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
    /// ```rust
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
    /// let chart = Chart::build(&df)?
    ///     .mark_point()
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
    /// ```rust
    /// use charton::prelude::*;
    /// use polars::prelude::*;
    ///
    /// let df = df!["x" => [1, 2, 3], "y" => [10, 20, 30]]?;
    /// let chart = Chart::build(&df)?
    ///     .mark_point()
    ///     .encode(x("x"), y("y"))?;
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
