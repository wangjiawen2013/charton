use ahash::AHashMap;

/// Estimates the pixel width of `text` drawn at `font_size` em.
///
/// Layout needs a text width before a backend is chosen (title and legend
/// rectangles, tick spacing, label rotation), and the SVG backend cannot
/// measure text at all, so this has to be a font-independent estimate. It is
/// deliberately an over-estimate for wide scripts: a CJK ideograph advances one
/// em, and under-estimating a label clips it, while over-estimating only leaves
/// a little padding.
pub(crate) fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    let mut wide_chars = 0;
    let mut narrow_chars = 0;
    let mut uppercase_chars = 0;
    let mut other_chars = 0;

    for c in text.chars() {
        if is_wide(c) {
            wide_chars += 1;
        } else if matches!(
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

    (wide_chars as f64
        + narrow_chars as f64 * 0.3
        + uppercase_chars as f64 * 0.65
        + other_chars as f64 * 0.55)
        * font_size
}

/// Whether `c` is East Asian Wide or Fullwidth, i.e. advances about one em in
/// the fonts that contain it.
///
/// Ranges follow the Unicode East Asian Width property for the blocks that chart
/// text actually uses. Halfwidth katakana (`U+FF61..=U+FFDC`) is intentionally
/// not included; it is narrow and falls into the generic bucket.
const fn is_wide(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x115F      // Hangul Jamo
        | 0x2E80..=0x303E    // CJK radicals, Kangxi, CJK symbols and punctuation
        | 0x3041..=0x33FF    // Kana, Bopomofo, Hangul compatibility, CJK compatibility
        | 0x3400..=0x4DBF    // CJK Unified Ideographs Extension A
        | 0x4E00..=0x9FFF    // CJK Unified Ideographs
        | 0xA000..=0xA4CF    // Yi
        | 0xAC00..=0xD7A3    // Hangul syllables
        | 0xF900..=0xFAFF    // CJK compatibility ideographs
        | 0xFE10..=0xFE19    // Vertical forms
        | 0xFE30..=0xFE4F    // CJK compatibility forms
        | 0xFF01..=0xFF60    // Fullwidth forms
        | 0xFFE0..=0xFFE6    // Fullwidth signs
        | 0x1F300..=0x1F9FF  // Emoji
        | 0x20000..=0x3FFFD  // CJK Extensions B and beyond
    )
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
/// - automatic generation of helper columns: `_lon`, `_lat`, `_geometry_type`, `_part_id`, `_ring_id`, `_vertex_order`, and `_path_id`
///
/// The returned Dataset is ready to be consumed by Charton charts, including
/// `mark_polygon()` with `alt::path_group("_path_id")` and
/// `CoordSystem::Geo`.
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
        _ => {
            return Err(ChartonError::Data(
                "Only FeatureCollection is supported".into(),
            ));
        }
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
    let mut ring_id_data: Vec<u32> = Vec::new();
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
                &mut ring_id_data,
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
        part_id_data
            .into_iter()
            .map(|v| v as i64)
            .collect::<Vec<_>>(),
    )?;
    ds.add_column(
        "_ring_id",
        ring_id_data
            .into_iter()
            .map(|v| v as i64)
            .collect::<Vec<_>>(),
    )?;
    ds.add_column(
        "_vertex_order",
        vertex_order_data
            .into_iter()
            .map(|v| v as i64)
            .collect::<Vec<_>>(),
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
            validity: Some(vec![0; len.div_ceil(8)]),
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
            (Some(InferredColumnType::Boolean), InferredColumnType::Boolean) => {
                Some(InferredColumnType::Boolean)
            }
            (Some(InferredColumnType::Float64), InferredColumnType::Float64) => {
                Some(InferredColumnType::Float64)
            }
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
    let mut validity = vec![0xFFu8; len.div_ceil(8)];

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
    let mut validity = vec![0xFFu8; len.div_ceil(8)];

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
    let mut validity = vec![0xFFu8; len.div_ceil(8)];

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
const fn clear_bit(validity: &mut [u8], index: usize) {
    validity[index / 8] &= !(1 << (index % 8));
}

#[cfg(feature = "geo")]
#[allow(clippy::too_many_arguments)]
fn extract_vertices_with_meta(
    geometry: &geojson::Geometry,
    feature_tag: &str,
    lon_data: &mut Vec<f64>,
    lat_data: &mut Vec<f64>,
    geometry_type_data: &mut Vec<String>,
    part_id_data: &mut Vec<u32>,
    ring_id_data: &mut Vec<u32>,
    vertex_order_data: &mut Vec<u32>,
    path_id_data: &mut Vec<String>,
) {
    let geom_type = geometry_type_name(&geometry.value);

    match &geometry.value {
        GeometryValue::Polygon { coordinates: rings } => {
            for (ring_id, ring) in rings.iter().enumerate() {
                for (order, point) in ring.iter().enumerate() {
                    push_vertex(
                        feature_tag,
                        lon_data,
                        lat_data,
                        geometry_type_data,
                        part_id_data,
                        ring_id_data,
                        vertex_order_data,
                        path_id_data,
                        point[0],
                        point[1],
                        geom_type,
                        0,
                        ring_id as u32,
                        order as u32,
                    );
                }
            }
        }
        GeometryValue::MultiPolygon {
            coordinates: polygons,
        } => {
            for (part_id, polygon) in polygons.iter().enumerate() {
                for (ring_id, ring) in polygon.iter().enumerate() {
                    for (order, point) in ring.iter().enumerate() {
                        push_vertex(
                            feature_tag,
                            lon_data,
                            lat_data,
                            geometry_type_data,
                            part_id_data,
                            ring_id_data,
                            vertex_order_data,
                            path_id_data,
                            point[0],
                            point[1],
                            geom_type,
                            part_id as u32,
                            ring_id as u32,
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
                ring_id_data,
                vertex_order_data,
                path_id_data,
                coordinates[0],
                coordinates[1],
                geom_type,
                0,
                0,
                0,
            );
        }
        GeometryValue::MultiPoint {
            coordinates: points,
        } => {
            for (i, point) in points.iter().enumerate() {
                push_vertex(
                    feature_tag,
                    lon_data,
                    lat_data,
                    geometry_type_data,
                    part_id_data,
                    ring_id_data,
                    vertex_order_data,
                    path_id_data,
                    point[0],
                    point[1],
                    geom_type,
                    i as u32,
                    0,
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
                    ring_id_data,
                    vertex_order_data,
                    path_id_data,
                    point[0],
                    point[1],
                    geom_type,
                    0,
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
                        ring_id_data,
                        vertex_order_data,
                        path_id_data,
                        point[0],
                        point[1],
                        geom_type,
                        part_id as u32,
                        0,
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
                    ring_id_data,
                    vertex_order_data,
                    path_id_data,
                );
            }
        }
    }
}

#[cfg(feature = "geo")]
#[allow(clippy::too_many_arguments)]
fn push_vertex(
    feature_tag: &str,
    lon_data: &mut Vec<f64>,
    lat_data: &mut Vec<f64>,
    geometry_type_data: &mut Vec<String>,
    part_id_data: &mut Vec<u32>,
    ring_id_data: &mut Vec<u32>,
    vertex_order_data: &mut Vec<u32>,
    path_id_data: &mut Vec<String>,
    lon: f64,
    lat: f64,
    geom_type: &str,
    part_id: u32,
    ring_id: u32,
    vertex_order: u32,
) {
    lon_data.push(lon);
    lat_data.push(lat);
    geometry_type_data.push(geom_type.to_string());
    part_id_data.push(part_id);
    ring_id_data.push(ring_id);
    vertex_order_data.push(vertex_order);
    path_id_data.push(format!("{}#{}", feature_tag, part_id));
}

#[cfg(feature = "geo")]
const fn geometry_type_name(value: &GeometryValue) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::estimate_text_width;

    /// Wide scripts advance about one em per character, so a CJK label must be
    /// measured at least that wide; under-estimating it is what clipped Chinese
    /// legend entries and titles.
    #[test]
    fn wide_characters_are_measured_at_one_em() {
        let size = 10.0;
        assert!((estimate_text_width("销售额趋势", size) - 5.0 * size).abs() < 1e-9);
        // Two fullwidth parentheses at 1em, plus two Latin-ish glyphs at 0.55em.
        assert!((estimate_text_width("（m²）", size) - 3.1 * size).abs() < 1e-9);
        // Latin keeps its narrower estimate.
        assert!(estimate_text_width("WWWW", size) < 4.0 * size);
    }
}
