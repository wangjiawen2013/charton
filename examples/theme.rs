//! Light and dark themes.
//!
//! A [`Theme`] is the complete visual preset of a chart: colors, fonts, margins
//! and layout. A [`ThemeMode`] is a ready-made light/dark *color* scheme layered
//! on top of it. Switching modes never changes layout or typography, only the
//! colors that define the look against the background.
//!
//! There are three ways to set it, from coarsest to finest:
//!
//! 1. `with_theme(Theme::dark())` — replace the whole theme with a preset.
//! 2. `configure_theme(|t| t.with_mode(ThemeMode::Dark))` — switch the preset
//!    on the existing theme. This is what the Nushell plugin's `--theme` maps
//!    onto.
//! 3. `configure_theme(|t| ...with_*)` — explicit overrides. Applied *after* a
//!    mode, so they always win over the preset.
//!
//! Run with: `cargo run --example theme`

use charton::error::ChartonError;
use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ds = load_dataset("iris")?;

    let base = || -> Result<LayeredChart, ChartonError> {
        Ok(Chart::build(ds.clone())?
            .mark_point()?
            .encode((
                alt::x("petal_length"),
                alt::y("petal_width"),
                alt::color("species"),
            ))?
            .with_size(640, 420))
    };

    // 1. Explicit preset: `Theme::light()` is just the default theme.
    base()?
        .with_title("Iris — Theme::light()")
        .with_theme(Theme::light())
        .save("docs/src/images/theme_light.svg")?;

    // 2. Explicit preset: dark. Only colors differ from the light theme.
    base()?
        .with_title("Iris — Theme::dark()")
        .with_theme(Theme::dark())
        .save("docs/src/images/theme_dark.svg")?;

    // 3. Switch the active theme with a `ThemeMode`, then layer user overrides
    //    on top. This mirrors exactly how the CLI exposes `--theme`.
    base()?
        .with_title("Iris — ThemeMode::Dark + custom grid")
        .configure_theme(|t| {
            t.with_mode(ThemeMode::Dark)
                // Overrides run after the mode, so these win over the preset.
                .with_grid(true)
                .with_grid_color("#45475A")
        })
        .save("docs/src/images/theme_dark_custom.svg")?;

    Ok(())
}
