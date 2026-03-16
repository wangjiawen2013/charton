use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let df_placebo = df![
        "Weeks since Randomization" => [0, 4, 8, 12, 16, 20, 28, 36, 44, 52, 60, 68],
        "Change from Baseline (%)" => [0.00, -1.11, -1.72, -2.18, -2.54, -2.83, -2.82, -2.98, -3.24, -3.31, -3.22, -2.76],
        "lower" => [-0.042, -1.18, -1.81, -2.28, -2.66, -3.00, -3.03, -3.22, -3.49, -3.54, -3.46, -3.03],
        "upper" => [0.042, -1.04, -1.63, -2.08, -2.42, -2.66, -2.61, -2.74, -2.99, -3.08, -2.98, -2.49]
    ].unwrap();

    let df_semaglutide = df![
        "Weeks since Randomization" => [0, 4, 8, 12, 16, 20, 28, 36, 44, 52, 60, 68],
        "Change from Baseline (%)" => [0.00, -2.27, -4.01, -5.9, -7.66, -9.46, -11.68, -13.33, -14.62, -15.47, -15.86, -15.6],
        "lower" => [-0.041, -2.3, -4.1, -5.98, -7.79, -9.58, -11.84, -13.55, -14.83, -15.72, -16.13, -15.86],
        "upper" => [0.041, -2.24, -3.92, -5.82, -7.53, -9.34, -11.52, -13.11, -14.41, -15.22, -15.59, -15.34]
    ].unwrap();

    let df_text = df!["x" => [68, 68], "y" => [-2, -16], "group" => ["Placebo", "Semaglutide"]]?;

    let placebo_point = Chart::build(&df_placebo)?
        .mark_point()?.configure_point(|p| p.with_color("#818284"))
        .encode((x("Weeks since Randomization"), y("Change from Baseline (%)")))?;
    let placebo_line = Chart::build(&df_placebo)?.mark_line()?.configure_line(|l| l.with_color("#818284")).
        encode((x("Weeks since Randomization"), y("Change from Baseline (%)")))?;
    let placebo_errorbar = Chart::build(&df_placebo)?.mark_errorbar()?.configure_errorbar(|e| e.with_color("#818284"))
        .encode((x("Weeks since Randomization"), y("lower"), y2("upper")))?;
    let placebo_text = Chart::build(&df_text)?.mark_text()?.encode((x("x"), y("y"), text("group")))?;

    let semaglutide_point = Chart::build(&df_semaglutide)?.mark_point()?.configure_point(|p| p.with_color("#5b88c3"))
        .encode((x("Weeks since Randomization"), y("Change from Baseline (%)")))?;
    let semaglutide_line = Chart::build(&df_semaglutide)?.mark_line()?.configure_line(|l| l.with_color("#5b88c3"))
        .encode((x("Weeks since Randomization"), y("Change from Baseline (%)")))?;
    let semaglutide_errorbar = Chart::build(&df_semaglutide)?.mark_errorbar()?.configure_errorbar(|e| e.with_color("#5b88c3"))
        .encode((x("Weeks since Randomization"), y("lower"), y2("upper")))?;
    let semaglutide_text = Chart::build(&df_text)?.mark_text()?.configure_text(|t| t.with_color("#5b88c3")).encode((x("x"), y("y"), text("group")))?;

    placebo_point
        .and(placebo_line)
        .and(placebo_errorbar)
        .and(placebo_text)
        .and(semaglutide_point)
        .and(semaglutide_line)
        .and(semaglutide_errorbar)
        .and(semaglutide_text)
        .save("examples/nejm.svg")?;

    Ok(())
}