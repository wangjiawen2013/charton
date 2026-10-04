use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Create sample data for demonstration
    let imdb_rating = [
        7.1, 6.8, 7.5, 8.2, 6.9, 7.3, 7.7, 8.0, 6.5, 7.2, 7.9, 6.7, 7.4, 7.8, 8.1, 6.6, 7.0, 7.6,
        8.3, 6.4, 7.2, 7.5, 7.9, 6.8, 7.1, 7.7, 8.0, 6.9, 7.3, 7.6, 7.4, 7.8, 8.2, 6.7, 7.0, 7.5,
        7.9, 6.6, 7.1, 7.7,
    ];
    let genre = [
        "Action", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action",
        "Comedy", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action",
        "Comedy", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action",
        "Comedy", "Drama", "Action", "Comedy", "Drama", "Action", "Comedy", "Drama", "Action",
        "Comedy", "Drama", "Action", "Comedy",
    ];
    let year = [
        2010, 2011, 2010, 2012, 2011, 2010, 2013, 2012, 2011, 2010, 2014, 2013, 2012, 2011, 2010,
        2015, 2014, 2013, 2012, 2011, 2010, 2016, 2015, 2014, 2013, 2012, 2011, 2010, 2017, 2016,
        2015, 2014, 2013, 2012, 2011, 2010, 2018, 2017, 2016, 2015,
    ];

    chart!(imdb_rating, genre, year)?
        .mark_density()?
        .configure_density(|density| {
            density
                .with_cumulative(true)
                .with_color("purple")
                .with_opacity(0.3)
        })
        .encode(alt::x("imdb_rating"))?
        .with_title("Cumulative Density Estimation")
        .with_x_label("IMDB Rating")
        .with_y_label("Cumulative Density")
        .save("target/example-output/distribution_gpu.png")?;

    Ok(())
}
