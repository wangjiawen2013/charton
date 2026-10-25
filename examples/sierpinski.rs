//! Sierpinski carpet demo.
//!
//! Ported from `examples/sierpinski.rs` in the
//! [plotters](https://github.com/plotters-rs/plotters) project. The square is
//! split into a 3x3 grid, the central ninth is left white, and the same rule is
//! applied recursively to the other eight squares, producing the self-similar
//! carpet below.
//!
//! The carpet is one blue polygon with one even-odd hole per removed square,
//! the same hollow-shape mechanism exercised by `tests/test_geo_holes.rs`.
//! The `png` feature is required:
//!
//! ```text
//! cargo run --example sierpinski --features png
//! ```

use charton::prelude::*;
use std::error::Error;

const OUT_FILE_NAME: &str = "docs/src/images/sierpinski.png";

const DEPTH: u32 = 5;
const GRID: usize = 3usize.pow(DEPTH);

/// Charton's default canvas is 500x400. The carpet is the largest square that
/// fits below the title band.
const CANVAS_WIDTH: f64 = 500.0;
const CANVAS_HEIGHT: f64 = 400.0;
const TITLE_SIZE: f64 = 30.0;
const TITLE_BAND: f64 = TITLE_SIZE * 1.2 + 6.0;
const CARPET_SIZE: f64 = CANVAS_HEIGHT - TITLE_BAND;

fn main() -> Result<(), Box<dyn Error>> {
    // The blue shape is the whole square minus every white centre. With the
    // even-odd fill rule, the outer ring plus the hole rings is exactly the
    // carpet; the white centres are transparent and show the canvas below.
    let mut rings = vec![(0.0, 0.0, GRID as f64, GRID as f64)];
    collect_holes(DEPTH, 0, 0, GRID, &mut rings);

    let mut x = Vec::with_capacity(rings.len() * 4);
    let mut y = Vec::with_capacity(rings.len() * 4);
    let mut group = Vec::with_capacity(rings.len() * 4);
    let mut ring_id = Vec::with_capacity(rings.len() * 4);
    for (index, &(x0, y0, x1, y1)) in rings.iter().enumerate() {
        let id = format!("ring-{index}");
        for (px, py) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
            x.push(px);
            y.push(py);
            group.push("carpet".to_string());
            ring_id.push(id.clone());
        }
    }

    let ds = Dataset::new()
        .with_column("x", x)?
        .with_column("y", y)?
        .with_column("group", group)?
        .with_column("ring", ring_id)?;

    Chart::build(ds)?
        .mark_polygon()?
        .configure_polygon(|polygon| {
            polygon
                .with_fill("blue")
                .with_stroke("none")
                .with_opacity(1.0)
        })
        .encode((
            alt::x("x"),
            alt::y("y"),
            // `with_subpath` keeps the outer ring and each hole as separate
            // rings of the same even-odd filled polygon.
            alt::path_group("group").with_subpath("ring"),
        ))?
        .with_x_domain(0.0, GRID as f64)
        .with_y_domain(0.0, GRID as f64)
        // Fill the panel edge to edge: the carpet is the whole panel.
        .with_x_expand(Expansion {
            mult: (0.0, 0.0),
            add: (0.0, 0.0),
        })
        .with_y_expand(Expansion {
            mult: (0.0, 0.0),
            add: (0.0, 0.0),
        })
        // Margins are fractions of the canvas; centre the square carpet.
        .with_margins(
            0.0,
            (CANVAS_WIDTH - CARPET_SIZE) / 2.0 / CANVAS_WIDTH,
            0.0,
            (CANVAS_WIDTH - CARPET_SIZE) / 2.0 / CANVAS_WIDTH,
        )
        .with_title("Sierpinski Carpet Demo")
        .configure_theme(|theme| {
            theme
                .with_show_axes(false)
                .with_show_legend(false)
                .with_grid(false)
                .with_title_size(TITLE_SIZE)
                .with_title_family("sans-serif")
        })
        .save(OUT_FILE_NAME)?;

    println!("Result has been saved to {OUT_FILE_NAME}");
    Ok(())
}

/// Records the white centre of every recursive 3x3 split, then recurses into
/// the eight blue sub-squares. This mirrors the plotters routine, where the
/// centre is filled white and the other eight areas are filled blue.
fn collect_holes(
    depth: u32,
    x: usize,
    y: usize,
    size: usize,
    rings: &mut Vec<(f64, f64, f64, f64)>,
) {
    if depth == 0 {
        return;
    }

    let sub_size = size / 3;
    let (cx, cy) = (x + sub_size, y + sub_size);
    rings.push((
        cx as f64,
        cy as f64,
        (cx + sub_size) as f64,
        (cy + sub_size) as f64,
    ));

    for j in 0..3 {
        for i in 0..3 {
            if i == 1 && j == 1 {
                continue;
            }
            collect_holes(
                depth - 1,
                x + i * sub_size,
                y + j * sub_size,
                sub_size,
                rings,
            );
        }
    }
}
