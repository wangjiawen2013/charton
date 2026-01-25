use crate::coordinate::{CoordinateTrait, CoordSystem, Rect};
use crate::chart::Chart;
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
    pub(crate) layers: Vec<Box<dyn Layer>>,
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

    /// Consolidates X-axis data domains across all layers.
    /// 
    /// Ensures all layers use a compatible Scale type and merges their data 
    /// into a single global domain (Continuous or Categorical).
    fn get_x_domain_from_layers(&self) -> Result<Option<(Scale, ScaleDomain)>, ChartonError> {
        let mut resolved_type: Option<Scale> = None;
        
        // Variables to track continuous bounds
        let mut global_min = f32::INFINITY;
        let mut global_max = f32::NEG_INFINITY;
        
        // Vector to track unique categorical labels
        let mut all_labels: Vec<String> = Vec::new();

        for (i, layer) in self.layers.iter().enumerate() {
            // Step 1: Identify scale type for X in this layer
            let current_type = match layer.get_x_scale_type_from_layer() {
                Some(t) => t,
                None => continue, // Skip layers without X encoding
            };

            // Step 2: Validate type consistency
            if let Some(ref existing_type) = resolved_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "X-axis scale type conflict at layer {}: Expected {:?}, found {:?}",
                        i, existing_type, current_type
                    )));
                }
            } else {
                resolved_type = Some(current_type);
            }

            // Step 3: Extract and merge domain data
            match resolved_type.as_ref().unwrap() {
                Scale::Discrete => {
                    if let Some(labels) = layer.get_x_discrete_tick_labels()? {
                        for label in labels {
                            if !all_labels.contains(&label) {
                                all_labels.push(label);
                            }
                        }
                    }
                }
                _ => {
                    let (min, max) = layer.get_x_continuous_bounds()?;
                    global_min = global_min.min(min);
                    global_max = global_max.max(max);
                }
            }
        }

        // Step 4: Finalize the domain
        match resolved_type {
            Some(stype) => {
                let domain = match stype {
                    Scale::Discrete => {
                        if all_labels.is_empty() { return Ok(None); }
                        ScaleDomain::Categorical(all_labels)
                    },
                    _ => {
                        if global_min.is_infinite() {
                            ScaleDomain::Continuous(0.0, 1.0)
                        } else {
                            // Handle edge case where min == max (e.g., single data point)
                            if (global_max - global_min).abs() < 1e-12 {
                                global_min -= 0.5;
                                global_max += 0.5;
                            }
                            ScaleDomain::Continuous(global_min, global_max)
                        }
                    }
                };
                Ok(Some((stype, domain)))
            },
            None => Ok(None),
        }
    }

    // Get the x-axis label from layers
    fn resolve_x_label(&self) -> String {
        // First check if we have an explicit label set on the chart
        if let Some(ref label) = self.x_label {
            return label.clone();
        }

        // Try to take the label from the first layer that has a label or field name defined
        for layer in &self.layers {
            // Use if let in case charts that don't have x encoding (like pie charts)
            if let Some(field) = layer.get_x_encoding_field() {
                return field;
            }
        }

        // Default fallback
        "X".to_string()
    }

    /// Consolidates Y-axis data domains across all layers.
    /// 
    /// Follows the same consolidation logic as X and Color channels:
    /// 1. Validates scale type consistency (Linear vs Discrete).
    /// 2. Aggregates min/max for continuous scales or unique labels for discrete ones.
    fn get_y_domain_from_layers(&self) -> Result<Option<(Scale, ScaleDomain)>, ChartonError> {
        let mut resolved_type: Option<Scale> = None;
        
        // Variables to track continuous bounds
        let mut global_min = f32::INFINITY;
        let mut global_max = f32::NEG_INFINITY;
        
        // Vector to track unique categorical labels
        let mut all_labels: Vec<String> = Vec::new();

        for (i, layer) in self.layers.iter().enumerate() {
            // Step 1: Check Y encoding and scale type
            let current_type = match layer.get_y_scale_type_from_layer() {
                Some(t) => t,
                None => continue, // Skip layers without Y encoding
            };

            // Step 2: Ensure Y scale consistency across layers
            if let Some(ref existing_type) = resolved_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "Y-axis scale type conflict at layer {}: Expected {:?}, found {:?}",
                        i, existing_type, current_type
                    )));
                }
            } else {
                resolved_type = Some(current_type);
            }

            // Step 3: Domain aggregation
            match resolved_type.as_ref().unwrap() {
                Scale::Discrete => {
                    if let Some(labels) = layer.get_y_discrete_tick_labels()? {
                        for label in labels {
                            if !all_labels.contains(&label) {
                                all_labels.push(label);
                            }
                        }
                    }
                }
                _ => {
                    let (min, max) = layer.get_y_continuous_bounds()?;
                    global_min = global_min.min(min);
                    global_max = global_max.max(max);
                }
            }
        }

        // Step 4: Construct final Y domain
        match resolved_type {
            Some(stype) => {
                let domain = match stype {
                    Scale::Discrete => {
                        if all_labels.is_empty() { return Ok(None); }
                        ScaleDomain::Categorical(all_labels)
                    },
                    _ => {
                        if global_min.is_infinite() {
                            ScaleDomain::Continuous(0.0, 1.0)
                        } else {
                            // Apply offset if min/max are identical to ensure a valid range
                            if (global_max - global_min).abs() < 1e-12 {
                                global_min -= 0.5;
                                global_max += 0.5;
                            }
                            ScaleDomain::Continuous(global_min, global_max)
                        }
                    }
                };
                Ok(Some((stype, domain)))
            },
            None => Ok(None),
        }
    }

    // Get the y-axis label from layers
    fn resolve_y_label(&self) -> String {
        // First check if we have an explicit label set on the chart
        if let Some(ref label) = self.y_label {
            return label.clone();
        }

        // Try to get label from the first layers
        for layer in &self.layers {
            // Use if let in case charts that don't have y encoding (like pie charts)
            if let Some(field) = layer.get_y_encoding_field() {
                return field;
            }
        }

        // Default fallback
        "Y".to_string()
    }

    /// Resolves the unified color encoding by aggregating metadata and data domains across all layers.
    /// 
    /// This method ensures:
    /// 1. **Field Consistency**: Identifies the source data column (e.g., "price").
    /// 2. **Type Validation**: Checks that all layers agree on the Scale type (e.g., all Linear or all Discrete).
    /// 3. **Domain Merging**:
    ///    - For Continuous: Calculates global [min, max], handling cases where min == max.
    ///    - For Categorical: Collects all unique strings in order of appearance.
    fn resolve_color_encoding(&self) -> Result<Option<(String, Scale, ScaleDomain)>, ChartonError> {
        let mut resolved_field: Option<String> = None;
        let mut resolved_type: Option<Scale> = None;
        
        let mut global_min = f32::INFINITY;
        let mut global_max = f32::NEG_INFINITY;
        let mut all_labels: Vec<String> = Vec::new();

        for (i, layer) in self.layers.iter().enumerate() {
            // Step 1: Check if this layer has a color mapping defined
            let (field, current_type) = match (layer.get_color_encoding_field(), layer.get_color_scale_type_from_layer()) {
                (Some(f), Some(t)) => (f, t),
                _ => continue, 
            };

            // Step 2: Establish or validate the "Source of Truth" for this chart's color channel
            if let Some(ref existing_type) = resolved_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "Color scale type conflict: Layer 0 is {:?}, but layer {} is {:?}",
                        existing_type, i, current_type
                    )));
                }
            } else {
                resolved_field = Some(field);
                resolved_type = Some(current_type);
            }

            // Step 3: Aggregate data based on the resolved scale type
            match resolved_type.as_ref().unwrap() {
                Scale::Discrete => {
                    if let Some(labels) = layer.get_color_discrete_labels()? {
                        for label in labels {
                            if !all_labels.contains(&label) { all_labels.push(label); }
                        }
                    }
                }
                _ => {
                    if let Some((min, max)) = layer.get_color_continuous_bounds()? {
                        global_min = global_min.min(min);
                        global_max = global_max.max(max);
                    }
                }
            }
        }

        // Step 4: Construct the final ScaleDomain
        if let (Some(field), Some(stype)) = (resolved_field, resolved_type) {
            let domain = match stype {
                Scale::Discrete => {
                    if all_labels.is_empty() { return Ok(None); }
                    ScaleDomain::Categorical(all_labels)
                },
                _ => {
                    if global_min.is_infinite() {
                        ScaleDomain::Continuous(0.0, 1.0)
                    } else {
                        // Protect against zero-range domains to avoid division by zero during normalization
                        let (mut final_min, mut final_max) = (global_min, global_max);
                        if (final_max - final_min).abs() < 1e-12 {
                            final_min -= 0.5;
                            final_max += 0.5;
                        }
                        ScaleDomain::Continuous(final_min, final_max)
                    }
                }
            };
            Ok(Some((field, stype, domain)))
        } else {
            Ok(None)
        }
    }

    /// Resolves the unified size encoding across all layers.
    ///
    /// Size scales are typically used for continuous variables (e.g., mapping "population" to radius).
    /// This method aggregates the numeric bounds and ensures a non-zero range for normalization.
    fn resolve_size_encoding(&self) -> Result<Option<(String, Scale, ScaleDomain)>, ChartonError> {
        let mut resolved_field: Option<String> = None;
        let mut resolved_type: Option<Scale> = None;
        let mut global_min = f32::INFINITY;
        let mut global_max = f32::NEG_INFINITY;

        for layer in &self.layers {
            if let Some(field) = layer.get_size_encoding_field() {
                if resolved_field.is_none() {
                    resolved_field = Some(field);
                    // Default to Linear size scaling if not explicitly specified
                    resolved_type = Some(layer.get_size_scale_type_from_layer().unwrap_or(Scale::Linear));
                }

                if let Some((min, max)) = layer.get_size_continuous_bounds()? {
                    global_min = global_min.min(min);
                    global_max = global_max.max(max);
                }
            }
        }

        if let (Some(field), Some(stype)) = (resolved_field, resolved_type) {
            let domain = if global_min.is_infinite() {
                ScaleDomain::Continuous(0.0, 1.0)
            } else {
                let (mut final_min, mut final_max) = (global_min, global_max);
                // Zero-range protection: ensures points are visible even if all data values are identical
                if (final_max - final_min).abs() < 1e-12 {
                    final_min -= 0.5;
                    final_max += 0.5;
                }
                ScaleDomain::Continuous(final_min, final_max)
            };
            Ok(Some((field, stype, domain)))
        } else {
            Ok(None)
        }
    }

    /// Resolves the unified shape encoding across all layers.
    ///
    /// Shape encodings are strictly categorical (Discrete). This method collects all 
    /// unique category labels to ensure the Shape Palette maps them consistently.
    fn resolve_shape_encoding(&self) -> Result<Option<(String, Scale, ScaleDomain)>, ChartonError> {
        let mut resolved_field: Option<String> = None;
        let mut all_labels: Vec<String> = Vec::new();

        for layer in &self.layers {
            if let Some(field) = layer.get_shape_encoding_field() {
                if resolved_field.is_none() {
                    resolved_field = Some(field);
                }

                if let Some(labels) = layer.get_shape_discrete_labels()? {
                    for label in labels {
                        if !all_labels.contains(&label) {
                            all_labels.push(label);
                        }
                    }
                }
            }
        }

        if let Some(field) = resolved_field {
            if all_labels.is_empty() { return Ok(None); }
            // Shape is inherently Discrete in this plotting system
            Ok(Some((field, Scale::Discrete, ScaleDomain::Categorical(all_labels))))
        } else {
            Ok(None)
        }
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
            self.layers.push(Box::new(layer));
        }
        // If layer is empty, silently ignore it
        self
    }

    /// Resolves the final rendering layout and global aesthetic scales by consolidating metadata.
    /// 
    /// This implementation follows an "Industrial Defense" layout pipeline:
    /// 1. **Consolidate Aesthetic Mappings**: Aggregates color, shape, and size scales from layers.
    /// 2. **Coordinate Resolution**: Initializes X and Y scales based on data domains.
    /// 3. **Measurement Phase**: Utilizes the LayoutEngine to calculate physical pixel requirements.
    /// 4. **Defense Mechanism**: Ensures a minimum panel size to prevent chart "collapse".
    fn resolve_rendering_layout(&self) -> Result<(Box<dyn CoordinateTrait>, Rect, GlobalAesthetics, Vec<GuideSpec>), ChartonError> {
        // --- STEP 1: CONSOLIDATE AESTHETIC MAPPINGS ---
        // We resolve encodings across all layers to ensure visual consistency.
        // Each resolver returns (FieldName, ScaleType, ScaleDomain).

        // 1a. Resolve Color Mapping
        // This handles both Continuous (Linear/Log) and Discrete color scales.
        let color_mapping = if let Some((field, scale_type, domain)) = self.resolve_color_encoding()? {
            let scale_impl = create_scale(&scale_type, domain, self.color_expand.expect("composite.rs line about 656"))?;
            let mapper = VisualMapper::new_color_default(&scale_type, &self.theme);
            Some(AestheticMapping {
                field,
                scale_type,
                scale_impl,
                mapper,
            })
        } else { None };

        // 1b. Resolve Shape Mapping
        // Shapes are strictly Categorical/Discrete in this system.
        let shape_mapping = if let Some((field, scale_type, domain)) = self.resolve_shape_encoding()? {
            let scale_impl = create_scale(&scale_type, domain, self.shape_expand.expect("Composite.rs line about 680"))?;
            let mapper = VisualMapper::new_shape_default();
            Some(AestheticMapping {
                field,
                scale_type,
                scale_impl,
                mapper,
            })
        } else { None };

        // 1c. Resolve Size Mapping
        // Size usually maps to a Linear scale (area or radius).
        let size_mapping = if let Some((field, scale_type, domain)) = self.resolve_size_encoding()? {
            let scale_impl = create_scale(&scale_type, domain, self.size_expand.expect("composite.rs line about 696"))?;
            // Radius range (2.0 to 9.0) provides clear visual distinction in legends.
            let mapper = VisualMapper::new_size_default(2.0, 9.0);
            Some(AestheticMapping {
                field,
                scale_type,
                scale_impl,
                mapper,
            })
        } else { None };

        // Initialize GlobalAesthetics: the single source of truth for non-positional scales.
        let aesthetics = GlobalAesthetics::new(color_mapping, shape_mapping, size_mapping);

        // --- STEP 2: RESOLVE COORDINATE SCALES (X & Y) ---
        // Position scales map data to the [0, 1] normalized range of the plot panel.

        // 2a. Resolve X-Axis
        let x_scale = if let Some((stype, domain)) = self.get_x_domain_from_layers()? {
            // Apply user-defined domain overrides if present.
            let domain = self.x_domain.clone().unwrap_or(domain);
            create_scale(&stype, domain, self.x_expand.expect("composite.rs line about 724"))?
        } else {
            // Fallback to a unit scale if no X data is found.
            create_scale(&Scale::Linear, ScaleDomain::Continuous(0.0, 1.0), self.x_expand.expect("composite.rs line about 740"))?
        };

        // 2b. Resolve Y-Axis
        let y_scale = if let Some((stype, domain)) = self.get_y_domain_from_layers()? {
            let domain = self.y_domain.clone().unwrap_or(domain);
            create_scale(&stype, domain, self.y_expand.expect("composite.rs line about 756"))?
        } else {
            create_scale(&Scale::Linear, ScaleDomain::Continuous(0.0, 1.0), self.y_expand.expect("composite.rs line about 772"))?
        };

        // Construct the coordinate system (Cartesian is the current standard).
        let final_coord: Box<dyn CoordinateTrait> = match self.coord_system {
            CoordSystem::Cartesian2D => Box::new(crate::coordinate::cartesian::Cartesian2D::new(
                x_scale, 
                y_scale, 
                self.flipped
            )),
            CoordSystem::Polar => todo!("Polar coordinate resolution not yet implemented"),
        };

        // --- STEP 3: GENERATE GUIDE SPECIFICATIONS ---
        // We group aesthetic mappings by field name to create unified legends/colorbars.
        let guide_specs = crate::core::guide::GuideManager::collect_guides(&aesthetics);

        // --- STEP 4: MEASUREMENT PHASE (THE LAYOUT ENGINE) ---
        // We calculate how much space legends and axes take to determine the remaining data panel size.
        let w = self.width as f32;
        let h = self.height as f32;

        // A. Theoretical Maximum Plot Area (Total size minus static chart margins).
        let initial_plot_w = w * (1.0 - self.left_margin - self.right_margin);
        let initial_plot_h = h * (1.0 - self.top_margin - self.bottom_margin);

        // B. Measure Legend Constraints.
        let legend_box = crate::core::layout::LayoutEngine::calculate_legend_constraints(
            &guide_specs,
            self.legend_position,
            w, h,
            initial_plot_w,
            initial_plot_h,
            self.legend_margin,
            &self.theme
        );

        // C. Measure Axis Constraints.
        // To measure axis labels accurately, we create a temporary context using an estimated panel.
        let temp_panel = Rect::new(
            (self.left_margin * w) + legend_box.left,
            (self.top_margin * h) + legend_box.top,
            (initial_plot_w - legend_box.left - legend_box.right).max(10.0),
            (initial_plot_h - legend_box.top - legend_box.bottom).max(10.0)
        );

        let temp_ctx = SharedRenderingContext::new(
            &*final_coord,
            temp_panel,
            self.legend_position,
            self.legend_margin,
            &aesthetics
        );

        let axis_box = crate::core::layout::LayoutEngine::calculate_axis_constraints(
            &temp_ctx,
            &self.theme,
            &self.resolve_x_label(),
            &self.resolve_y_label()
        );

        // --- STEP 5: FINAL PANEL RESOLUTION & DEFENSE ---
        // We subtract all measured components (Legends + Axes) from the chart total.
        let final_left = (self.left_margin * w) + legend_box.left + axis_box.left;
        let final_right = (self.right_margin * w) + legend_box.right;
        let final_top = (self.top_margin * h) + legend_box.top;
        let final_bottom = (self.bottom_margin * h) + legend_box.bottom + axis_box.bottom;

        // Apply the "Defense" rule: Ensure the panel never shrinks below the theme's minimum size.
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
