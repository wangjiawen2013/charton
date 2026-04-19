use charton::prelude::*;
use std::error::Error;

// The data is obtained from paper "Once-Weekly Semaglutide in Adults with Overweight or Obesity"
// using [webplotdigitizer](https://automeris.io/).
fn main() -> Result<(), Box<dyn Error>> {
    // Placebo group data (control group)
    let ds_placebo = vec![
        // X-axis: time since randomization (weeks)
        (
            "Weeks since Randomization",
            [0, 4, 8, 12, 16, 20, 28, 36, 44, 52, 60, 68].into_column(),
        ),
        // Y-axis: mean percentage change in body weight from baseline. Negative values indicate weight loss
        (
            "Change from Baseline (%)",
            [
                0.00, -1.11, -1.72, -2.18, -2.54, -2.83, -2.82, -2.98, -3.24, -3.31, -3.22, -2.76,
            ]
            .into_column(),
        ),
        // Lower bound of the confidence interval (typically 95% CI)
        (
            "lower",
            [
                -0.042, -1.18, -1.81, -2.28, -2.66, -3.00, -3.03, -3.22, -3.49, -3.54, -3.46, -3.03,
            ]
            .into_column(),
        ),
        // Upper bound of the confidence interval (95% CI)
        (
            "upper",
            [
                0.042, -1.04, -1.63, -2.08, -2.42, -2.66, -2.61, -2.74, -2.99, -3.08, -2.98, -2.49,
            ]
            .into_column(),
        ),
    ]
    .to_dataset()?;

    // Semaglutide group data (treatment group)
    let ds_semaglutide = vec![
        (
            "Weeks since Randomization",
            [0, 4, 8, 12, 16, 20, 28, 36, 44, 52, 60, 68].into_column(),
        ),
        (
            "Change from Baseline (%)",
            [
                0.00, -2.27, -4.01, -5.9, -7.66, -9.46, -11.68, -13.33, -14.62, -15.47, -15.86,
                -15.6,
            ]
            .into_column(),
        ),
        (
            "lower",
            [
                -0.041, -2.3, -4.1, -5.98, -7.79, -9.58, -11.84, -13.55, -14.83, -15.72, -16.13,
                -15.86,
            ]
            .into_column(),
        ),
        (
            "upper",
            [
                0.041, -2.24, -3.92, -5.82, -7.53, -9.34, -11.52, -13.11, -14.41, -15.22, -15.59,
                -15.34,
            ]
            .into_column(),
        ),
    ]
    .to_dataset()?;

    // Text labels (placed at the right side of the plot)
    let ds_text = vec![
        ("x", [68.8, 68.8].into_column()),
        ("y", [-3.05, -15.86].into_column()),
        ("group", ["Placebo", "Semaglutide"].into_column()),
    ]
    .to_dataset()?;

    // Reference line (y = 0 → no weight change)
    let ds_reference = vec![
        ("x", [0.0, 68.0].into_column()),
        ("y", [0.0, 0.0].into_column()),
    ]
    .to_dataset()?;

    // Layer 1: Placebo points (markers at each time point)
    let placebo_point = Chart::build(&ds_placebo)?
        .mark_point()?
        .configure_point(|p| {
            p.with_color("#818284")
                .with_shape("triangle")
                .with_size(5.0)
        })
        .encode((
            alt::x("Weeks since Randomization"),
            alt::y("Change from Baseline (%)"),
        ))?;

    // Layer 2: Placebo line (connects the points)
    let placebo_line = Chart::build(&ds_placebo)?
        .mark_line()?
        .configure_line(|l| l.with_color("#818284"))
        .encode((
            alt::x("Weeks since Randomization"),
            alt::y("Change from Baseline (%)"),
        ))?;

    // Layer 3: Placebo error bars (confidence intervals)
    let placebo_errorbar = Chart::build(&ds_placebo)?
        .mark_errorbar()?
        .configure_errorbar(|e| {
            e.with_color("#818284")
                .with_cap_length(4.0)
                .with_stroke_width(1.5)
        })
        .encode((
            alt::x("Weeks since Randomization"),
            alt::y("lower"),
            alt::y2("upper"),
        ))?;

    // Layer 4: Placebo text label
    let placebo_text = Chart::build(&ds_text.head(1))?
        .mark_text()?
        .configure_text(|t| t.with_anchor("left").with_size(14.0))
        .encode((alt::x("x"), alt::y("y"), alt::text("group")))?;

    // Layer 5: Semaglutide points
    let semaglutide_point = Chart::build(&ds_semaglutide)?
        .mark_point()?
        .configure_point(|p| p.with_color("#5b88c3").with_shape("square").with_size(3.0))
        .encode((
            alt::x("Weeks since Randomization"),
            alt::y("Change from Baseline (%)"),
        ))?;

    // Layer 6: Semaglutide line
    let semaglutide_line = Chart::build(&ds_semaglutide)?
        .mark_line()?
        .configure_line(|l| l.with_color("#5b88c3"))
        .encode((
            alt::x("Weeks since Randomization"),
            alt::y("Change from Baseline (%)"),
        ))?;

    // Layer 7: Semaglutide error bars
    let semaglutide_errorbar = Chart::build(&ds_semaglutide)?
        .mark_errorbar()?
        .configure_errorbar(|e| {
            e.with_color("#5b88c3")
                .with_cap_length(4.0)
                .with_stroke_width(1.5)
        })
        .encode((
            alt::x("Weeks since Randomization"),
            alt::y("lower"),
            alt::y2("upper"),
        ))?;

    // Layer 8: Semaglutide text label
    let semaglutide_text = Chart::build(&ds_text.tail(1))?
        .mark_text()?
        .configure_text(|t| t.with_anchor("left").with_size(14.0))
        .encode((alt::x("x"), alt::y("y"), alt::text("group")))?;

    // Layer 9: Reference line (baseline at 0%)
    let reference_line = Chart::build(&ds_reference)?
        .mark_line()?
        .configure_line(|l| l.with_dash([6.0, 6.0]))
        .encode((alt::x("x"), alt::y("y")))?;

    // Combine all layers (Grammar of Graphics composition)
    placebo_point
        .and(reference_line)
        .and(placebo_line)
        .and(placebo_errorbar)
        .and(placebo_text)
        .and(semaglutide_point)
        .and(semaglutide_line)
        .and(semaglutide_errorbar)
        .and(semaglutide_text)
        .with_x_expand(Expansion {
            mult: (0.00, 0.02),
            add: (0.0, 0.0),
        })
        .with_y_expand(Expansion {
            mult: (0.15, 0.01),
            add: (0.0, 0.0),
        })
        .with_size(1000, 400)
        .with_right_margin(0.08)
        .with_left_margin(0.02)
        .with_top_margin(0.02)
        .with_bottom_margin(0.03)
        .with_x_ticks([
            0.0, 4.0, 8.0, 12.0, 16.0, 20.0, 28.0, 36.0, 44.0, 52.0, 60.0, 68.0,
        ])
        .with_y_ticks([
            0.0, -2.0, -4.0, -6.0, -8.0, -10.0, -12.0, -14.0, -16.0, -18.0,
        ])
        .save("docs/src/images/weight_loss_curve.svg")?;

    Ok(())
}
