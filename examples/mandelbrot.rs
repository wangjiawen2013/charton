//! Mandelbrot set, drawn from the escape time of every sample.
//!
//! Ported from `examples/mandelbrot.rs` in the
//! [plotters](https://github.com/plotters-rs/plotters) project. For every pixel
//! the example iterates `z <- z^2 + c` and colours the pixel by the number of
//! iterations before the orbit escapes; points that never escape stay black.
//!
//! The plot is a raster asset, so the `png` feature is required:
//!
//! ```text
//! cargo run --example mandelbrot --features png
//! ```

use charton::prelude::*;
use std::error::Error;
use std::ops::Range;

const OUT_FILE_NAME: &str = "docs/src/images/mandelbrot.png";

/// The plotters example samples a 750x550 plotting area. Keep that sampling
/// resolution for detail; the chart uses Charton's default 500x400 canvas.
const SAMPLES: (usize, usize) = (750, 550);
const MAX_ITER: usize = 100;

fn main() -> Result<(), Box<dyn Error>> {
    let (real, imaginary) = (-2.1f64..0.6f64, -1.2f64..1.2f64);

    // One horizontal run per adjacent group of pixels with the same escape
    // count. A run becomes one rectangle (a four-vertex polygon), which keeps
    // the chart at ~35k shapes instead of ~400k individual pixels.
    let mut runs: Vec<Vec<(f64, f64, f64, f64)>> = vec![Vec::new(); MAX_ITER + 1];
    let rows = mandelbrot_rows(real.clone(), imaginary.clone(), SAMPLES, MAX_ITER);
    let (dx, dy) = (
        (real.end - real.start) / SAMPLES.0 as f64,
        (imaginary.end - imaginary.start) / SAMPLES.1 as f64,
    );

    for (row, samples) in rows.iter().enumerate() {
        let mut start = 0;
        while start < SAMPLES.0 {
            let iterations = samples[start];
            let mut end = start + 1;
            while end < SAMPLES.0 && samples[end] == iterations {
                end += 1;
            }

            // Half a pixel of overlap closes the anti-aliasing seams between
            // neighbouring runs. Later classes are drawn on top, so the shared
            // edge belongs to the larger escape count.
            let x0 = real.start + dx * start as f64 - dx * 0.5;
            let x1 = real.start + dx * end as f64 + dx * 0.5;
            let y0 = imaginary.start + dy * row as f64 - dy * 0.5;
            let y1 = imaginary.start + dy * (row + 1) as f64 + dy * 0.5;
            runs[iterations].push((x0, x1, y0, y1));
            start = end;
        }
    }

    // plotters' MandelbrotHSL is HSL(hue = iterations / MAX_ITER, 1.0, 0.5).
    // Charton has no built-in HSL wheel, so the exact colours are supplied as
    // a custom discrete palette, one entry per escape count. The data below is
    // emitted in the same order, because a discrete scale follows first
    // appearance.
    let palette: Vec<SingleColor> = runs
        .iter()
        .enumerate()
        .filter(|(_, rectangles)| !rectangles.is_empty())
        .map(|(iterations, _)| {
            if iterations == MAX_ITER {
                SingleColor::new("#000000")
            } else {
                mandelbrot_hsl(iterations as f64 / MAX_ITER as f64)
            }
        })
        .collect();

    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut iterations = Vec::new();
    let mut run_id = Vec::new();
    let mut run = 0usize;
    for (class, rectangles) in runs.iter().enumerate() {
        for &(x0, x1, y0, y1) in rectangles {
            let id = format!("run-{run}");
            run += 1;
            for (px, py) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
                x.push(px);
                y.push(py);
                iterations.push(class.to_string());
                run_id.push(id.clone());
            }
        }
    }

    let ds = Dataset::new()
        .with_column("x", x)?
        .with_column("y", y)?
        .with_column("iterations", iterations)?
        .with_column("run", run_id)?;

    let fractal = Chart::build(ds)?
        .mark_polygon()?
        .configure_polygon(|polygon| {
            polygon
                .with_fill("#000000")
                .with_stroke("none")
                .with_opacity(1.0)
        })
        .encode((
            alt::x("x"),
            alt::y("y"),
            alt::color("iterations").with_scale(Scale::Discrete),
            alt::path_group("run"),
        ))?;

    // plotters' mesh draws a frame around the plotting area. Charton's axes
    // only draw the left and bottom lines, so add the closed frame as a
    // second, unfilled polygon layer.
    let (x0, x1) = (real.start + dx * 0.5, real.end - dx * 0.5);
    let (y0, y1) = (imaginary.start + dy * 0.5, imaginary.end - dy * 0.5);
    let frame = Chart::build(
        Dataset::new()
            .with_column("x", vec![x0, x1, x1, x0])?
            .with_column("y", vec![y0, y0, y1, y1])?
            .with_column("frame", vec!["frame"; 4])?,
    )?
    .mark_polygon()?
    .configure_polygon(|polygon| {
        polygon
            .with_fill("none")
            .with_stroke("black")
            .with_stroke_width(1.0)
    })
    .encode((alt::x("x"), alt::y("y"), alt::path_group("frame")))?;

    fractal
        .and(frame)
        .with_x_domain(-2.1, 0.6)
        .with_y_domain(-1.2, 1.2)
        // The panel maps the data range exactly, as in the plotters version.
        .with_x_expand(Expansion {
            mult: (0.0, 0.0),
            add: (0.0, 0.0),
        })
        .with_y_expand(Expansion {
            mult: (0.0, 0.0),
            add: (0.0, 0.0),
        })
        // plotters draws ticks but no axis titles here.
        .with_x_label("")
        .with_y_label("")
        .configure_theme(|theme| {
            theme
                .with_show_legend(false)
                .with_grid(false)
                .with_palette(palette)
                // Approximate the 10px tick-label band of the plotters mesh.
                .with_tick_label_size(10.0)
                .with_tick_count(6)
                .with_tick_length(3.0)
        })
        .save(OUT_FILE_NAME)?;

    println!("Result has been saved to {OUT_FILE_NAME}");
    Ok(())
}

/// Computes one escape count per sample. Rows run from low to high imaginary
/// values, matching Charton's upward Y axis (the set is symmetric).
fn mandelbrot_rows(
    real: Range<f64>,
    imaginary: Range<f64>,
    samples: (usize, usize),
    max_iter: usize,
) -> Vec<Vec<usize>> {
    let step = (
        (real.end - real.start) / samples.0 as f64,
        (imaginary.end - imaginary.start) / samples.1 as f64,
    );
    (0..samples.1)
        .map(|j| {
            (0..samples.0)
                .map(|i| {
                    // Sample the corner of each screen pixel, matching the
                    // plotters loop (`real.start + step * index`).
                    let c = (
                        real.start + step.0 * i as f64,
                        imaginary.start + step.1 * j as f64,
                    );
                    let mut z = (0.0, 0.0);
                    let mut count = 0;
                    while count < max_iter && z.0 * z.0 + z.1 * z.1 <= 1e10 {
                        z = (z.0 * z.0 - z.1 * z.1 + c.0, 2.0 * z.0 * z.1 + c.1);
                        count += 1;
                    }
                    count
                })
                .collect()
        })
        .collect()
}

/// The exact colour conversion used by plotters' `MandelbrotHSL` palette.
fn mandelbrot_hsl(hue: f64) -> SingleColor {
    let (saturation, lightness) = (1.0, 0.5);
    let q = if lightness < 0.5 {
        lightness * (1.0 + saturation)
    } else {
        lightness + saturation - lightness * saturation
    };
    let p = 2.0 * lightness - q;

    let channel = |mut t: f64| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        let value = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (value * 255.0).round() as u8
    };

    SingleColor::from_rgba(
        f64::from(channel(hue + 1.0 / 3.0)) / 255.0,
        f64::from(channel(hue)) / 255.0,
        f64::from(channel(hue - 1.0 / 3.0)) / 255.0,
        1.0,
    )
}
