use crate::coordinate::{CoordinateTrait, CoordSystem, Rect};
use crate::chart::Chart;
use crate::core::layer::Layer;
use crate::core::legend::{LegendSpec, LegendPosition};
use crate::core::context::SharedRenderingContext;
use crate::scale::{Scale, ScaleDomain, create_scale, mapper::VisualMapper};
use crate::encode::aesthetics::GlobalAesthetics;
use crate::theme::Theme;
use crate::error::ChartonError;
use std::fmt::Write;

/// `LayeredChart` is the core structure for building multi-layer visualizations.
///
/// It manages the overall layout, coordinate systems, and data-driven properties.
/// Visual styling is delegated to the [Theme] struct.
#[derive(Clone)]
pub struct LayeredChart {
    // --- Physical Dimensions ---
    width: u32,
    height: u32,

    // --- Layout Margins (Proportions 0.0 to 1.0) ---
    left_margin: f64,
    right_margin: f64,
    top_margin: f64,
    bottom_margin: f64,

    // --- Aesthetic Styling ---
    theme: Theme,
    title: Option<String>,

    // --- Chart Components ---
    layers: Vec<Box<dyn Layer>>,
    coord_system: CoordSystem,

    // --- Axis Data & Scale Configuration ---
    x_domain_min: Option<f64>,
    x_domain_max: Option<f64>,
    x_label: Option<String>,

    y_domain_min: Option<f64>,
    y_domain_max: Option<f64>,
    y_label: Option<String>,

    flipped: bool,

    // --- Legend Logic ---
    legend_enabled: Option<bool>,
    legend_title: Option<String>,
    pub(crate) legend_position: LegendPosition,
    pub(crate) legend_margin: f64,
}

impl Default for LayeredChart {
    fn default() -> Self {
        Self::new()
    }
}

impl LayeredChart {
    /// Initializes a new `LayeredChart` with default settings.
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

            x_domain_min: None,
            x_domain_max: None,
            x_label: None,
            y_domain_min: None,
            y_domain_max: None,
            y_label: None,

            flipped: false,

            legend_enabled: None,
            legend_title: None,
            legend_position: LegendPosition::Right,
            legend_margin: 15.0,
        }
    }

    // --- Physical Dimensions ---

    /// Convenience method to set both width and height.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    // --- Layout Margins ---

    pub fn left_margin(mut self, margin: f64) -> Self {
        self.left_margin = margin;
        self
    }

    pub fn right_margin(mut self, margin: f64) -> Self {
        self.right_margin = margin;
        self
    }

    pub fn top_margin(mut self, margin: f64) -> Self {
        self.top_margin = margin;
        self
    }

    pub fn bottom_margin(mut self, margin: f64) -> Self {
        self.bottom_margin = margin;
        self
    }

    /// Convenience method to set all margins at once.
    pub fn margins(mut self, top: f64, right: f64, bottom: f64, left: f64) -> Self {
        self.top_margin = top;
        self.right_margin = right;
        self.bottom_margin = bottom;
        self.left_margin = left;
        self
    }

    // --- Aesthetic Styling ---

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
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

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    // --- Axis Data & Scale Configuration ---

    pub fn x_domain_min(mut self, min: f64) -> Self {
        self.x_domain_min = Some(min);
        self
    }

    pub fn x_domain_max(mut self, max: f64) -> Self {
        self.x_domain_max = Some(max);
        self
    }

    pub fn x_label(mut self, label: impl Into<String>) -> Self {
        self.x_label = Some(label.into());
        self
    }

    pub fn y_domain_min(mut self, min: f64) -> Self {
        self.y_domain_min = Some(min);
        self
    }

    pub fn y_domain_max(mut self, max: f64) -> Self {
        self.y_domain_max = Some(max);
        self
    }

    pub fn y_label(mut self, label: impl Into<String>) -> Self {
        self.y_label = Some(label.into());
        self
    }

    pub fn flipped(mut self, flipped: bool) -> Self {
        self.flipped = flipped;
        self
    }

    // --- Legend Logic ---

    pub fn legend_enabled(mut self, enabled: bool) -> Self {
        self.legend_enabled = Some(enabled);
        self
    }

    pub fn legend_title(mut self, title: impl Into<String>) -> Self {
        self.legend_title = Some(title.into());
        self
    }

    pub fn legend_position(mut self, position: LegendPosition) -> Self {
        self.legend_position = position;
        self
    }

    pub fn legend_margin(mut self, margin: f64) -> Self {
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
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;
        
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
    fn get_x_axis_label_from_layers(&self) -> String {
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
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;
        
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
    fn get_y_axis_label_from_layers(&self) -> String {
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

    /// Consolidates color data domains across all layers to ensure visual consistency.
    /// 
    /// This method performs two critical tasks:
    /// 1. **Type Validation**: It ensures that all layers using the color channel share the 
    ///    same Scale type (e.g., you cannot mix a Continuous 'Linear' scale with a 
    ///    Discrete 'Ordinal' scale in the same chart).
    /// 2. **Domain Aggregation**: 
    ///    - For Continuous scales: It finds the global minimum and maximum across all layers.
    ///    - For Categorical scales: It collects all unique labels while preserving 
    ///      insertion order across layers.
    ///
    /// # Returns
    /// - `Ok(Some((Scale, ScaleDomain)))`: A unified Scale type and domain ready for Scale initialization.
    /// - `Ok(None)`: If no layers have color encodings defined.
    /// - `Err(ChartonError)`: If a type conflict is detected between layers.
    fn get_color_domain_from_layers(&self) -> Result<Option<(Scale, ScaleDomain)>, ChartonError> {
        let mut resolved_type: Option<Scale> = None;
        
        // Variables to track continuous bounds
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;
        
        // Vector to track unique categorical labels
        let mut all_labels: Vec<String> = Vec::new();

        for (i, layer) in self.layers.iter().enumerate() {
            // Step 1: Check if this layer has a color encoding and what its scale type is
            let current_type = match layer.get_color_scale_type_from_layer() {
                Some(t) => t,
                None => continue, // Skip layers that don't encode color
            };

            // Step 2: Ensure type consistency across the entire layered chart
            if let Some(ref existing_type) = resolved_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "Color scale type conflict at layer {}: Layer 0 is {:?}, but layer {} is {:?}",
                        i, existing_type, i, current_type
                    )));
                }
            } else {
                // This is the first layer with color; it sets the 'source of truth' for the chart
                resolved_type = Some(current_type);
            }

            // Step 3: Extract and merge domain data based on the resolved type
            match resolved_type.as_ref().unwrap() {
                Scale::Discrete => {
                    // Collect unique strings for categorical mapping
                    if let Some(labels) = layer.get_color_discrete_labels()? {
                        for label in labels {
                            if !all_labels.contains(&label) {
                                all_labels.push(label);
                            }
                        }
                    }
                }
                _ => {
                    // Update global min/max for continuous mapping (Linear, Log, etc.)
                    if let Some((min, max)) = layer.get_color_continuous_bounds()? {
                        global_min = global_min.min(min);
                        global_max = global_max.max(max);
                    }
                }
            }
        }

        // Step 4: Construct the final ScaleDomain based on the accumulated data
        match resolved_type {
            Some(stype) => {
                let domain = match stype {
                    Scale::Discrete => {
                        if all_labels.is_empty() { return Ok(None); }
                        ScaleDomain::Categorical(all_labels)
                    },
                    _ => {
                        // Handle cases where no valid numeric data was found despite having a scale type
                        if global_min.is_infinite() {
                            // Fallback to a unit range [0, 1] if data is missing or empty
                            ScaleDomain::Continuous(0.0, 1.0)
                        } else {
                            // Or apply optional user-defined overrides if they exist at the chart level
                            // (Assuming self.color_domain_min/max exist similar to x_domain_min)
                            ScaleDomain::Continuous(global_min, global_max)
                        }
                    }
                };
                Ok(Some((stype, domain)))
            },
            None => Ok(None),
        }
    }

    /// Consolidates shape data domains across all layers.
    ///
    /// # Returns
    /// - `Ok(Some(ScaleDomain::Categorical))`: Unified unique shape labels.
    /// - `Ok(None)`: If no shape encodings are defined.
    fn get_shape_domain_from_layers(&self) -> Result<Option<ScaleDomain>, ChartonError> {
        let mut resolved_type: Option<Scale> = None;
        let mut all_labels: Vec<String> = Vec::new();

        for (i, layer) in self.layers.iter().enumerate() {
            let current_type = match layer.get_shape_scale_type_from_layer() {
                Some(t) => t,
                None => continue,
            };

            // Type Validation (Ensuring Shape remains Discrete)
            if let Some(ref existing_type) = resolved_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "Shape scale type conflict at layer {}: expected {:?}, found {:?}",
                        i, existing_type, current_type
                    )));
                }
            } else {
                resolved_type = Some(current_type);
            }

            // Domain Aggregation
            if let Some(labels) = layer.get_shape_discrete_labels()? {
                for label in labels {
                    if !all_labels.contains(&label) {
                        all_labels.push(label);
                    }
                }
            }
        }

        match resolved_type {
            Some(_) => {
                if all_labels.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(ScaleDomain::Categorical(all_labels)))
                }
            },
            None => Ok(None),
        }
    }

    /// Consolidates size data domains across all layers.
    ///
    /// # Returns
    /// - `Ok(Some(ScaleDomain::Continuous))`: Unified numeric range for size mapping.
    /// - `Ok(None)`: If no size encodings are defined.
    fn get_size_domain_from_layers(&self) -> Result<Option<ScaleDomain>, ChartonError> {
        let mut resolved_type: Option<Scale> = None;
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;

        for (i, layer) in self.layers.iter().enumerate() {
            let current_type = match layer.get_size_scale_type_from_layer() {
                Some(t) => t,
                None => continue,
            };

            if let Some(ref existing_type) = resolved_type {
                if existing_type != &current_type {
                    return Err(ChartonError::Scale(format!(
                        "Size scale type conflict at layer {}: expected {:?}, found {:?}",
                        i, existing_type, current_type
                    )));
                }
            } else {
                resolved_type = Some(current_type);
            }

            if let Some((min, max)) = layer.get_size_continuous_bounds()? {
                global_min = global_min.min(min);
                global_max = global_max.max(max);
            }
        }

        match resolved_type {
            Some(_) => {
                if global_min.is_infinite() {
                    Ok(Some(ScaleDomain::Continuous(0.0, 1.0)))
                } else {
                    // Ensure the range is not zero
                    if (global_max - global_min).abs() < 1e-12 {
                        global_min -= 0.5;
                        global_max += 0.5;
                    }
                    Ok(Some(ScaleDomain::Continuous(global_min, global_max)))
                }
            },
            None => Ok(None),
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
    /// 1. Finalize Aesthetic & Coordinate Scales.
    /// 2. Determine initial "safe zones" based on user-defined proportional margins.
    /// 3. Measure Legend requirements constrained by the initial safe zones.
    /// 4. Measure Axis requirements based on the remaining space.
    /// 5. Apply a "Minimum Panel Defense" to guarantee at least 100x100px for data rendering,
    ///    pushing legends outside the canvas if necessary rather than crushing the plot.
    fn resolve_rendering_layout(&self, legend_specs: &[LegendSpec]) -> Result<(Box<dyn CoordinateTrait>, Rect, GlobalAesthetics), ChartonError> {
        // --- STEP 1: RESOLVE AESTHETIC SCALES ---
        // Consolidate color, shape, and size mappings across all layers.
        let color_bundle = if let Some((scale_type, domain)) = self.get_color_domain_from_layers()? {
            let scale = create_scale(&scale_type, domain, self.theme.color_expand)?;
            let mapper = VisualMapper::new_color_default(&scale_type, &self.theme);
            Some((scale, mapper))
        } else { None };

        let shape_bundle = if let Some(domain) = self.get_shape_domain_from_layers()? {
            let scale = create_scale(&Scale::Discrete, domain, self.theme.shape_expand)?;
            let mapper = VisualMapper::new_shape_default();
            Some((scale, mapper))
        } else { None };

        let size_bundle = if let Some(domain) = self.get_size_domain_from_layers()? {
            let scale = create_scale(&Scale::Linear, domain, self.theme.size_expand)?;
            // Use a range (2.0, 9.0) radius that fits the 18.0px legend container
            let mapper = VisualMapper::new_size_default(2.0, 9.0);
            Some((scale, mapper))
        } else { None };

        let aesthetics = GlobalAesthetics {
            color: color_bundle,
            shape: shape_bundle,
            size: size_bundle,
        };

        // --- STEP 2: RESOLVE COORDINATE SCALES (X & Y) ---
        // Construct the scales for the primary axes, respecting user overrides for min/max.
        let x_scale = if let Some((stype, mut domain)) = self.get_x_domain_from_layers()? {
            if let ScaleDomain::Continuous(ref mut min, ref mut max) = domain {
                if let Some(u_min) = self.x_domain_min { *min = u_min; }
                if let Some(u_max) = self.x_domain_max { *max = u_max; }
            }
            create_scale(&stype, domain, self.theme.x_expand)?
        } else {
            create_scale(&Scale::Linear, ScaleDomain::Continuous(0.0, 1.0), self.theme.x_expand)?
        };

        let y_scale = if let Some((stype, mut domain)) = self.get_y_domain_from_layers()? {
            if let ScaleDomain::Continuous(ref mut min, ref mut max) = domain {
                if let Some(u_min) = self.y_domain_min { *min = u_min; }
                if let Some(u_max) = self.y_domain_max { *max = u_max; }
            }
            create_scale(&stype, domain, self.theme.y_expand)?
        } else {
            create_scale(&Scale::Linear, ScaleDomain::Continuous(0.0, 1.0), self.theme.y_expand)?
        };

        // Construct the coordinate system trait object.
        let final_coord: Box<dyn CoordinateTrait> = match self.coord_system {
            CoordSystem::Cartesian2D => Box::new(crate::coordinate::cartesian::Cartesian2D::new(
                x_scale, 
                y_scale, 
                self.flipped
            )),
            CoordSystem::Polar => todo!("Polar coordinate resolution not yet implemented"),
        };

        // --- STEP 3: MEASUREMENT PHASE (THE LAYOUT ENGINE) ---
        let w = self.width as f64;
        let h = self.height as f64;

        // A. Calculate initial plot dimensions based purely on proportional margins.
        // This represents the "Theoretical Max" space for the plot + axes.
        let initial_plot_w = w * (1.0 - self.left_margin - self.right_margin);
        let initial_plot_h = h * (1.0 - self.top_margin - self.bottom_margin);

        // B. Measure Legend Constraints.
        // Legends are constrained by the initial plot height (Y-axis length) to ensure 
        // they don't grow taller than the chart itself.
        let legend_box = crate::core::layout::LayoutEngine::calculate_legend_constraints(
            legend_specs,
            self.legend_position,
            w, h,               // Full canvas for defense logic
            initial_plot_w,     // Theoretical width limit
            initial_plot_h,     // Theoretical height limit (Micro-layout ceiling)
            self.legend_margin,
            &self.theme
        );

        // C. Measure Axis Constraints.
        // Create a temporary context that accounts for the space eaten by legends.
        let temp_panel = Rect::new(
            (self.left_margin * w) + legend_box.left,
            (self.top_margin * h) + legend_box.top,
            (initial_plot_w - legend_box.left - legend_box.right).max(0.0),
            (initial_plot_h - legend_box.top - legend_box.bottom).max(0.0)
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
            self.x_label.as_deref().unwrap_or(""),
            self.y_label.as_deref().unwrap_or("")
        );

        // --- STEP 4: FINAL PANEL RESOLUTION & DEFENSE ---
        
        // Combine all pixel requirements (Margins + Legends + Axes).
        let final_left = (self.left_margin * w) + legend_box.left + axis_box.left;
        let final_right = (self.right_margin * w) + legend_box.right;
        let final_top = (self.top_margin * h) + legend_box.top;
        let final_bottom = (self.bottom_margin * h) + legend_box.bottom + axis_box.bottom;

        // Calculate raw plot area.
        let mut plot_w = w - final_left - final_right;
        let mut plot_h = h - final_top - final_bottom;

        // INDUSTRIAL DEFENSE: Guarantee a minimum 100x100px rendering area.
        // If plot_w/h is too small, we force it to 100px. This might push 
        // the legend or right-side margins off the canvas, which is preferred 
        // over an invisible or negative-sized chart.
        let min_dim = 100.0;
        if plot_w < min_dim { plot_w = min_dim; }
        if plot_h < min_dim { plot_h = min_dim; }

        let panel = Rect::new(final_left, final_top, plot_w, plot_h);

        Ok((final_coord, panel, aesthetics))
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
            font_color,
            title_text
        )?;

        Ok(())
    }

    /// Renders the entire layered chart to the provided SVG string.
    ///
    /// This implementation follows the Grammar of Graphics pipeline:
    /// 1. **Sync**: Consolidate data domains across all layers to ensure visual consistency.
    /// 2. **Back-fill**: Update layers with unified scale/domain metadata.
    /// 3. **Layout**: Calculate plot area (Panel) using dynamic margins, axis labels, and legend dimensions.
    /// 4. **Draw**: Render axes, marks, and unified legends using the resolved context.
    pub fn render(&mut self, svg: &mut String) -> Result<(), ChartonError> {
        // 0. Guard: If no layers exist, we render nothing.
        if self.layers.is_empty() { 
            return Ok(()); 
        }

        // --- STEP 1: SYNC & BACK-FILL PHASE ---
        // Calculate global domains from all layers to ensure visual consistency 
        // across the entire chart (e.g., "Red" always means the same category).
        let global_color = self.get_color_domain_from_layers()?;
        let global_shape = self.get_shape_domain_from_layers()?;
        let global_size = self.get_size_domain_from_layers()?;

        // Back-fill: Update each layer with the unified global metadata.
        // This synchronization is a core requirement of the Grammar of Graphics.
        for layer in self.layers.iter_mut() {
            if let Some((scale, domain)) = &global_color {
                layer.set_scale_type("color", scale.clone());
                layer.set_domain("color", domain.clone());
            }
            if let Some(domain) = &global_shape {
                layer.set_domain("shape", domain.clone());
            }
            if let Some(domain) = &global_size {
                layer.set_domain("size", domain.clone());
            }
        }

        // --- STEP 2: LEGEND COLLECTION ---
        // Collect unified legend specifications after the back-fill phase.
        // The LegendManager aggregates requirements from all layers into unique guides.
        // We do this BEFORE layout resolution so the engine knows how much space to reserve.
        let legend_specs = crate::core::legend::LegendManager::collect_legends(&self.layers);

        // --- STEP 3: LAYOUT RESOLUTION ---
        // Resolve the coordinate system, plot area (Panel), and aesthetics rules.
        // This method calculates the 'squeezed' panel by measuring legend and axis constraints.
        let (coord_box, panel, aesthetics) = self.resolve_rendering_layout(&legend_specs)?; 

        // Construct the SharedRenderingContext.
        // Note: We use the 'new' constructor to handle the lifetime association ('a) 
        // between the context and the owned objects (coord_box/aesthetics).
        // By passing &aesthetics as a reference, we satisfy the borrow checker.
        let context = SharedRenderingContext::new(
            &*coord_box, 
            panel,
            self.legend_position,
            self.legend_margin,
            &aesthetics
        );

        // --- STEP 4: DRAWING PHASE ---
        
        // 5. Render Chart Title - Pass the panel to allow vertical centering
        // This is usually rendered at the top of the canvas, outside the Panel.
        self.render_title(svg, &panel)?;

        // 6. Render Axes (X and Y) 
        // We determine if axes are needed by checking chart-level overrides or layer requirements.
        let should_render_axes = if !self.theme.show_axes {
            // 1. Global theme override: If the theme says "no axes", respect it.
            false
        } else {
            // 2. Fallback to layer requirements:
            // Check if any layer explicitly needs axes (e.g., a scatter plot needs them, but a pie chart might not).
            self.layers.iter().any(|layer| layer.requires_axes())
        };

        if should_render_axes {
            // Retrieve labels using helper methods which aggregate or default labels.
            let x_label = self.get_x_axis_label_from_layers();
            let y_label = self.get_y_axis_label_from_layers();

            crate::render::axis_renderer::render_axes(
                svg, 
                &self.theme, 
                &context, 
                &x_label, 
                &y_label
            )?;
        }

        // 7. Render Marks (Data Geometries)
        // Each layer draws its specific marks (points, lines, etc.) within the context's panel.
        // We use an SvgBackend to abstract the raw string manipulations.
        let mut backend = crate::render::backend::svg::SvgBackend::new(svg, Some(&context.panel));
        for layer in &self.layers {
            layer.render_marks(&mut backend, &context)?;
        }

        // 8. Render Unified Legends
        // The LegendRenderer uses the context and theme to position the legend blocks 
        // in the margins calculated during the Layout Phase.
        crate::render::legend_renderer::LegendRenderer::render_legend(
            svg, 
            &legend_specs, 
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
            self.theme.background_color
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
