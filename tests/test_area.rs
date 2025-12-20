use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

#[test]
fn test_area_1() -> Result<(), Box<dyn Error>> {
    // Create sample data similar to the Iowa electricity dataset
    let df = df! [
        "year" => [2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012],
        "net_generation" => [50.0, 70.0, 130.0, 160.0, 180.0, 200.0, 170.0, 140.0, 90.0, 60.0, 80.0, 90.0, 100.0, 110.0, 120.0, 130.0, 140.0, 120.0, 90.0, 70.0],
        "source" => ["Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear"],
    ]?;

    // Create an area chart
    let area_chart = Chart::build(&df)?
        .mark_area()
        .with_area_opacity(0.3)
        .with_area_stroke(Some(SingleColor::new("black")))
        .encode((x("year"), y("net_generation"), color("source")))?;

    // Create a layered chart for the area
    LayeredChart::new()
        .with_size(600, 400)
        .with_title("Iowa Electricity Generation")
        .with_x_label("Year")
        .with_y_label("Net Generation")
        .add_layer(area_chart)
        .save("./tests/area_1.svg")?;

    Ok(())
}

#[test]
fn test_area_2() -> Result<(), Box<dyn Error>> {
    // Create sample data for demonstration
    let df = df! [
        "IMDB_Rating" => [
            7.1, 6.8, 7.5, 8.2, 6.9, 7.3, 7.7, 8.0, 6.5, 7.2,
            7.9, 6.7, 7.4, 7.8, 8.1, 6.6, 7.0, 7.6, 8.3, 6.4,
            7.2, 7.5, 7.9, 6.8, 7.1, 7.7, 8.0, 6.9, 7.3, 7.6,
            7.4, 7.8, 8.2, 6.7, 7.0, 7.5, 7.9, 6.6, 7.1, 7.7
        ],
        "Genre" => [
            "Action", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy",
            "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama",
            "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action",
            "Comedy", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy"
        ],
        "Year" => [
            2010, 2011, 2010, 2012, 2011, 2010, 2013, 2012, 2011, 2010,
            2014, 2013, 2012, 2011, 2010, 2015, 2014, 2013, 2012, 2011,
            2010, 2016, 2015, 2014, 2013, 2012, 2011, 2010, 2017, 2016,
            2015, 2014, 2013, 2012, 2011, 2010, 2018, 2017, 2016, 2015
        ]
    ]?;

    let chart = Chart::build(&df)?
        .transform_density(
            DensityTransform::new("IMDB_Rating")
                .with_as("IMDB_Rating", "cumulative_density")
                .with_cumulative(true),
        )?
        .mark_area()
        .encode((x("IMDB_Rating"), y("cumulative_density")))?
        .with_area_color(Some(SingleColor::new("purple")))
        .with_area_opacity(0.3);

    LayeredChart::new()
        .with_size(600, 400)
        .with_title("Cumulative Density Estimation")
        .with_x_label("IMDB Rating")
        .with_y_label("Cumulative Density")
        .add_layer(chart)
        .save("./tests/area_2.svg")?;

    Ok(())
}
