//! Geographic polygons with holes.

#![cfg(feature = "geo")]

use charton::prelude::*;
use std::error::Error;

#[test]
fn geojson_holes_become_even_odd_subpaths() -> Result<(), Box<dyn Error>> {
    let geojson = r#"{
      "type": "FeatureCollection",
      "features": [{
        "type": "Feature",
        "properties": { "name": "ring" },
        "geometry": {
          "type": "Polygon",
          "coordinates": [
            [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0], [0.0, 0.0]],
            [[1.0, 1.0], [1.0, 3.0], [3.0, 3.0], [3.0, 1.0], [1.0, 1.0]]
          ]
        }
      }]
    }"#;

    let ds = geojson_to_dataset(geojson)?;
    let mut ring_ids: Vec<f64> = (0..ds.height())
        .filter_map(|i| ds.get("_ring_id", i).to_f64())
        .collect();
    ring_ids.sort_by(f64::total_cmp);
    ring_ids.dedup();
    assert_eq!(
        ring_ids.len(),
        2,
        "both the outer ring and the hole are kept"
    );

    let svg = Chart::build(ds)?
        .mark_polygon()?
        .encode((
            alt::x("_lon"),
            alt::y("_lat"),
            alt::path_group("_path_id").with_subpath("_ring_id"),
        ))?
        .with_coord(CoordSystem::Geo)
        .to_svg()?;

    assert!(
        svg.contains(r#"fill-rule="evenodd""#),
        "closed paths must use the even-odd fill rule: {svg}"
    );
    let path = svg
        .split(r#"<path d=""#)
        .nth(1)
        .expect("the polygon path is present");
    let subpaths = path.matches('M').count();
    assert!(
        subpaths >= 2,
        "the outer ring and the hole must be separate subpaths, got {subpaths}: {path}"
    );

    Ok(())
}
