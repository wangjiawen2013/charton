use charton::prelude::*;
use rand::prelude::*;
use rand_distr::{Distribution, Normal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize the random number generator (rand 0.9 syntax)
    let mut rng = rand::rng();

    let mut categories = Vec::new(); // X-axis: Major groups
    let mut outcomes = Vec::new(); // Y-axis: Quantitative results
    let mut treatments = Vec::new(); // Color: Discrete sub-groups (for Dodge/Grouping)

    let configs = [
        ("Cohort A", 45.0, 12.0),
        ("Cohort B", 70.0, 6.0), // Narrow std_dev creates high density for Beeswarm
        ("Cohort C", 35.0, 18.0),
    ];

    // Define discrete sub-categories
    let treatment_types = ["Placebo", "Active"];

    for (label, mean, std_dev) in configs {
        let dist = Normal::new(mean, std_dev)?;

        // Generate 150 points per cohort to showcase density-based layout
        for _ in 0..150 {
            categories.push(label.to_string());
            outcomes.push(dist.sample(&mut rng));

            // Randomly assign a discrete label for categorical color mapping
            let sub_idx = rng.random_range(0..treatment_types.len());
            treatments.push(treatment_types[sub_idx].to_string());
        }
    }

    // 2. Render the Chart
    // The engine's transform_point_data will detect "treatments" as Discrete,
    // automatically applying both Dodge (side-by-side) and Beeswarm (collision) logic.
    chart!(categories, outcomes, treatments)?
        .mark_point()?
        .encode((
            alt::x("categories"),
            alt::y("outcomes"),
            alt::color("treatments"),
        ))?
        .save("examples/beeswarm_discrete_final.svg")?;

    println!("✨ Beeswarm example with discrete grouping (rand 0.9) generated successfully!");
    Ok(())
}
