use crate::error::ChartonError;
use crate::theme::Theme;
use crate::coordinate::{CoordinateTrait, Rect, cartesian::Cartesian2D};
use crate::scale::{Scale, ScaleDomain, Expansion, create_scale};
use super::layer::Layer;
use super::context::SharedRenderingContext;

/// `LayeredChart` is a multi-layer container that combines multiple chart components
/// into a single visualization. 
///
/// It holds shared properties such as dimensions, margins, theme settings, 
/// and global axis configurations that apply to the entire visualization.
pub struct LayeredChart {
    width: u32,
    height: u32,
    left_margin: f64,
    right_margin: f64,
    top_margin: f64,
    bottom_margin: f64,
    theme: Theme,

    title: Option<String>,
    layers: Vec<Box<dyn Layer>>,

    // Manual domain overrides
    x_domain_min: Option<f64>,
    x_domain_max: Option<f64>,
    y_domain_min: Option<f64>,
    y_domain_max: Option<f64>,

    // Axis labels and custom ticks
    x_label: Option<String>,
    y_label: Option<String>,
    x_tick_values: Option<Vec<f64>>,
    x_tick_labels: Option<Vec<String>>,
    y_tick_values: Option<Vec<f64>>,
    y_tick_labels: Option<Vec<String>>,

    /// Flag indicating whether x and y axes should be flipped (swapped).
    flipped: bool,

    legend: Option<bool>,
    legend_title: Option<String>,

    background: Option<String>,
    axes: Option<bool>,
}

impl Default for LayeredChart {
    fn default() -> Self {
        Self::new()
    }
}

impl LayeredChart {
    /// Creates a new `LayeredChart` with default settings.
    ///
    /// Defaults: 500x400px, 15% left/bottom margins, 10% right/top margins, 
    /// default theme, and white background.
    pub fn new() -> Self {
        Self {
            width: 500,
            height: 400,
            left_margin: 0.15,
            right_margin: 0.10,
            top_margin: 0.10,
            bottom_margin: 0.15,
            theme: Theme::default(),

            title: None,
            layers: Vec::new(),

            x_domain_min: None,
            x_domain_max: None,
            x_label: None,
            x_tick_values: None,
            x_tick_labels: None,

            y_domain_min: None,
            y_domain_max: None,
            y_label: None,
            y_tick_values: None,
            y_tick_labels: None,

            flipped: false,

            legend: None,
            legend_title: None,

            background: Some("white".to_string()),
            axes: None,
        }
    }

    /// Sets the dimensions of the chart in pixels.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Flips the X and Y axes (often used for horizontal bar charts).
    pub fn flip(mut self, flipped: bool) -> Self {
        self.flipped = flipped;
        self
    }

    /// Adds a new layer to the chart. 
    /// Layers are rendered in the order they are added (bottom to top).
    pub fn add_layer(mut self, layer: Box<dyn Layer>) -> Self {
        self.layers.push(layer);
        self
    }

    /// Calculates the effective right margin by considering the space required 
    /// for legends across all layers.
    fn calculate_effective_right_margin(&self) -> f64 {
        let mut max_legend_width = 0.0;
        let chart_height = self.height as f64;
        let top_px = self.top_margin * chart_height;
        let bottom_px = self.bottom_margin * chart_height;
        let available_height = chart_height - top_px - bottom_px;

        for layer in &self.layers {
            let width = layer.calculate_legend_width(
                &self.theme, 
                available_height, 
                top_px, 
                bottom_px
            );
            if width > max_legend_width {
                max_legend_width = width;
            }
        }

        let margin_extension = max_legend_width / (self.width as f64);
        self.right_margin + margin_extension
    }

    /// Orchestrates the data boundaries and creates the coordinate system.
    pub fn build_coordinate_system(&self) -> Result<Box<dyn CoordinateTrait>, ChartonError> {
        // 1. Collect global X bounds from all layers unless overridden
        let mut x_min = self.x_domain_min.unwrap_or(f64::INFINITY);
        let mut x_max = self.x_domain_max.unwrap_or(f64::NEG_INFINITY);

        if self.x_domain_min.is_none() || self.x_domain_max.is_none() {
            for layer in &self.layers {
                let (l_min, l_max) = layer.get_x_continuous_bounds()?;
                if self.x_domain_min.is_none() { x_min = x_min.min(l_min); }
                if self.x_domain_max.is_none() { x_max = x_max.max(l_max); }
            }
        }

        // 2. Collect global Y bounds from all layers unless overridden
        let mut y_min = self.y_domain_min.unwrap_or(f64::INFINITY);
        let mut y_max = self.y_domain_max.unwrap_or(f64::NEG_INFINITY);

        if self.y_domain_min.is_none() || self.y_domain_max.is_none() {
            for layer in &self.layers {
                let (l_min, l_max) = layer.get_y_continuous_bounds()?;
                if self.y_domain_min.is_none() { y_min = y_min.min(l_min); }
                if self.y_domain_max.is_none() { y_max = y_max.max(l_max); }
            }
        }

        // 3. Determine Scale types (using the first layer as a strategy provider)
        let x_scale_type = self.layers.first()
            .and_then(|l| l.get_x_scale_type().ok().flatten())
            .unwrap_or(Scale::Linear);

        let y_scale_type = self.layers.first()
            .and_then(|l| l.get_y_scale_type().ok().flatten())
            .unwrap_or(Scale::Linear);

        // 4. Instantiate the concrete scales
        let x_scale = create_scale(&x_scale_type, ScaleDomain::Continuous(x_min, x_max), Expansion::default())?;
        let y_scale = create_scale(&y_scale_type, ScaleDomain::Continuous(y_min, y_max), Expansion::default())?;

        // 5. Create the Cartesian coordinate system with the flipped state
        Ok(Box::new(Cartesian2D::new(x_scale, y_scale, self.flipped)))
    }

    /// Renders the composite visualization into an SVG string.
    pub fn render(&self) -> Result<String, ChartonError> {
        let mut svg = String::new();
        let eff_right = self.calculate_effective_right_margin();
        
        let panel = Rect::new(
            self.left_margin * self.width as f64,
            self.top_margin * self.height as f64,
            (1.0 - self.left_margin - eff_right) * self.width as f64,
            (1.0 - self.top_margin - self.bottom_margin) * self.height as f64,
        );

        // build_coordinate_system now correctly passes self.flipped to Cartesian2D
        let coord = self.build_coordinate_system()?;
        
        let ctx = SharedRenderingContext::new(
            coord.as_ref(),
            panel,
            self.flipped,
            self.legend.unwrap_or(false),
        );

        // Render Marks
        for layer in &self.layers {
            layer.render_marks(&mut svg, &ctx)?;
        }

        // Render Legends
        for layer in &self.layers {
            layer.render_legends(&mut svg, &self.theme, &ctx)?;
        }

        Ok(svg)
    }
}