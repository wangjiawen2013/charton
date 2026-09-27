use charton::prelude::*;
use std::error::Error;

#[test]
fn test_quasirandom_1() -> Result<(), Box<dyn Error>> {
    // Three cohorts with a wavy, density varying value so that the cloud has
    // both crowded and sparse regions.
    let cohort: Vec<&str> = (0..240).map(|i| ["A", "B", "C"][i % 3]).collect();
    let value: Vec<f64> = (0..240)
        .map(|i| {
            let t = i as f64;
            (t * 0.37).sin() * 12.0 + (t * 0.11).cos() * 4.0
        })
        .collect();
    let treatment: Vec<&str> = (0..240)
        .map(|i| if i % 2 == 0 { "Placebo" } else { "Active" })
        .collect();

    chart!(cohort, value, treatment)?
        .mark_point()?
        .configure_point(|m| m.with_layout("quasirandom").with_size(1.5))
        .encode((alt::x("cohort"), alt::y("value"), alt::color("treatment")))?
        .with_size(600, 400)
        .save("./tests/quasirandom_1.svg")?;

    let svg = std::fs::read_to_string("./tests/quasirandom_1.svg")?;
    assert!(
        svg.contains("<circle"),
        "quasirandom layout should emit point markers"
    );

    Ok(())
}

#[test]
fn test_quasirandom_2() -> Result<(), Box<dyn Error>> {
    // The pseudorandom pairing and a flipped coordinate system must both work.
    let cohort: Vec<&str> = (0..120).map(|i| ["A", "B"][i % 2]).collect();
    let value: Vec<f64> = (0..120).map(|i| (i as f64 * 0.21).sin() * 10.0).collect();

    chart!(cohort, value)?
        .mark_point()?
        .configure_point(|m| {
            m.with_layout("quasirandom")
                .with_quasirandom_method("pseudorandom")
                .with_size(1.5)
        })
        .encode((alt::x("cohort"), alt::y("value")))?
        .with_size(600, 400)
        .coord_flip()
        .save("./tests/quasirandom_2.svg")?;

    let svg = std::fs::read_to_string("./tests/quasirandom_2.svg")?;
    assert!(
        svg.contains("<circle"),
        "flipped quasirandom layout should emit point markers"
    );

    Ok(())
}
