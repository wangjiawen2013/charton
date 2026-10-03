//! Geographic polygons smoke test (was `examples/geo_islands.rs`).

use charton::prelude::*;
use std::error::Error;

#[test]
fn geo_islands_with_points_and_labels() -> Result<(), Box<dyn Error>> {
    // Simplified outlines of Hainan and Taiwan. Rows sharing a "region" form
    // one polygon.
    let hainan_lon = vec![
        108.62, 109.10, 109.80, 110.40, 110.80, 111.00, 110.60, 110.20, 109.60, 109.00, 108.62,
    ];
    let hainan_lat = vec![
        18.20, 18.80, 19.20, 19.80, 20.00, 18.80, 18.20, 18.00, 18.10, 18.20, 18.20,
    ];
    let hainan_pop = 10.08;

    let taiwan_lon = vec![
        121.00, 121.50, 122.00, 121.80, 121.00, 120.50, 120.10, 120.20, 120.60, 121.00,
    ];
    let taiwan_lat = vec![
        25.30, 25.10, 24.00, 22.00, 21.80, 21.90, 22.50, 23.50, 24.80, 25.30,
    ];
    let taiwan_pop = 23.57;

    let n_hainan = hainan_lon.len();
    let n_taiwan = taiwan_lon.len();
    let total = n_hainan + n_taiwan;

    let mut lon = Vec::with_capacity(total);
    let mut lat = Vec::with_capacity(total);
    let mut region = Vec::with_capacity(total);
    let mut population = Vec::with_capacity(total);

    for i in 0..n_hainan {
        lon.push(hainan_lon[i]);
        lat.push(hainan_lat[i]);
        region.push("Hainan");
        population.push(hainan_pop);
    }
    for i in 0..n_taiwan {
        lon.push(taiwan_lon[i]);
        lat.push(taiwan_lat[i]);
        region.push("Taiwan");
        population.push(taiwan_pop);
    }

    let ds = Dataset::new()
        .with_column("lon", lon)?
        .with_column("lat", lat)?
        .with_column("region", region)?
        .with_column("population", population)?;

    let capitals = Dataset::new()
        .with_column("lon", vec![110.35, 121.57])? // Haikou, Taipei
        .with_column("lat", vec![20.02, 25.03])?
        .with_column("name", vec!["Haikou", "Taipei"])?;

    let geo_layer = Chart::build(ds)?
        .mark_geoshape()?
        .encode((
            alt::x("lon"),
            alt::y("lat"),
            alt::path_group("region"),
            alt::color("population"),
        ))?
        .configure_geoshape(|m| m.with_stroke("#333333").with_stroke_width(0.5));

    let point_layer = Chart::build(capitals.clone())?
        .mark_point()?
        .encode((alt::x("lon"), alt::y("lat")))?
        .configure_point(|m| m.with_color("black").with_size(6.0));

    let label_layer = Chart::build(capitals)?
        .mark_text()?
        .encode((alt::x("lon"), alt::y("lat"), alt::text("name")))?
        .configure_text(|m| m.with_color("black").with_size(11.0));

    geo_layer
        .and(point_layer)
        .and(label_layer)
        .with_title("Hainan & Taiwan — Equal Earth Projection")
        .with_x_label("Longitude")
        .with_y_label("Latitude")
        .with_coord(CoordSystem::Geo)
        .with_grid(true)
        .save("./target/test-output/geo_islands.svg")?;

    Ok(())
}
