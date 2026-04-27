use charton::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut categories = Vec::new();
    let mut outcomes = Vec::new();
    let mut treatments = Vec::new();

    let configs = [
        ("Cohort A", 45.0, 12.0),
        ("Cohort B", 70.0, 6.0),
        ("Cohort C", 35.0, 18.0),
    ];

    let treatment_types = ["Placebo", "Active"];

    // Use a simple deterministic counter to simulate "randomness"
    let mut seed: u32 = 42;

    for (label, mean, std_dev) in configs {
        for i in 0..150 {
            categories.push(label.to_string());

            // 1. Deterministic Pseudo-random using LCG
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let raw_rand = (seed & 0x7FFFFFFF) as f64 / 2147483647.0;

            // 2. Simple Box-Muller transform to simulate Normal Distribution
            // This creates the "cluster" effect needed to show off Beeswarm
            let u1 = raw_rand;
            let u2 = ((i as f64 * 0.1).sin() + 1.0) / 2.0; // Another "random" seed
            let z0 = (-2.0 * u1.ln().max(-10.0)).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();

            outcomes.push(mean + z0 * std_dev);

            // 3. Deterministic sub-group assignment
            let sub_idx = (i % 2) as usize;
            treatments.push(treatment_types[sub_idx].to_string());
        }
    }

    // 2. Render the Chart
    let beeswarm = chart!(categories, outcomes, treatments)?
        .mark_point()?
        .configure_point(|m| m.with_layout("beeswarm").with_size(2.5))
        .encode((
            alt::x("categories"),
            alt::y("outcomes"),
            alt::color("treatments"),
        ))?;
    
    let boxplot = chart!(categories, outcomes, treatments)?
        .mark_boxplot()?.configure_boxplot(|b| b.with_outliers(false).with_opacity(0.0).with_stroke_width(1.5))
        .encode((
            alt::x("categories"),
            alt::y("outcomes"),
            alt::color("treatments"),
        ))?;
    
    beeswarm.and(boxplot).save("docs/src/images/beeswarm.svg")?;

    Ok(())
}
