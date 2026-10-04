//! Case study: exploratory feature analysis before training a classifier.
//!
//! Two views you would actually look at on the Iris dataset:
//!
//! 1. the **class-conditional distribution** of a feature (is it separable?),
//! 2. a **scatter of two features**, coloured by class, to see the decision
//!    boundary a linear model would find.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    // 1. Class-conditional density of petal length.
    chart!(&iris)?
        .mark_density()?
        .configure_density(|density| density.with_opacity(0.5))
        .encode((alt::x("petal_length"), alt::color("species")))?
        .with_title("Petal length by species")
        .with_x_label("Petal length (cm)")
        .with_y_label("Density")
        .save("docs/src/images/case_data_science_density.svg")?;

    // 2. Two features, coloured by class. Setosa is linearly separable from the
    //    other two; versicolor and virginica overlap.
    chart!(&iris)?
        .mark_point()?
        .configure_point(|point| point.with_size(40.0).with_opacity(0.8))
        .encode((
            alt::x("sepal_length"),
            alt::y("petal_length"),
            alt::color("species"),
        ))?
        .with_title("Sepal vs petal length")
        .with_x_label("Sepal length (cm)")
        .with_y_label("Petal length (cm)")
        .save("docs/src/images/case_data_science_scatter.svg")?;

    println!("Saved case_data_science_density.svg and case_data_science_scatter.svg");
    Ok(())
}
