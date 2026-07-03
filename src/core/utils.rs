use ahash::AHashMap;

/// Estimates text width using character categorization.
pub(crate) fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    let mut narrow_chars = 0;
    let mut uppercase_chars = 0;
    let mut other_chars = 0;

    for c in text.chars() {
        if matches!(
            c,
            '.' | ',' | ':' | ';' | '!' | 'i' | 'j' | 'l' | '-' | '|' | '1' | 't' | 'f' | 'r'
        ) {
            narrow_chars += 1;
        } else if c.is_ascii_uppercase() {
            uppercase_chars += 1;
        } else {
            other_chars += 1;
        }
    }

    (narrow_chars as f64 * 0.3 + uppercase_chars as f64 * 0.65 + other_chars as f64 * 0.55)
        * font_size
}

// =============================== Font Database Utilities =====================================
#[cfg(any(feature = "png", feature = "pdf"))]
use std::sync::OnceLock;

#[cfg(feature = "pdf")]
use std::sync::Arc;

/// Global cache for the font database to avoid expensive system scans on every render.
/// We use Arc to allow thread-safe sharing across multiple chart generation tasks.
#[cfg(feature = "pdf")]
static GLOBAL_FONT_DB: OnceLock<Arc<svg2pdf::usvg::fontdb::Database>> = OnceLock::new();

/// Retrieves a shared instance of the font database.
///
/// The first call triggers a full system font scan and loads the embedded fallback fonts.
/// Subsequent calls return a cloned reference to the cached database, reducing the
/// font initialization overhead to nearly 0ms.
#[cfg(feature = "pdf")]
pub(crate) fn get_font_db() -> Arc<svg2pdf::usvg::fontdb::Database> {
    GLOBAL_FONT_DB
        .get_or_init(|| {
            // Initialize the database using the usvg version re-exported by svg2pdf.
            let mut fontdb = svg2pdf::usvg::fontdb::Database::new();

            // 1. Scan the operating system's font directories.
            // This is an expensive I/O operation, which is why we cache the result globally.
            fontdb.load_system_fonts();

            // 2. Load the built-in "emergency" font.
            // This ensures consistent text rendering even in restricted environments
            // like minimal Docker containers or CI runners where system fonts may be missing.
            let default_font_data = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
            fontdb.load_font_data(default_font_data.to_vec());

            // 3. Define the default fallback family for 'sans-serif' requests.
            // When an SVG specifies "sans-serif" but no system mapping exists,
            // the renderer will use the "Inter" font we just loaded.
            fontdb.set_sans_serif_family("Inter");

            Arc::new(fontdb)
        })
        .clone()
}

// =============================== Raster Font Utilities (PNG) =====================================
#[cfg(feature = "png")]
use ab_glyph::FontArc;
#[cfg(feature = "png")]
use std::sync::RwLock;

/// Global cache for raster fonts.
/// Maps lowercase font family names to FontArc instances.
#[cfg(feature = "png")]
static RASTER_FONT_REGISTRY: OnceLock<RwLock<AHashMap<String, FontArc>>> = OnceLock::new();

/// Global cache for the system font database.
/// This allows us to search for system fonts by name without rescanning the OS directories every time.
#[cfg(feature = "png")]
static SYSTEM_FONT_DB: OnceLock<fontdb::Database> = OnceLock::new();

/// Retrieves or initializes the global system font database.
/// This performs an expensive I/O operation (scanning OS font dirs) only once.
#[cfg(feature = "png")]
fn get_system_font_db() -> &'static fontdb::Database {
    SYSTEM_FONT_DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        // Load all available system fonts
        db.load_system_fonts();
        db
    })
}

/// Initializes the font registry with the default embedded font (Inter).
#[cfg(feature = "png")]
fn get_raster_registry() -> &'static RwLock<AHashMap<String, FontArc>> {
    RASTER_FONT_REGISTRY.get_or_init(|| {
        let mut map = AHashMap::new();

        // Load default fallback font (Inter) from embedded assets
        let default_font_data = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
        if let Ok(font) = FontArc::try_from_slice(default_font_data) {
            // Register as "inter" for explicit requests
            map.insert("inter".to_string(), font.clone());
            // Register as "sans-serif" for generic fallbacks
            map.insert("sans-serif".to_string(), font);
        } else {
            eprintln!("Warning: Failed to load default Inter font for raster rendering.");
        }

        RwLock::new(map)
    })
}

/// Retrieves a raster font by family name.
///
/// Logic:
/// 1. Check if the font is already loaded in our local cache.
/// 2. If not, search the System Font Database for the family name.
/// 3. If found in system, load it into memory, cache it, and return it.
/// 4. If not found in system, fallback to "sans-serif" (Inter).
///
/// # Arguments
/// * `family` - The font family name (e.g., "Arial", "Times New Roman", "Inter"). Case-insensitive.
#[cfg(feature = "png")]
pub(crate) fn get_raster_font(family: &str) -> FontArc {
    let registry = get_raster_registry();

    // 1. Fast path: Check if already loaded in our local cache
    {
        let map = registry.read().expect("Failed to read font registry");
        if let Some(font) = map.get(&family.to_lowercase()) {
            return font.clone();
        }
    }

    // 2. Slow path: Search system fonts and load if found
    // We need a write lock to insert the newly loaded font into the cache
    let mut map = registry.write().expect("Failed to write to font registry");

    // Double-check after acquiring write lock (another thread might have loaded it)
    if let Some(font) = map.get(&family.to_lowercase()) {
        return font.clone();
    }

    // Search in the global system font database
    let sys_db = get_system_font_db();
    let families = [fontdb::Family::Name(family)];
    let query = fontdb::Query {
        families: &families,
        weight: fontdb::Weight::NORMAL,
        stretch: fontdb::Stretch::Normal,
        style: fontdb::Style::Normal,
    };

    // Try to find the font in the system database and load it
    if let Some(id) = sys_db.query(&query)
        && let Some(face_info) = sys_db.face(id)
        && let fontdb::Source::File(ref path) = face_info.source
        && let Ok(font_data) = std::fs::read(path)
        && let Ok(font) = FontArc::try_from_vec(font_data)
    {
        // Cache it for future requests
        map.insert(family.to_lowercase(), font.clone());
        return font;
    }

    // Fallback: If system font not found or failed to load, use sans-serif (Inter)
    if let Some(font) = map.get("sans-serif") {
        return font.clone();
    }

    // Ultimate fallback: Panic if no fonts are available (should not happen if Inter loads correctly)
    panic!(
        "No fonts available. Default 'Inter' font failed to load and '{}' not found in system.",
        family
    );
}

/// Registers a new font for raster rendering from raw data.
///
/// This allows users to add custom fonts that are not present in the system
/// or embedded assets.
///
/// # Arguments
/// * `name` - The font family name (e.g., "MyCustomFont").
/// * `data` - The raw TTF/OTF font data.
///
/// # Example
/// ```ignore
/// use charton::core::utils::register_raster_font;
/// let font_data = std::fs::read("path/to/font.ttf")?;
/// register_raster_font("MyFont", font_data)?;
/// ```
#[cfg(feature = "png")]
pub fn register_raster_font(name: &str, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    // try_from_vec takes ownership of the Vec, ensuring the data lives as long as the FontArc
    let font = FontArc::try_from_vec(data)?;

    let registry = get_raster_registry();
    let mut map = registry.write().expect("Failed to write to font registry");
    map.insert(name.to_lowercase(), font);
    Ok(())
}

//=============================== Parallelization Utilities =====================================
#[cfg(feature = "parallel")]
use rayon::prelude::*;

// --- Trait Definitions ---

pub trait IntoParallelizable {
    type Item;
    #[cfg(feature = "parallel")]
    type Iter: ParallelIterator<Item = Self::Item>;
    #[cfg(not(feature = "parallel"))]
    type Iter: Iterator<Item = Self::Item>;

    fn maybe_into_par_iter(self) -> Self::Iter;
}

pub trait Parallelizable {
    type Item;
    #[cfg(feature = "parallel")]
    type Iter: ParallelIterator<Item = Self::Item>;
    #[cfg(not(feature = "parallel"))]
    type Iter: Iterator<Item = Self::Item>;

    fn maybe_par_iter(self) -> Self::Iter;
}

// --- Implementation for Shared References (Read-only) ---

impl<'a, T: Sync + Send + 'a> Parallelizable for &'a Vec<T> {
    type Item = &'a T;

    #[cfg(feature = "parallel")]
    type Iter = rayon::slice::Iter<'a, T>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::slice::Iter<'a, T>;

    fn maybe_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.par_iter()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.iter()
        }
    }
}

impl<'a, T: Sync + Send + 'a> Parallelizable for &'a [T] {
    type Item = &'a T;

    #[cfg(feature = "parallel")]
    type Iter = rayon::slice::Iter<'a, T>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::slice::Iter<'a, T>;

    fn maybe_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.par_iter()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.iter()
        }
    }
}

// --- Implementation for Mutable References (Read-Write) ---

impl<'a, T: Sync + Send + 'a> Parallelizable for &'a mut Vec<T> {
    type Item = &'a mut T;

    #[cfg(feature = "parallel")]
    type Iter = rayon::slice::IterMut<'a, T>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::slice::IterMut<'a, T>;

    fn maybe_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.par_iter_mut()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.iter_mut()
        }
    }
}

// --- Implementation for AHashMap (Shared) ---

impl<'a, K: Sync + Send + 'a, V: Sync + Send + 'a> Parallelizable for &'a AHashMap<K, V> {
    type Item = (&'a K, &'a V);

    #[cfg(feature = "parallel")]
    type Iter = rayon::collections::hash_map::Iter<'a, K, V>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::collections::hash_map::Iter<'a, K, V>;

    fn maybe_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.par_iter()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.iter()
        }
    }
}

// --- Implementation for AHashMap (Mutable) ---

impl<'a, K: Sync + Send + 'a, V: Sync + Send + 'a> Parallelizable for &'a mut AHashMap<K, V> {
    type Item = (&'a K, &'a mut V);

    #[cfg(feature = "parallel")]
    type Iter = rayon::collections::hash_map::IterMut<'a, K, V>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::collections::hash_map::IterMut<'a, K, V>;

    fn maybe_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.par_iter_mut()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.iter_mut()
        }
    }
}

// --- Implementation for Owned Types (Consuming) ---

impl<T: Send + Sync> IntoParallelizable for Vec<T> {
    type Item = T;
    #[cfg(feature = "parallel")]
    type Iter = rayon::vec::IntoIter<T>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::vec::IntoIter<T>;

    fn maybe_into_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.into_par_iter()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.into_iter()
        }
    }
}

impl IntoParallelizable for std::ops::Range<usize> {
    type Item = usize;
    #[cfg(feature = "parallel")]
    type Iter = rayon::range::Iter<usize>;
    #[cfg(not(feature = "parallel"))]
    type Iter = std::ops::Range<usize>;

    fn maybe_into_par_iter(self) -> Self::Iter {
        #[cfg(feature = "parallel")]
        {
            self.into_par_iter()
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.into_iter()
        }
    }
}

// Utility helpers for Charton.
//
// This module contains helper functions that belong to the data conversion and
// dataset construction layer rather than the chart rendering pipeline. It is
// intentionally designed to be lightweight and reusable across different
// client applications.

#[cfg(feature = "geo")]
use crate::core::data::{ColumnVector, Dataset};
#[cfg(feature = "geo")]
use crate::error::ChartonError;
#[cfg(feature = "geo")]
use geojson::{GeoJson, GeometryValue};
#[cfg(feature = "geo")]
use serde_json::Value;
#[cfg(feature = "geo")]
use std::collections::{HashMap, HashSet};

#[cfg(feature = "geo")]
/// Convert a GeoJSON FeatureCollection into a Charton Dataset.
///
/// This utility is intended for generic GeoJSON ingestion where the property
/// columns are not fixed in advance. It supports:
///
/// - arbitrary property keys per feature
/// - mixed column orders across features
/// - automatic column type inference for numeric, boolean, and string values
/// - vertex-level duplication of feature properties for polygon/multipolygon data
/// - automatic generation of helper columns: `_lon`, `_lat`, `_geometry_type`, `_part_id`, `_vertex_order`, and `_path_id`
///
/// The returned Dataset is ready to be consumed by Charton charts, including
/// `mark_geoshape()` with `alt::path_group("_path_id")`.
///
/// # Errors
///
/// Returns a `ChartonError::Data` if the input string is not valid GeoJSON or if
/// the feature collection cannot be converted.
pub fn geojson_to_dataset(geojson_str: &str) -> Result<Dataset, ChartonError> {
    let geojson = geojson_str
        .parse::<GeoJson>()
        .map_err(|err| ChartonError::Data(format!("GeoJSON parse error: {}", err)))?;

    let features = match geojson {
        GeoJson::FeatureCollection(fc) => fc.features,
        _ => return Err(ChartonError::Data("Only FeatureCollection is supported".into())),
    };

    let mut all_column_names: Vec<String> = Vec::new();
    let mut seen_columns = HashSet::new();
    for feature in &features {
        if let Some(props) = &feature.properties {
            for key in props.keys() {
                if seen_columns.insert(key.clone()) {
                    all_column_names.push(key.clone());
                }
            }
        }
    }

    let mut lon_data: Vec<f64> = Vec::new();
    let mut lat_data: Vec<f64> = Vec::new();
    let mut geometry_type_data: Vec<String> = Vec::new();
    let mut part_id_data: Vec<u32> = Vec::new();
    let mut vertex_order_data: Vec<u32> = Vec::new();
    let mut path_id_data: Vec<String> = Vec::new();

    let mut prop_columns: HashMap<String, Vec<Value>> = HashMap::new();
    for name in &all_column_names {
        prop_columns.insert(name.clone(), Vec::new());
    }

    for (feature_idx, feature) in features.into_iter().enumerate() {
        let props = feature.properties.unwrap_or_default();
        let feature_tag = format!("feature_{}", feature_idx);
        let vertex_count_before = lon_data.len();

        if let Some(geometry) = &feature.geometry {
            extract_vertices_with_meta(
                geometry,
                &feature_tag,
                &mut lon_data,
                &mut lat_data,
                &mut geometry_type_data,
                &mut part_id_data,
                &mut vertex_order_data,
                &mut path_id_data,
            );
        }

        let vertices_added = lon_data.len() - vertex_count_before;
        for name in &all_column_names {
            let value = props.get(name).cloned().unwrap_or(Value::Null);
            let col = prop_columns.get_mut(name).unwrap();
            for _ in 0..vertices_added {
                col.push(value.clone());
            }
        }
    }

    let mut ds = Dataset::new();

    for name in &all_column_names {
        let values = prop_columns.remove(name).unwrap();
        let col_vector = infer_and_build_column(values);
        ds.add_column(name, col_vector)?;
    }

    ds.add_column("_lon", lon_data)?;
    ds.add_column("_lat", lat_data)?;
    ds.add_column("_geometry_type", geometry_type_data)?;
    ds.add_column(
        "_part_id",
        part_id_data.into_iter().map(|v| v as i64).collect::<Vec<_>>(),
    )?;
    ds.add_column(
        "_vertex_order",
        vertex_order_data.into_iter().map(|v| v as i64).collect::<Vec<_>>(),
    )?;
    ds.add_column("_path_id", path_id_data)?;

    Ok(ds)
}

#[cfg(feature = "geo")]
#[derive(PartialEq)]
enum InferredColumnType {
    Float64,
    Boolean,
    String,
}

#[cfg(feature = "geo")]
fn infer_and_build_column(raw_col: Vec<Value>) -> ColumnVector {
    let len = raw_col.len();

    if raw_col.iter().all(|v| v.is_null()) {
        return ColumnVector::String {
            data: vec![String::new(); len],
            validity: Some(vec![0; (len + 7) / 8]),
        };
    }

    let mut inferred_type: Option<InferredColumnType> = None;
    for value in raw_col.iter().filter(|v| !v.is_null()) {
        let value_type = match value {
            Value::Number(_) => InferredColumnType::Float64,
            Value::Bool(_) => InferredColumnType::Boolean,
            _ => InferredColumnType::String,
        };

        inferred_type = match (inferred_type, value_type) {
            (None, next) => Some(next),
            (Some(InferredColumnType::String), _) => Some(InferredColumnType::String),
            (Some(InferredColumnType::Boolean), InferredColumnType::Boolean) => Some(InferredColumnType::Boolean),
            (Some(InferredColumnType::Float64), InferredColumnType::Float64) => Some(InferredColumnType::Float64),
            _ => Some(InferredColumnType::String),
        };

        if inferred_type == Some(InferredColumnType::String) {
            break;
        }
    }

    match inferred_type.unwrap_or(InferredColumnType::String) {
        InferredColumnType::Float64 => build_f64_column(raw_col),
        InferredColumnType::Boolean => build_bool_column(raw_col),
        InferredColumnType::String => build_string_column(raw_col),
    }
}

#[cfg(feature = "geo")]
fn build_f64_column(raw_col: Vec<Value>) -> ColumnVector {
    let len = raw_col.len();
    let mut data = Vec::with_capacity(len);
    let mut validity = vec![0xFFu8; (len + 7) / 8];

    for (i, value) in raw_col.iter().enumerate() {
        if value.is_null() {
            data.push(f64::NAN);
            clear_bit(&mut validity, i);
        } else {
            data.push(value.as_f64().unwrap_or(f64::NAN));
        }
    }

    ColumnVector::Float64 {
        data,
        validity: Some(validity),
    }
}

#[cfg(feature = "geo")]
fn build_bool_column(raw_col: Vec<Value>) -> ColumnVector {
    let len = raw_col.len();
    let mut data = Vec::with_capacity(len);
    let mut validity = vec![0xFFu8; (len + 7) / 8];

    for (i, value) in raw_col.iter().enumerate() {
        if value.is_null() {
            data.push(false);
            clear_bit(&mut validity, i);
        } else {
            data.push(value.as_bool().unwrap_or(false));
        }
    }

    ColumnVector::Boolean {
        data,
        validity: Some(validity),
    }
}

#[cfg(feature = "geo")]
fn build_string_column(raw_col: Vec<Value>) -> ColumnVector {
    let len = raw_col.len();
    let mut data = Vec::with_capacity(len);
    let mut validity = vec![0xFFu8; (len + 7) / 8];

    for (i, value) in raw_col.iter().enumerate() {
        if value.is_null() {
            data.push(String::new());
            clear_bit(&mut validity, i);
        } else if let Some(s) = value.as_str() {
            data.push(s.to_string());
        } else {
            data.push(value.to_string());
        }
    }

    ColumnVector::String {
        data,
        validity: Some(validity),
    }
}

#[cfg(feature = "geo")]
fn clear_bit(validity: &mut [u8], index: usize) {
    validity[index / 8] &= !(1 << (index % 8));
}

#[cfg(feature = "geo")]
fn extract_vertices_with_meta(
    geometry: &geojson::Geometry,
    feature_tag: &str,
    lon_data: &mut Vec<f64>,
    lat_data: &mut Vec<f64>,
    geometry_type_data: &mut Vec<String>,
    part_id_data: &mut Vec<u32>,
    vertex_order_data: &mut Vec<u32>,
    path_id_data: &mut Vec<String>,
) {
    let geom_type = geometry_type_name(&geometry.value);

    match &geometry.value {
        GeometryValue::Polygon { coordinates: rings } => {
            if let Some(outer) = rings.first() {
                for (order, point) in outer.iter().enumerate() {
                    push_vertex(
                        feature_tag,
                        lon_data,
                        lat_data,
                        geometry_type_data,
                        part_id_data,
                        vertex_order_data,
                        path_id_data,
                        point[0],
                        point[1],
                        geom_type,
                        0,
                        order as u32,
                    );
                }
            }
        }
        GeometryValue::MultiPolygon { coordinates: polygons } => {
            for (part_id, polygon) in polygons.iter().enumerate() {
                if let Some(outer) = polygon.first() {
                    for (order, point) in outer.iter().enumerate() {
                        push_vertex(
                            feature_tag,
                            lon_data,
                            lat_data,
                            geometry_type_data,
                            part_id_data,
                            vertex_order_data,
                            path_id_data,
                            point[0],
                            point[1],
                            geom_type,
                            part_id as u32,
                            order as u32,
                        );
                    }
                }
            }
        }
        GeometryValue::Point { coordinates } => {
            push_vertex(
                feature_tag,
                lon_data,
                lat_data,
                geometry_type_data,
                part_id_data,
                vertex_order_data,
                path_id_data,
                coordinates[0],
                coordinates[1],
                geom_type,
                0,
                0,
            );
        }
        GeometryValue::MultiPoint { coordinates: points } => {
            for (i, point) in points.iter().enumerate() {
                push_vertex(
                    feature_tag,
                    lon_data,
                    lat_data,
                    geometry_type_data,
                    part_id_data,
                    vertex_order_data,
                    path_id_data,
                    point[0],
                    point[1],
                    geom_type,
                    i as u32,
                    0,
                );
            }
        }
        GeometryValue::LineString { coordinates: line } => {
            for (order, point) in line.iter().enumerate() {
                push_vertex(
                    feature_tag,
                    lon_data,
                    lat_data,
                    geometry_type_data,
                    part_id_data,
                    vertex_order_data,
                    path_id_data,
                    point[0],
                    point[1],
                    geom_type,
                    0,
                    order as u32,
                );
            }
        }
        GeometryValue::MultiLineString { coordinates: lines } => {
            for (part_id, line) in lines.iter().enumerate() {
                for (order, point) in line.iter().enumerate() {
                    push_vertex(
                        feature_tag,
                        lon_data,
                        lat_data,
                        geometry_type_data,
                        part_id_data,
                        vertex_order_data,
                        path_id_data,
                        point[0],
                        point[1],
                        geom_type,
                        part_id as u32,
                        order as u32,
                    );
                }
            }
        }
        GeometryValue::GeometryCollection { geometries } => {
            for geom in geometries {
                extract_vertices_with_meta(
                    geom,
                    feature_tag,
                    lon_data,
                    lat_data,
                    geometry_type_data,
                    part_id_data,
                    vertex_order_data,
                    path_id_data,
                );
            }
        }
    }
}

#[cfg(feature = "geo")]
fn push_vertex(
    feature_tag: &str,
    lon_data: &mut Vec<f64>,
    lat_data: &mut Vec<f64>,
    geometry_type_data: &mut Vec<String>,
    part_id_data: &mut Vec<u32>,
    vertex_order_data: &mut Vec<u32>,
    path_id_data: &mut Vec<String>,
    lon: f64,
    lat: f64,
    geom_type: &str,
    part_id: u32,
    vertex_order: u32,
) {
    lon_data.push(lon);
    lat_data.push(lat);
    geometry_type_data.push(geom_type.to_string());
    part_id_data.push(part_id);
    vertex_order_data.push(vertex_order);
    path_id_data.push(format!("{}#{}", feature_tag, part_id));
}

#[cfg(feature = "geo")]
fn geometry_type_name(value: &GeometryValue) -> &'static str {
    match value {
        GeometryValue::Point { .. } => "Point",
        GeometryValue::MultiPoint { .. } => "MultiPoint",
        GeometryValue::LineString { .. } => "LineString",
        GeometryValue::MultiLineString { .. } => "MultiLineString",
        GeometryValue::Polygon { .. } => "Polygon",
        GeometryValue::MultiPolygon { .. } => "MultiPolygon",
        GeometryValue::GeometryCollection { .. } => "GeometryCollection",
    }
}
