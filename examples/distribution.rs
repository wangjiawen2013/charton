use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
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
        .with_title("Cumulative Density Estimation")
        .with_x_label("IMDB Rating")
        .with_y_label("Cumulative Density")
        .add_layer(chart)
        .save("./examples/distribution.svg")?;

    Ok(())
}
