use charton::prelude::*;
use std::error::Error;

#[test]
fn test_scatter_1() -> Result<(), Box<dyn Error>> {
    let a = [Some(130.0), None, Some(156.0), Some(1500.0), None];
    let b = [-0.0001, -0.002, 0.001, 0.003, 1.0];
    let c = ["USA", "USA", "Europe", "USA", "Japan"];

    // Create a point chart using the new API
    let point_chart = chart!(a, b, c)?
        .mark_point()?
        .configure_point(|p| {
            p.with_stroke_width(1.0)
                .with_stroke("black")
                .with_color("red")
        })
        .encode((
            alt::x("a").with_scale(Scale::Linear),
            alt::y("b").with_scale(Scale::Linear),
        ))?;

    // Create a layered chart and add the point chart as a layer
    point_chart
        .with_size(500, 400)
        .coord_flip()
        .save("./target/test-output/scatter_1.svg")?;

    Ok(())
}

#[test]
fn test_scatter_2() -> Result<(), Box<dyn Error>> {
    let a = [
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        17.0, 18.0,
    ];
    let b = [
        10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 110.0, 120.0, 130.0, 140.0,
        150.0, 160.0, 170.0, 180.0,
    ];
    let category = [
        "A123XY", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q",
        "R",
    ];

    chart!(a, b, category)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::shape("category")))?
        .with_size(500, 300)
        .save("./target/test-output/scatter_2.svg")?;

    Ok(())
}

#[test]
fn test_scatter_3() -> Result<(), Box<dyn Error>> {
    let a = [130.0, -165.0, 156.0, -150.0, 1400.0];
    let b = [-0.0001, -0.002, 0.001, 0.003, 1.0];
    let origin = ["USA", "USA", "Europe", "USA", "Japan"];

    chart!(a, b, origin)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("origin")))?
        .with_size(500, 300)
        .with_title("Data")
        .with_x_label("A")
        .with_y_label("B")
        .save("./target/test-output/scatter_3.svg")?;

    Ok(())
}

#[test]
fn test_scatter_4() -> Result<(), Box<dyn Error>> {
    let a = [130.0, 165.0, 156.0, 150.0];
    let b = [18.0, 15.0, 20.0, 16.0];
    let origin = ["USA", "USA", "Europe", "Japan"];

    chart!(a, b, origin)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("origin")))?
        .with_size(500, 300)
        .with_title("Car Data")
        .configure_theme(|t| t.with_title_size(20.0).with_title_color("#333"))
        .save("./target/test-output/scatter_4.svg")?;

    Ok(())
}

#[test]
fn test_scatter_5() -> Result<(), Box<dyn Error>> {
    let a = [130.0, 165.0, 156.0, 1500.0];
    let b = [18.0, 15.0, 20.0, 16.0];
    let origin = ["USA", "USA", "Europe", "Japan"];

    chart!(a, b, origin)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("origin")))?
        .with_size(500, 400)
        .with_title("Data")
        .with_x_label("A)")
        .with_y_label("B")
        .configure_theme(|t| {
            t.with_title_size(20.0)
                .with_title_color("#333")
                .with_y_tick_label_angle(-45.0)
                .with_label_color("steelblue")
                .with_label_family("serif")
                .with_label_size(36.0)
        })
        .coord_flip()
        .save("./target/test-output/scatter_5.svg")?;

    Ok(())
}

#[test]
fn test_scatter_6() -> Result<(), Box<dyn Error>> {
    let a = &[130.0, 165.0, 150.0, 150.0, 225.0, 97.0];
    let b = &[18.0, 15.0, 18.0, 16.0, 17.0, 30.0];
    let origin = &["USA", "Germany", "Japan", "USA", "Germany", "Japan"];

    let point_chart = chart!(a, b, origin)?.mark_point()?.encode((
        alt::x("a"),
        alt::y("b"),
        alt::color("origin"),
    ))?;

    point_chart
        .with_size(500, 400)
        .with_title("Standard Chart: A vs B")
        .with_x_label("A")
        .with_y_label("B")
        .save("./target/test-output/scatter_6.svg")?;

    Ok(())
}

#[test]
fn test_scatter_7() -> Result<(), Box<dyn Error>> {
    let a = [1.0, 2.0, 3.0, 4.0, 5.0, 1.5, 2.5, 3.5, 4.5, 5.5];
    let b = [2.0, 4.0, 1.0, 5.0, 3.0, 3.5, 1.5, 4.5, 2.5, 5.5];
    let category = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];
    let value = [10.0, 20.0, 15.0, 25.0, 12.0, 18.0, 22.0, 14.0, 28.0, 16.0];
    let confidence = [0.9, 0.7, 0.8, 0.6, 0.95, 0.85, 0.75, 0.88, 0.72, 0.92];

    chart!(a, b, category, value, confidence)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("category")))?
        .with_size(500, 300)
        .with_title("visualization")
        .save("./target/test-output/scatter_7.svg")?;

    Ok(())
}

#[test]
fn test_scatter_8() -> Result<(), Box<dyn Error>> {
    // Example with GDP data that benefits from log scale
    let country = ["A", "B", "C", "D", "E"];
    let gdp = [1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0];
    let population = [100000.0, 500000.0, 2000000.0, 10000000.0, 50000000.0];

    chart!(country, gdp, population)?
        .mark_point()?
        .encode((
            alt::x("population"),
            alt::y("gdp").with_scale(Scale::Log), // Use logarithmic scale for GDP
        ))?
        .with_size(500, 400)
        .save("./target/test-output/scatter_8.svg")?;

    Ok(())
}

#[test]
fn test_scatter_9() -> Result<(), Box<dyn Error>> {
    // Example with categorical data on x-axis and numerical data on y-axis
    let department = [
        "Engineering",
        "Marketing",
        "Sales",
        "HR",
        "Finance",
        "Engineering",
        "Marketing",
    ];
    let salary = [
        85000.0, 65000.0, 60000.0, 55000.0, 75000.0, 90000.0, 68000.0,
    ];

    // Create a point chart
    chart!(department, salary)?
        .mark_point()?
        .encode((alt::x("department"), alt::y("salary")))?
        .with_size(600, 400)
        .with_title("Salary by Department")
        .save("./target/test-output/scatter_9.svg")?;

    Ok(())
}

#[test]
fn test_scatter_10() -> Result<(), Box<dyn Error>> {
    // Create sample data with a continuous variable for color encoding
    let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let b = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
    let value = [1.5, 2.3, 3.7, 4.1, 5.9, 6.2, 7.8, 8.4, 9.1, 10.6]; // Continuous variable for color

    // Create a chart with color encoding using the continuous variable
    let chart = chart!(a, b, value)?.mark_point()?.encode((
        alt::x("a"),
        alt::y("b"),
        alt::color("value"), // This will trigger the colorbar instead of discrete legend
        alt::size("value"),  // This will trigger the colorbar instead of discrete legend
    ))?;

    chart
        .with_size(500, 300)
        .with_title("Chart with Colorbar")
        .save("./target/test-output/scatter_10.svg")?;

    Ok(())
}

#[test]
fn test_scatter_11() -> Result<(), Box<dyn Error>> {
    // Create sample data with a categorical variable for shape encoding
    let a = [1.0, 2.0, 3.0, 4.0, 5.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let b = [10.0, 20.0, 30.0, 40.0, 50.0, 15.0, 25.0, 35.0, 45.0, 55.0];
    let category = ["A", "B", "C", "A", "B", "C", "A", "B", "C", "A"]; // Categorical variable for shape

    // Create a chart with shape encoding using the categorical variable
    chart!(a, b, category)?
        .mark_point()?
        .encode((
            alt::x("a"),
            alt::y("b"),
            //size("x"),
            alt::shape("category"), // This will trigger the shape legend
            alt::color("category"), // This will trigger the color legend
        ))?
        .with_size(500, 300)
        .with_title("Chart with Shape Legend")
        .save("./target/test-output/scatter_11.svg")?;

    Ok(())
}

#[test]
fn test_scatter_12() -> Result<(), Box<dyn Error>> {
    let a = [1.0, 2.0, 3.0, 4.0, 5.0];
    let b = [10.0, 20.0, 30.0, 40.0, 50.0];
    let category = ["A", "B", "A", "B", "C"];

    // Create a point chart with only x, y, and color encodings
    chart!(a, b, category)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("category")))?
        .with_size(500, 300)
        .save("./target/test-output/scatter_12.svg")?;

    Ok(())
}

#[test]
fn test_scatter_13() -> Result<(), Box<dyn Error>> {
    // Create sample data
    let a = [1.0, 2.0, 3.0, 4.0, 5.0];
    let b = [10.0, 20.0, 30.0, 40.0, 50.0];
    let category = ["A", "B", "C", "D", "E"];

    chart!(a, b, category)?
        .mark_point()?
        .encode((alt::x("category"), alt::y("b"), alt::color("category")))?
        .with_size(600, 400)
        .with_title("Chart with Explicit Tick Values")
        .with_x_label("Catergory")
        .with_y_label("B Values")
        .save("./target/test-output/scatter_13.svg")?;

    Ok(())
}

#[test]
fn test_scatter_14() -> Result<(), Box<dyn Error>> {
    let a = [130.0, 165.0, 156.0, 1500.0];
    let b = [18.0, 15.0, 20.0, 16.0];
    let origin = ["USA", "USA", "Europe", "Japan"];

    chart!(a, b, origin)?
        .mark_point()?
        .encode((alt::x("a"), alt::y("b"), alt::color("origin")))?
        .save("./target/test-output/scatter_14.svg")?;

    Ok(())
}

#[test]
fn test_scatter_15() -> Result<(), Box<dyn Error>> {
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
    chart!(categories, outcomes, treatments)?
        .mark_point()?
        .configure_point(|m| m.with_layout("jitter").with_size(2.0))
        .encode((
            alt::x("categories"),
            alt::y("outcomes"),
            alt::color("treatments"),
        ))?
        .save("./target/test-output/scatter_15.svg")?;

    Ok(())
}

/// Shape + size + colour on a flipped axis (was `examples/scatter.rs`).
#[test]
fn test_scatter_16() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("mtcars")?;

    Chart::build(ds)?
        .mark_point()?
        .encode((
            alt::x("wt"),
            alt::y("mpg"),
            alt::color("gear").with_scale(Scale::Discrete),
            alt::shape("gear").with_scale(Scale::Discrete),
            alt::size("mpg"),
        ))?
        .coord_flip()
        .configure_theme(|t| t.with_x_tick_label_angle(-45.0))
        .save("./target/test-output/scatter_16.svg")?;

    Ok(())
}

/// `with_dodge(false)` keeps colour-grouped points on the category centre — the
/// opt-out a dumbbell needs so both ends stay on the shared segment.
#[test]
fn point_dodge_can_be_disabled() -> Result<(), Box<dyn Error>> {
    let cat = ["A", "A", "B", "B"];
    let val = [1.0, 2.0, 1.0, 2.0];
    let grp = ["x", "y", "x", "y"];

    chart!(cat, val, grp)?
        .mark_point()?
        .configure_point(|p| p.with_dodge(false))
        .encode((alt::x("cat"), alt::y("val"), alt::color("grp")))?
        .configure_theme(|t| t.with_show_legend(false))
        .save("./target/test-output/point_no_dodge.svg")?;

    let svg = std::fs::read_to_string("./target/test-output/point_no_dodge.svg")?;
    let xs: Vec<f64> = svg
        .split("<circle ")
        .skip(1)
        .filter_map(|seg| {
            let s = seg.find("cx=\"")? + 4;
            let e = seg[s..].find('"')? + s;
            seg[s..e].parse().ok()
        })
        .collect();

    assert_eq!(xs.len(), 4, "expected four points, got {xs:?}");

    // The four points collapse onto exactly two x positions (one per category),
    // each shared by the two colour groups — so nothing dodged.
    let mut sorted = xs.clone();
    sorted.sort_by(f64::total_cmp);
    assert!((sorted[0] - sorted[1]).abs() < 0.5, "groups dodged: {xs:?}");
    assert!((sorted[2] - sorted[3]).abs() < 0.5, "groups dodged: {xs:?}");
    assert!(
        (sorted[1] - sorted[2]).abs() > 1.0,
        "categories collapsed: {xs:?}"
    );
    Ok(())
}

/// x positions of every drawn point (legend is off in these tests).
fn circle_cx(svg: &str) -> Vec<f64> {
    svg.split("<circle ")
        .skip(1)
        .filter_map(|seg| {
            let s = seg.find("cx=\"")? + 4;
            let e = seg[s..].find('"')? + s;
            seg[s..e].parse().ok()
        })
        .collect()
}

fn distinct_count(mut xs: Vec<f64>) -> usize {
    xs.sort_by(f64::total_cmp);
    let mut n = 0;
    let mut last = f64::NEG_INFINITY;
    for x in xs {
        if (x - last).abs() > 0.5 {
            n += 1;
            last = x;
        }
    }
    n
}

/// On a categorical x a colour grouping still dodges: the two series sit side
/// by side, giving four distinct x positions over two categories.
#[test]
fn categorical_axis_points_dodge() -> Result<(), Box<dyn Error>> {
    let cat = ["A", "A", "B", "B"];
    let val = [1.0, 2.0, 1.0, 2.0];
    let grp = ["x", "y", "x", "y"];

    chart!(cat, val, grp)?
        .mark_point()?
        .encode((alt::x("cat"), alt::y("val"), alt::color("grp")))?
        .configure_theme(|t| t.with_show_legend(false))
        .save("./target/test-output/point_cat_dodge.svg")?;

    let svg = std::fs::read_to_string("./target/test-output/point_cat_dodge.svg")?;
    assert_eq!(
        distinct_count(circle_cx(&svg)),
        4,
        "expected two dodged lanes"
    );
    Ok(())
}

/// On a continuous x a colour grouping does NOT dodge — it is an attribute. The
/// offset would otherwise be scaled by an arbitrary data unit.
#[test]
fn continuous_axis_points_do_not_dodge() -> Result<(), Box<dyn Error>> {
    let x = [0.0, 1.0, 0.0, 1.0];
    let y = [1.0, 1.0, 2.0, 2.0];
    let grp = ["a", "a", "b", "b"];

    chart!(x, y, grp)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y"), alt::color("grp")))?
        .configure_theme(|t| t.with_show_legend(false))
        .save("./target/test-output/point_cont_no_dodge.svg")?;

    let svg = std::fs::read_to_string("./target/test-output/point_cont_no_dodge.svg")?;
    // Two x values, each shared by both colour groups.
    assert_eq!(
        distinct_count(circle_cx(&svg)),
        2,
        "continuous axis must not dodge"
    );
    Ok(())
}
