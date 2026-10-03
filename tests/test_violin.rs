use charton::prelude::*;
use std::error::Error;

/// A single violin is a closed, symmetric polygon built from a density curve
/// and the general band geometry.
#[test]
fn test_violin_1() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        .transform_density(
            DensityTransform::new("sepal_length")
                .with_as("sepal_length", "density")
                .with_trim(true),
        )?
        .transform_band(BandTransform::new("sepal_length", "density"))?
        .mark_polygon()?
        .configure_geoshape(|mark| mark.with_fill("#7fb3d5").with_stroke("#2c3e50"))
        .encode((alt::x("x"), alt::y("y"), alt::path_group("path_group")))?
        .save("./target/test-output/violin_1.svg")?;

    Ok(())
}

/// Dodged violins sit side by side within each category.
#[test]
fn test_violin_2() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    chart!(&penguins)?
        .transform_density(
            DensityTransform::new("Body Mass (g)")
                .with_as("Body Mass (g)", "density")
                .with_groupbys(["Sex", "Species"])
                .with_trim(true),
        )?
        .transform_band(
            BandTransform::new("Body Mass (g)", "density")
                .with_center("Sex")
                .with_group("Species")
                .with_position(Position::dodge()),
        )?
        .mark_polygon()?
        .configure_geoshape(|mark| mark.with_fill("#95a5a6").with_stroke("#2c3e50"))
        .encode((
            alt::x("x").with_category_labels("Sex"),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("Species"),
        ))?
        .save("./target/test-output/violin_2.svg")?;

    Ok(())
}

/// Faceted violins built from the existing density transform and area mark,
/// held symmetric by the `Mirror` stack mode (the Vega-Lite recipe).
#[test]
fn test_violin_3() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    chart!(&penguins)?
        .transform_density(
            DensityTransform::new("Body Mass (g)")
                .with_as("Body Mass (g)", "density")
                .with_groupbys(["Species"])
                .with_trim(true),
        )?
        .mark_area()?
        .configure_area(|a| a.with_opacity(0.7).with_stroke("#7e5109"))
        .encode((
            alt::x("Body Mass (g)"),
            alt::y("density").with_stack("mirror"),
            alt::color("Species"),
        ))?
        .facet(FacetSpec::wrap("Species").with_columns(3))
        .coord_flip()
        .save("./target/test-output/violin_3.svg")?;

    Ok(())
}

/// The inner box is a second polygon layer built from the same observations.
#[test]
fn test_violin_4() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    let outline = chart!(&penguins)?
        .transform_density(
            DensityTransform::new("Body Mass (g)")
                .with_as("Body Mass (g)", "density")
                .with_groupbys(["Sex", "Species"])
                .with_trim(true),
        )?
        .transform_band(
            BandTransform::new("Body Mass (g)", "density")
                .with_center("Sex")
                .with_group("Species")
                .with_position(Position::dodge())
                .with_scale(BandScale::PerGroup),
        )?
        .mark_polygon()?
        .configure_geoshape(|mark| mark.with_fill("#d6eaf8").with_stroke("#2c3e50"))
        .encode((
            alt::x("x").with_category_labels("Sex"),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("Species"),
        ))?;

    let inner_box = chart!(&penguins)?
        .transform_quantile_box(
            QuantileBoxTransform::new("Body Mass (g)")
                .with_category("Sex")
                .with_group("Species")
                .with_position(Position::dodge()),
        )?
        .mark_polygon()?
        .configure_geoshape(|mark| mark.with_fill("white").with_stroke("black"))
        .encode((
            alt::x("x").with_category_labels("Sex"),
            alt::y("y"),
            alt::path_group("path_group"),
        ))?;

    outline
        .and(inner_box)
        .save("./target/test-output/violin_4.svg")?;

    Ok(())
}

/// The pure Vega-Lite recipe: the centered stack is computed inside each panel,
/// so `density + area + stack:"center" + facet` draws a symmetric violin even
/// without the `Mirror` mode. This is the per-facet statistic in action.
#[test]
fn test_violin_5() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        .transform_density(
            DensityTransform::new("sepal_length")
                .with_as("sepal_length", "density")
                .with_groupbys(["species"])
                .with_trim(true),
        )?
        .mark_area()?
        .configure_area(|a| a.with_opacity(0.7).with_stroke("black"))
        .encode((
            alt::x("sepal_length"),
            alt::y("density").with_stack("center"),
            alt::color("species"),
        ))?
        .facet(FacetSpec::wrap("species").with_columns(3))
        .coord_flip()
        .save("./target/test-output/violin_5.svg")?;

    Ok(())
}

/// Split violin: two groups share one centre line, one half per group.
#[test]
fn test_violin_split() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    chart!(&penguins)?
        .transform_density(
            DensityTransform::new("Body Mass (g)")
                .with_as("Body Mass (g)", "density")
                .with_groupbys(["Species", "Sex"])
                .with_trim(true),
        )?
        .transform_band(
            BandTransform::new("Body Mass (g)", "density")
                .with_center("Species")
                .with_group("Sex")
                .with_split(true)
                .with_scale(BandScale::PerGroup),
        )?
        .mark_polygon()?
        .configure_geoshape(|mark| mark.with_fill("#95a5a6").with_stroke("#2c3e50"))
        .encode((
            alt::x("x").with_category_labels("Species"),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("Sex"),
        ))?
        .save("./target/test-output/violin_6.svg")?;

    Ok(())
}

/// Raincloud: violin outline + inner box + jittered observations, all as layers.
#[test]
fn test_violin_raincloud() -> Result<(), Box<dyn Error>> {
    let penguins = load_dataset("penguins")?;

    let violin = chart!(&penguins)?
        .transform_density(
            DensityTransform::new("Body Mass (g)")
                .with_as("Body Mass (g)", "density")
                .with_groupbys(["Species"])
                .with_trim(true),
        )?
        .transform_band(
            BandTransform::new("Body Mass (g)", "density")
                .with_center("Species")
                .with_scale(BandScale::PerGroup),
        )?
        .mark_polygon()?
        .configure_geoshape(|mark| mark.with_fill("#d6eaf8").with_stroke("#2c3e50"))
        .encode((
            alt::x("x").with_category_labels("Species"),
            alt::y("y"),
            alt::path_group("path_group"),
        ))?;

    let inner_box = chart!(&penguins)?
        .transform_quantile_box(
            QuantileBoxTransform::new("Body Mass (g)").with_category("Species"),
        )?
        .mark_polygon()?
        .configure_geoshape(|mark| mark.with_fill("white").with_stroke("black"))
        .encode((
            alt::x("x").with_category_labels("Species"),
            alt::y("y"),
            alt::path_group("path_group"),
        ))?;

    let rain = chart!(&penguins)?
        .mark_point()?
        .configure_point(|point| {
            point
                .with_layout("jitter")
                .with_size(2.5)
                .with_opacity(0.55)
        })
        .encode((alt::x("Species"), alt::y("Body Mass (g)")))?;

    violin
        .and(inner_box)
        .and(rain)
        .save("./target/test-output/violin_7.svg")?;

    Ok(())
}
