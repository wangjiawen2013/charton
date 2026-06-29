use super::{CoordLayout, CoordinateTrait, Rect};
use crate::core::layer::RenderBackend;
use crate::error::ChartonError;
use crate::scale::{ExplicitTick, ScaleTrait};
use crate::theme::Theme;
use crate::visual::color::SingleColor;
use std::f64::consts::PI;
use std::sync::Arc;

// ============================================================================
// Geo Projection Enum
// ============================================================================

/// Supported map projections for the Geo coordinate system.
///
/// Following modern cartographic best practices, **equal-area** projections
/// are the default choice for statistical data visualization. Conformal
/// projections like Mercator must be explicitly selected by the user.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum GeoProjection {
    /// **Equal Earth** — Modern equal-area pseudocylindrical projection.
    ///
    /// Designed by Savric et al. (2019) as a visually pleasing, area-accurate
    /// alternative to Gall-Peters. This is the default projection used by
    /// Altair/Vega-Lite and is recommended for choropleth maps and any
    /// visualization where area comparisons are meaningful.
    ///
    /// Formula: polynomial evaluation. No iterative solving required.
    #[default]
    EqualEarth,

    /// **Mollweide** — Classic equal-area pseudocylindrical projection (1805).
    ///
    /// The traditional choice for global thematic maps (ggplot2's default).
    /// Requires Newton-Raphson iteration.
    Mollweide,

    /// **Equirectangular** — Simple linear mapping of longitude/latitude to x/y.
    ///
    /// Also known as Plate Carree. No area preservation, but useful for
    /// debugging and viewing raw coordinate grids.
    Equirectangular,

    /// **Mercator** — Conformal cylindrical projection (1569).
    ///
    /// Preserves angles but severely distorts area. Must be explicitly selected.
    /// Only appropriate for navigation or tile-map overlays.
    Mercator,
}

// ============================================================================
// Geo Coordinate System
// ============================================================================

/// A 2D geographic coordinate system with selectable map projections.
///
/// In a Geo system:
/// - **X dimension** is mapped to **Longitude**, typically [-180, 180] or [0, 360].
/// - **Y dimension** is mapped to **Latitude**, typically [-90, 90].
///
/// The `transform` method first applies the selected projection to convert
/// spherical coordinates into planar (x, y) space, then scales the result
/// to fit within the target panel.
pub struct Geo {
    pub x_scale: Arc<dyn ScaleTrait>, // Longitude scale
    pub y_scale: Arc<dyn ScaleTrait>, // Latitude scale
    pub x_field: String,
    pub y_field: String,

    /// The cartographic projection to apply.
    pub projection: GeoProjection,

    /// Central meridian (longitude) in radians. Defaults to 0 (Greenwich).
    pub center_lon: f64,

    /// Central parallel (latitude) in radians. Defaults to 0 (Equator).
    pub center_lat: f64,
}

impl Geo {
    /// Creates a new Geo coordinate system from two boxed scales.
    pub fn new(
        x_scale: Arc<dyn ScaleTrait>,
        y_scale: Arc<dyn ScaleTrait>,
        x_field: String,
        y_field: String,
    ) -> Self {
        Self {
            x_scale,
            y_scale,
            x_field,
            y_field,
            projection: GeoProjection::default(),
            center_lon: 0.0,
            center_lat: 0.0,
        }
    }

    /// Sets the map projection (builder pattern).
    pub const fn with_projection(mut self, projection: GeoProjection) -> Self {
        self.projection = projection;
        self
    }

    /// Sets the central meridian in degrees (builder pattern).
    pub fn with_center_lon(mut self, degrees: f64) -> Self {
        self.center_lon = degrees.to_radians();
        self
    }

    /// Sets the central parallel in degrees (builder pattern).
    pub fn with_center_lat(mut self, degrees: f64) -> Self {
        self.center_lat = degrees.to_radians();
        self
    }

    /// Maps a raw (longitude_radians, latitude_radians) pair through the
    /// active projection into planar (x, y) coordinates.
    fn project_point(&self, lon_rad: f64, lat_rad: f64) -> (f64, f64) {
        let lambda = lon_rad - self.center_lon;
        let phi = lat_rad - self.center_lat;

        match self.projection {
            GeoProjection::EqualEarth => project_equal_earth(lambda, phi),
            GeoProjection::Mollweide => project_mollweide(lambda, phi),
            GeoProjection::Equirectangular => project_equirectangular(lambda, phi),
            GeoProjection::Mercator => project_mercator(lambda, phi),
        }
    }
}

// ============================================================================
// Projection Implementations
// ============================================================================

/// Simple linear projection. No area or angle preservation.
fn project_equirectangular(lon_rad: f64, lat_rad: f64) -> (f64, f64) {
    (lon_rad, lat_rad)
}

/// Conformal cylindrical projection.
fn project_mercator(lon_rad: f64, lat_rad: f64) -> (f64, f64) {
    let phi = lat_rad.clamp(-1.4844, 1.4844); // approx +/- 85.06 degrees
    let y = (PI / 4.0 + phi / 2.0).tan().ln();
    (lon_rad, y)
}

// --- Equal Earth ---

const A1: f64 = 1.340264;
const A2: f64 = -0.081106;
const A3: f64 = 0.000893;
const A4: f64 = 0.003796;
const SQRT3: f64 = 1.732_050_807_568_877_2;

/// Equal Earth — modern equal-area pseudocylindrical projection.
fn project_equal_earth(lon_rad: f64, lat_rad: f64) -> (f64, f64) {
    let sin_phi = lat_rad.sin();
    let theta = ((SQRT3 / 2.0) * sin_phi).asin();
    let theta2 = theta * theta;
    let theta4 = theta2 * theta2;
    let theta6 = theta4 * theta2;

    let cos_theta = theta.cos();

    let denom = 3.0 * (A1 + 3.0 * A2 * theta2 + 5.0 * A3 * theta4 + 7.0 * A4 * theta6);
    let x = (2.0 * SQRT3 * lon_rad * cos_theta) / denom;

    let y = theta.mul_add(theta2.mul_add(theta2.mul_add(A4, A3), A2), A1) * theta;

    (x * 1.1, y * 1.3)
}

// --- Mollweide ---

const SQRT2: f64 = 1.414_213_562_373_095_1;

/// Mollweide — classic equal-area pseudocylindrical projection (1805).
fn project_mollweide(lon_rad: f64, lat_rad: f64) -> (f64, f64) {
    if lat_rad.abs() >= PI / 2.0 {
        let theta = lat_rad;
        let x = (2.0 * SQRT3 / PI) * lon_rad * theta.cos();
        let y = SQRT3 * theta.sin();
        return (x * 0.8, y * 0.8);
    }

    let target = PI * lat_rad.sin();
    let mut theta = lat_rad;
    for _ in 0..10 {
        let delta = (theta - theta.sin() - target) / (1.0 - theta.cos());
        theta -= delta;
        if delta.abs() < 1e-10 {
            break;
        }
    }

    let x = (2.0 * SQRT2 / PI) * lon_rad * theta.cos();
    let y = SQRT2 * theta.sin();

    (x * 0.8, y * 0.8)
}

// ============================================================================
// Scale Inversion Helper
// ============================================================================

/// Inverts a normalized [0, 1] value back to the raw data domain.
///
/// Uses the scale's `domain()` method which returns `(min, max)`.
/// For linear scales (the expected type for geographic coordinates),
/// this is a simple linear interpolation.
fn invert_norm(scale: &dyn ScaleTrait, norm: f64) -> f64 {
    let (domain_min, domain_max) = scale.domain();
    domain_min + norm * (domain_max - domain_min)
}

// ============================================================================
// CoordinateTrait Implementation
// ============================================================================

impl CoordinateTrait for Geo {
    fn render_axes(
        &self,
        backend: &mut dyn RenderBackend,
        theme: &Theme,
        panel: &Rect,
        x_label: &str,
        x_explicit: Option<&[ExplicitTick]>,
        y_label: &str,
        y_explicit: Option<&[ExplicitTick]>,
    ) -> Result<(), ChartonError> {
        crate::render::geo_axis_renderer::render_geo_axes(
            backend, theme, panel, self, x_label, x_explicit, y_label, y_explicit,
        )
    }

    fn render_grid_lines(
        &self,
        backend: &mut dyn RenderBackend,
        theme: &Theme,
        panel: &Rect,
        x_explicit: Option<&[ExplicitTick]>,
        y_explicit: Option<&[ExplicitTick]>,
    ) -> Result<(), ChartonError> {
        crate::render::geo_axis_renderer::render_geo_grid(
            backend, theme, panel, self, x_explicit, y_explicit,
        )
    }

    fn transform(&self, x_norm: f64, y_norm: f64, panel: &Rect) -> (f64, f64) {
        let (proj_bounds, panel_bounds) = self.compute_projection_bounds(panel);

        let lon = invert_norm(self.x_scale.as_ref(), x_norm);
        let lat = invert_norm(self.y_scale.as_ref(), 1.0 - y_norm);

        let (proj_x, proj_y) = self.project_point(lon.to_radians(), lat.to_radians());

        let px_range = panel_bounds.1 - panel_bounds.0;
        let py_range = panel_bounds.3 - panel_bounds.2;

        let px_diff = if proj_bounds.1 > proj_bounds.0 {
            proj_bounds.1 - proj_bounds.0
        } else {
            1.0
        };
        let py_diff = if proj_bounds.3 > proj_bounds.2 {
            proj_bounds.3 - proj_bounds.2
        } else {
            1.0
        };

        let final_x = panel_bounds.0 + ((proj_x - proj_bounds.0) / px_diff) * px_range;
        let final_y = panel_bounds.2 + ((proj_bounds.3 - proj_y) / py_diff) * py_range;

        (final_x, final_y)
    }

    fn transform_path(
        &self,
        points: &[(f64, f64)],
        is_closed: bool,
        panel: &Rect,
    ) -> Vec<(f64, f64)> {
        if points.is_empty() {
            return vec![];
        }

        let needs_interpolation = !matches!(
            self.projection,
            GeoProjection::Equirectangular | GeoProjection::Mercator
        );

        if !needs_interpolation {
            return points
                .iter()
                .map(|(x, y)| self.transform(*x, *y, panel))
                .collect();
        }

        let mut result = Vec::with_capacity(points.len() * 4);
        let threshold = 0.005;

        for i in 0..points.len() {
            let p1 = points[i];
            result.push(self.transform(p1.0, p1.1, panel));

            let next_point = if i + 1 < points.len() {
                Some(points[i + 1])
            } else if is_closed && !points.is_empty() {
                Some(points[0])
            } else {
                None
            };

            if let Some(p2) = next_point {
                let dx = (p2.0 - p1.0).abs();
                let dy = (p2.1 - p1.1).abs();
                let dist = dx.max(dy);

                if dist > threshold {
                    let steps = (dist / threshold).ceil() as usize;
                    for s in 1..steps {
                        let t = s as f64 / steps as f64;
                        result.push(self.transform(
                            p1.0 + (p2.0 - p1.0) * t,
                            p1.1 + (p2.1 - p1.1) * t,
                            panel,
                        ));
                    }
                }
            }
        }

        result
    }

    fn get_x_arc(&self) -> Arc<dyn ScaleTrait> {
        self.x_scale.clone()
    }

    fn get_y_arc(&self) -> Arc<dyn ScaleTrait> {
        self.y_scale.clone()
    }

    fn get_x_scale(&self) -> &dyn ScaleTrait {
        self.x_scale.as_ref()
    }

    fn get_y_scale(&self) -> &dyn ScaleTrait {
        self.y_scale.as_ref()
    }

    fn get_x_label(&self) -> &str {
        &self.x_field
    }

    fn get_y_label(&self) -> &str {
        &self.y_field
    }

    fn is_flipped(&self) -> bool {
        false
    }

    fn is_clipped(&self) -> bool {
        true
    }

    fn layout_hints(&self) -> CoordLayout {
        CoordLayout {
            default_bar_stroke: SingleColor::new("#333333"),
            default_bar_stroke_width: 0.5,
            default_bar_width: 1.0,
            default_bar_spacing: 0.0,
            default_bar_span: 1.0,
            needs_interpolation: !matches!(
                self.projection,
                GeoProjection::Equirectangular | GeoProjection::Mercator
            ),
        }
    }
}

// ============================================================================
// Internal Helper: Projection Bounds Computation
// ============================================================================

impl Geo {
    fn compute_projection_bounds(
        &self,
        panel: &Rect,
    ) -> ((f64, f64, f64, f64), (f64, f64, f64, f64)) {
        let margin_ratio = 0.05;

        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        let samples = [
            (0.0, 0.0),
            (0.5, 0.0),
            (1.0, 0.0),
            (0.0, 0.5),
            (0.5, 0.5),
            (1.0, 0.5),
            (0.0, 1.0),
            (0.5, 1.0),
            (1.0, 1.0),
            (0.25, 0.25),
            (0.75, 0.25),
            (0.25, 0.75),
            (0.75, 0.75),
        ];

        for (xn, yn) in &samples {
            let lon = invert_norm(self.x_scale.as_ref(), *xn);
            let lat = invert_norm(self.y_scale.as_ref(), 1.0 - *yn);

            let (px, py) = self.project_point(lon.to_radians(), lat.to_radians());
            min_x = min_x.min(px);
            max_x = max_x.max(px);
            min_y = min_y.min(py);
            max_y = max_y.max(py);
        }

        if max_x <= min_x {
            max_x = min_x + 1.0;
        }
        if max_y <= min_y {
            max_y = min_y + 1.0;
        }

        let proj_bounds = (min_x, max_x, min_y, max_y);

        let margin_x = panel.width * margin_ratio;
        let margin_y = panel.height * margin_ratio;

        let panel_x_min = panel.x + margin_x;
        let panel_x_max = panel.x + panel.width - margin_x;
        let panel_y_min = panel.y + margin_y;
        let panel_y_max = panel.y + panel.height - margin_y;

        let panel_bounds = (panel_x_min, panel_x_max, panel_y_min, panel_y_max);

        (proj_bounds, panel_bounds)
    }
}
