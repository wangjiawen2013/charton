use crate::scale::{Expansion, ResolvedScale, Scale, ScaleDomain};

/// Represents a polygon path-grouping encoding for geographic/geometric marks.
///
/// In long-form (tidy) map data, each row represents a single vertex of a polygon.
/// The `PathGroup` channel identifies which polygon each vertex belongs to.
/// All rows sharing the same `field` value will be assembled into a single closed path.
///
/// This follows the Grammar of Graphics convention: polygons are defined by
/// grouping sorted (x, y) point sequences via a categorical identifier.
///
/// # Example
///
/// ```ignore
/// // Data: each row is a vertex of a region boundary
/// let ds = Dataset::new()
///     .with_column("lon", vec![116.4, 116.5, 116.5, 116.4])?
///     .with_column("lat", vec![39.9, 39.9, 40.0, 40.0])?
///     .with_column("region", vec!["Beijing"; 4])?;
///
/// Chart::build(ds)?
///     .mark_geo_path()?
///     .encode((
///         alt::x("lon"),
///         alt::y("lat"),
///         alt::path_group("region"),
///     ))?
///     .save("map.svg")?;
/// ```
#[derive(Debug, Clone)]
pub struct PathGroup {
    /// The name of the data column used to group vertices into distinct polygons.
    pub(crate) field: String,

    /// The desired scale type (typically inferred as Discrete for group identifiers).
    pub(crate) scale_type: Option<Scale>,

    /// An explicit domain for the grouping categories.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding/buffer to the scale domain.
    pub(crate) expansion: Option<Expansion>,

    // --- System Resolution (Result/Outputs) ---
    /// Stores the resolved scale instance.
    pub(crate) resolved_scale: ResolvedScale,
}

impl PathGroup {
    /// Creates a new PathGroup encoding for a specific data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            scale_type: None,
            domain: None,
            expansion: None,
            resolved_scale: ResolvedScale::none(),
        }
    }

    /// Sets the preferred scale type (typically `Scale::Discrete`).
    pub const fn with_scale(mut self, scale_type: Scale) -> Self {
        self.scale_type = Some(scale_type);
        self
    }

    /// Explicitly sets the data domain for grouping categories.
    pub fn with_domain(mut self, domain: ScaleDomain) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Configures the expansion padding for the grouping scale.
    pub const fn with_expansion(mut self, expansion: Expansion) -> Self {
        self.expansion = Some(expansion);
        self
    }
}

/// Convenience builder function to create a new PathGroup encoding.
///
/// # Arguments
/// * `field` - The column name identifying polygon membership.
pub fn path_group(field: &str) -> PathGroup {
    PathGroup::new(field)
}
