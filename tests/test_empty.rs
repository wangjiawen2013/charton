use charton::prelude::*;
use std::error::Error;

#[test]
fn test_empty_1() -> Result<(), Box<dyn Error>> {
    let a = Vec::<f64>::new();
    let b = Vec::<f64>::new();

    // Create a chart with empty data
    chart!(a, b)?
        .mark_point()?
        .configure_point(|p| {
            p.with_stroke_width(1.0)
                .with_stroke("black")
                .with_color("red")
        })
        .encode((
            alt::x("a").with_scale(Scale::Linear),
            alt::y("b").with_scale(Scale::Linear),
        ))?
        .with_size(500, 400)
        .coord_flip()
        .save("./tests/empty_1.svg")?;

    Ok(())
}

// Test the combination of empty charts with scatter chart
#[test]
fn test_empty_2() -> Result<(), Box<dyn Error>> {
    let a = Vec::<f64>::new();
    let b = Vec::<f64>::new();

    let empty_chart = chart!(a, b)?.mark_point()?.encode((
        alt::x("a").with_scale(Scale::Linear),
        alt::y("b").with_scale(Scale::Linear),
    ))?;

    let a = vec![
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        17.0, 18.0,
    ];
    let b = vec![
        10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 110.0, 120.0, 130.0, 140.0,
        150.0, 160.0, 170.0, 180.0,
    ];
    let category = vec![
        "A123XY", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q",
        "R",
    ];

    let point_chart = chart!(a, b, category)?.mark_point()?.encode((
        alt::x("a"),
        alt::y("b"),
        alt::shape("category"),
    ))?;

    point_chart
        .and(empty_chart)
        .with_size(500, 300)
        .save("./tests/empty_2.svg")?;

    Ok(())
}
