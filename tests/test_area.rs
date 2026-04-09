use charton::prelude::*;
use std::error::Error;

#[test]
fn test_area_1() -> Result<(), Box<dyn Error>> {
    // Create sample data similar to the Iowa electricity dataset
    let year = vec![
        2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2003, 2004, 2005, 2006, 2007,
        2008, 2009, 2010, 2011, 2012,
    ];
    let net_generation = vec![
        50.0, 70.0, 130.0, 160.0, 180.0, 200.0, 170.0, 140.0, 90.0, 60.0, 80.0, 90.0, 100.0, 110.0,
        120.0, 130.0, 140.0, 120.0, 90.0, 70.0,
    ];
    let source = vec![
        "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Coal", "Nuclear",
        "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear", "Nuclear",
        "Nuclear",
    ];

    // Create an area chart
    chart!(year, net_generation, source)?
        .mark_area()?
        .configure_area(|a| a.with_opacity(0.3).with_stroke("black"))
        .encode((x("year"), y("net_generation"), color("source")))?
        .with_size(600, 400)
        .with_title("Iowa Electricity Generation")
        .with_x_label("Year")
        .with_y_label("Net Generation")
        .save("./tests/area_1.svg")?;

    Ok(())
}

#[test]
fn test_area_2() -> Result<(), Box<dyn Error>> {
    // Create sample data for demonstration
    let imdb_rating = vec![
        7.1, 6.8, 7.5, 8.2, 6.9, 7.3, 7.7, 8.0, 6.5, 7.2, 7.9, 6.7, 7.4, 7.8, 8.1, 6.6, 7.0, 7.6,
        8.3, 6.4, 7.2, 7.5, 7.9, 6.8, 7.1, 7.7, 8.0, 6.9, 7.3, 7.6, 7.4, 7.8, 8.2, 6.7, 7.0, 7.5,
        7.9, 6.6, 7.1, 7.7,
    ];
    let genre = vec![
        "Action", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action",
        "Comedy", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action",
        "Comedy", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action",
        "Comedy", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action",
        "Comedy", "Drama", "Action", "Comedy",
    ];
    let year = vec![
        2010, 2011, 2010, 2012, 2011, 2010, 2013, 2012, 2011, 2010, 2014, 2013, 2012, 2011, 2010,
        2015, 2014, 2013, 2012, 2011, 2010, 2016, 2015, 2014, 2013, 2012, 2011, 2010, 2017, 2016,
        2015, 2014, 2013, 2012, 2011, 2010, 2018, 2017, 2016, 2015,
    ];

    let chart = chart!(imdb_rating, genre, year)?
        .transform_density(
            DensityTransform::new("imdb_rating")
                .with_as("imdb_rating", "cumulative_density")
                .with_cumulative(true),
        )?
        .mark_area()?
        .configure_area(|a| a.with_color("purple").with_opacity(0.3))
        .encode((x("imdb_rating"), y("cumulative_density")))?;

    chart
        .with_size(600, 400)
        .with_title("Cumulative Density Estimation")
        .with_x_label("IMDB Rating")
        .with_y_label("Cumulative Density")
        .save("./tests/area_2.svg")?;

    Ok(())
}
