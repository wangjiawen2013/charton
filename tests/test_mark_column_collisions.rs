//! Composite marks generate intermediate columns (`x`, `y`, `path_group`,
//! `density`, `level`). A user's own columns may be named exactly the same, so
//! the generated names must never collide with them.

use charton::prelude::*;

fn category_and_value(cat: &str, val: &str) -> Dataset {
    Dataset::new()
        .with_column(cat, vec!["a", "a", "b", "b"])
        .unwrap()
        .with_column(val, vec![1.0, 2.0, 3.0, 4.0])
        .unwrap()
}

#[test]
fn violin_survives_columns_named_x_and_y() {
    let r = Chart::build(category_and_value("x", "y"))
        .unwrap()
        .mark_violin()
        .unwrap()
        .encode((alt::x("x"), alt::y("y")));
    assert!(r.is_ok(), "violin collision: {:?}", r.err());
}

#[test]
fn violin_survives_a_value_column_named_density() {
    let r = Chart::build(category_and_value("grp", "density"))
        .unwrap()
        .mark_violin()
        .unwrap()
        .encode((alt::x("grp"), alt::y("density")));
    assert!(r.is_ok(), "violin collision: {:?}", r.err());
}

#[test]
fn density_survives_a_value_column_named_density() {
    let d = Dataset::new()
        .with_column("density", vec![1.0, 2.0, 3.0, 4.0])
        .unwrap();
    let r = Chart::build(d)
        .unwrap()
        .mark_density()
        .unwrap()
        .encode(alt::x("density"));
    assert!(r.is_ok(), "density collision: {:?}", r.err());
}

#[test]
fn density_2d_survives_columns_named_x_and_y() {
    let d = Dataset::new()
        .with_column("x", vec![1.0, 2.0, 3.0, 4.0, 1.0, 2.0, 3.0, 4.0])
        .unwrap()
        .with_column("y", vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0])
        .unwrap();
    let r = Chart::build(d)
        .unwrap()
        .mark_density_2d()
        .unwrap()
        .encode((alt::x("x"), alt::y("y")));
    assert!(r.is_ok(), "density_2d collision: {:?}", r.err());
}

#[test]
fn contour_survives_columns_named_x_y_and_z() {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut z = Vec::new();
    for i in 0..5 {
        for j in 0..5 {
            let xv = i as f64;
            let yv = j as f64;
            x.push(xv);
            y.push(yv);
            z.push((xv - 2.0).hypot(yv - 2.0));
        }
    }
    let d = Dataset::new()
        .with_column("x", x)
        .unwrap()
        .with_column("y", y)
        .unwrap()
        .with_column("z", z)
        .unwrap();
    let r = Chart::build(d)
        .unwrap()
        .mark_contour("z")
        .unwrap()
        .encode((alt::x("x"), alt::y("y")));
    assert!(r.is_ok(), "contour collision: {:?}", r.err());
}
