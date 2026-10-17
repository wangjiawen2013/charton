//! Font-path reproduction for the CPU raster (PNG) backend.
//!
//! Renders the issue's minimal skeleton and saves the PNG under
//! `target/example-output/font-issue/`.
//!
//! Run the default chart with:
//!
//! ```text
//! cargo run --example chinese --features png
//! ```
//!
//! Or render a single chart by passing a title and an optional family:
//!
//! ```text
//! cargo run --example chinese --features png -- "销售额（m²）" "Hiragino Sans GB"
//! ```
//!

use charton::prelude::*;

fn render(title: &str, family: Option<&str>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let ds = Dataset::new()
        .with_column("x", vec![1.0, 2.0, 3.0])?
        .with_column("y", vec![2.0, 3.0, 5.0])?;
    let lc: LayeredChart = Chart::build(ds)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_title(title)
        .with_size(400, 200)
        .into();
    let lc = match family {
        Some(f) => lc.configure_theme(|t| {
            t.with_title_family(f)
                .with_label_family(f)
                .with_tick_label_family(f)
                .with_legend_label_family(f)
        }),
        None => lc,
    };
    Ok(lc.to_png()?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::Path::new("target/example-output/font-issue");
    std::fs::create_dir_all(out_dir)?;

    // Default chart.
    let png = render("销售额（m²）", Some("Arial"))?;
    std::fs::write(out_dir.join("explicit-family-chinese.png"), png)?;

    println!("Saved PNGs to {}", out_dir.display());
    Ok(())
}
