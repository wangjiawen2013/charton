use crate::scale::{Expansion, ResolvedScale, Scale, ScaleDomain};

/// Represents a polygon path-grouping encoding for geographic/geometric marks.
///
/// In long-form (tidy) map data, each row represents a single vertex of a polygon.
/// The `PathGroup` channel identifies which polygon each vertex belongs to.
///
/// An optional subpath field (`with_subpath`) separates the rings of one shape:
/// every `path_group` becomes one path, and each distinct subpath value inside
/// it becomes one ring. A ring with a negative/opposite winding is not needed:
/// closed paths are filled with the even-odd rule, so an inner ring
/// automatically punches a hole. Without a subpath field, each `path_group` is
/// one ring.
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
///     .mark_polygon()?
///     .encode((
///         alt::x("lon"),
///         alt::y("lat"),
///         alt::path_group("region"),
///     ))?
///     .with_coord(CoordSystem::Geo)
///     .save("map.svg")?;
/// ```
#[derive(Debug, Clone)]
pub struct PathGroup {
    /// The name of the data column used to group vertices into distinct shapes.
    pub(crate) field: String,

    /// Optional column that separates rings (subpaths) inside one shape.
    pub(crate) subpath: Option<String>,

    /// The desired scale type (typically inferred as Discrete for group identifiers).
    pub(crate) scale_type: Option<Scale>,

    /// An explicit domain for the grouping categories.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding/buffer to the scale domain.
    pub(crate) expansion: Option<Expansion>,

    // --- System Resolution (Result/Outputs) ---
    /// Stores the resolved scale instance.
    #[allow(dead_code)]
    pub(crate) resolved_scale: ResolvedScale,
}

impl PathGroup {
    /// Creates a new PathGroup encoding for a specific data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            subpath: None,
            scale_type: None,
            domain: None,
            expansion: None,
            resolved_scale: ResolvedScale::none(),
        }
    }

    /// Separates multiple rings inside one path group.
    ///
    /// Use this with GeoJSON output (`_ring_id`) or any column that identifies
    /// a subpath. Closed paths are filled with the even-odd rule, so an inner
    /// ring becomes a hole without relying on its winding direction.
    pub fn with_subpath(mut self, field: &str) -> Self {
        self.subpath = Some(field.to_string());
        self
    }

    /// The subpath column, when one was configured.
    pub(crate) fn subpath_field(&self) -> Option<&str> {
        self.subpath.as_deref()
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
