use charton::prelude::*;
use std::error::Error;

/// Contours of `z = sin(x) * cos(y)` on a regular grid, drawn as open paths.
#[test]
fn test_contour_lines() -> Result<(), Box<dyn Error>> {
    let n = 31;
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut z = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let xv = -3.0 + 6.0 * i as f64 / (n as f64 - 1.0);
            let yv = -3.0 + 6.0 * j as f64 / (n as f64 - 1.0);
            x.push(xv);
            y.push(yv);
            z.push(xv.sin() * yv.cos());
        }
    }

    chart!(x, y, z)?
        .transform_contour(ContourTransform::new("x", "y", "z").with_levels(8))?
        .mark_path()?
        .encode((
            alt::x("x"),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("level"),
        ))?
        .save("./target/test-output/contour_1.svg")?;

    Ok(())
}

/// Explicit levels: the same iso-lines drawn as open paths.
#[test]
fn test_contour_explicit_levels() -> Result<(), Box<dyn Error>> {
    let n = 21;
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut z = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let xv = i as f64 / (n as f64 - 1.0);
            let yv = j as f64 / (n as f64 - 1.0);
            x.push(xv);
            y.push(yv);
            z.push((xv - 0.5).powi(2) + (yv - 0.5).powi(2));
        }
    }

    chart!(x, y, z)?
        .transform_contour(
            ContourTransform::new("x", "y", "z").with_levels_values(vec![0.05, 0.15, 0.3]),
        )?
        .mark_path()?
        .encode((
            alt::x("x"),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("level"),
        ))?
        .save("./target/test-output/contour_2.svg")?;

    Ok(())
}

/// Density contour: scattered points → 2D density grid → iso-lines → path.
#[test]
fn test_density_contour() -> Result<(), Box<dyn Error>> {
    let iris = load_dataset("iris")?;

    chart!(iris)?
        .transform_density_2d(
            Density2DTransform::new("sepal_length", "petal_length").with_grid_size(40),
        )?
        .transform_contour(ContourTransform::new("x", "y", "density").with_levels(8))?
        .mark_path()?
        .encode((
            alt::x("x"),
            alt::y("y"),
            alt::path_group("path_group"),
            alt::color("level"),
        ))?
        .save("./target/test-output/contour_3.svg")?;

    Ok(())
}
