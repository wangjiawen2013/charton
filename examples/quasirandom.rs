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

    // Deterministic counter used to build a repeatable normal sample.
    let mut seed: u32 = 42;

    for (label, mean, std_dev) in configs {
        for i in 0..150 {
            categories.push(label.to_string());

            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let raw_rand = (seed & 0x7FFFFFFF) as f64 / 2147483647.0;

            let u1 = raw_rand;
            let u2 = ((i as f64 * 0.1).sin() + 1.0) / 2.0;
            let z0 = (-2.0 * u1.ln().max(-10.0)).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();

            outcomes.push(mean + z0 * std_dev);

            let sub_idx = (i % 2) as usize;
            treatments.push(treatment_types[sub_idx].to_string());
        }
    }

    // The default method spreads the cloud with a low discrepancy sequence,
    // which gives the smooth violin outline.
    chart!(categories, outcomes, treatments)?
        .mark_point()?
        .configure_point(|m| m.with_layout("quasirandom").with_size(1.5))
        .encode((
            alt::x("categories"),
            alt::y("outcomes"),
            alt::color("treatments"),
        ))?
        .save("docs/src/images/quasirandom.svg")?;

    // The pseudorandom method keeps the density scaling but pairs the points
    // with a random-like sequence, so the outline is noisier.
    chart!(categories, outcomes, treatments)?
        .mark_point()?
        .configure_point(|m| {
            m.with_layout("quasirandom")
                .with_quasirandom_method("pseudorandom")
                .with_size(1.5)
        })
        .encode((
            alt::x("categories"),
            alt::y("outcomes"),
            alt::color("treatments"),
        ))?
        .save("target/example-output/quasirandom_pseudorandom.svg")?;

    Ok(())
}
