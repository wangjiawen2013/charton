use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // World map geojson was downloaed from "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_0_countries.geojson";
    let world_map = std::path::Path::new("assets/ne_110m_admin_0_countries.geojson");

    let geojson_str = std::fs::read_to_string(world_map)?;

    let ds = geojson_to_dataset(&geojson_str)?;

    println!("Data loaded successfully!");
    println!("Total vertices: {}", ds.height());
    println!("Total columns: {}", ds.width());

    // Preview the first 10 vertices
    println!("\nFirst 10 vertices:");
    let preview = ds.select(&["NAME", "_lon", "_lat", "POP_EST", "_path_id"])?;
    println!("{:#?}", preview);

    // ==================== Draw the world map ====================
    println!("\nDrawing the world map...");

    let geo_layer = Chart::build(ds)?.mark_geoshape()?.encode((
        alt::x("_lon"),
        alt::y("_lat"),
        alt::path_group("_path_id"),
        //alt::color("POP_EST")
    ))?;

    geo_layer
        .with_x_label("Longitude")
        .with_y_label("Latitude")
        .with_coord(CoordSystem::Geo)
        .with_grid(true)
        .with_x_domain(-140.0, 150.0)
        //.with_size(1000, 500)
        .save("docs/src/images/world_map.svg")?;

    Ok(())
}
