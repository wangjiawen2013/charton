use charton::prelude::*;
use std::error::Error;

/// A currency-prefixed, compact-notation axis is the canonical production case
/// (Economist-style charts): `40000000000 -> "$40B"`.
#[test]
fn y_axis_labels_can_be_abbreviated_with_a_currency_prefix() -> Result<(), Box<dyn Error>> {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![0.0, 1.0e10, 2.0e10, 3.0e10, 4.0e10];

    let svg = chart!(x, y)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y")))?
        .with_y_ticks(vec![0.0, 2.0e10, 4.0e10])
        .with_y_label_format(
            LabelFormat::new()
                .with_prefix("$")
                .with_compact_notation()
                .with_precision(0),
        )
        .to_svg()?;

    assert!(
        svg.contains(">$0</text>"),
        "expected the $0 tick in:\n{svg}"
    );
    assert!(
        svg.contains(">$20B</text>"),
        "expected the $20B tick in:\n{svg}"
    );
    assert!(
        svg.contains(">$40B</text>"),
        "expected the $40B tick in:\n{svg}"
    );
    assert!(
        !svg.contains(">$0.0000E0</text>"),
        "the raw automatic format leaked through:\n{svg}"
    );

    Ok(())
}

/// Legend entries use the same formatter pipeline as axes.
#[test]
fn legend_labels_can_be_transformed() -> Result<(), Box<dyn Error>> {
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let y = vec![1.0, 2.0, 3.0, 4.0];
    let group = vec!["north", "south", "north", "south"];

    let svg = chart!(x, y, group)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y"), alt::color("group")))?
        .with_legend_label_format(
            LabelFormat::new().with_text_formatter(|label| label.to_uppercase()),
        )
        .to_svg()?;

    assert!(svg.contains(">NORTH</text>"), "expected NORTH in:\n{svg}");
    assert!(svg.contains(">SOUTH</text>"), "expected SOUTH in:\n{svg}");
    assert!(
        !svg.contains(">north</text>"),
        "the original label leaked through:\n{svg}"
    );

    Ok(())
}

/// The colour legend title can be renamed independently of the data field.
#[test]
fn color_legend_title_can_be_set() -> Result<(), Box<dyn Error>> {
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let y = vec![1.0, 2.0, 3.0, 4.0];
    let group = vec!["north", "south", "north", "south"];

    let svg = chart!(x, y, group)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y"), alt::color("group")))?
        .with_color_label("Region")
        .to_svg()?;

    assert!(
        svg.contains(">Region</text>"),
        "expected the renamed legend title in:\n{svg}"
    );
    assert!(
        !svg.contains(">group</text>"),
        "the field name is still used as the legend title:\n{svg}"
    );
    // Renaming the legend must not change the data field, so the series must
    // still keep its palette colour (the first Tab10 blue).
    assert!(
        svg.contains("rgba(31,119,180,1.000)"),
        "the series lost its colour when the legend was renamed:\n{svg}"
    );

    Ok(())
}

/// The legend can be switched off entirely, reserving no layout space for it.
#[test]
fn legend_can_be_disabled() -> Result<(), Box<dyn Error>> {
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let y = vec![1.0, 2.0, 3.0, 4.0];
    let group = vec!["north", "south", "north", "south"];

    let shown = chart!(x, y, group)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y"), alt::color("group")))?
        .to_svg()?;
    let hidden = chart!(x, y, group)?
        .mark_point()?
        .encode((alt::x("x"), alt::y("y"), alt::color("group")))?
        .configure_theme(|theme| theme.with_show_legend(false))
        .to_svg()?;

    assert!(
        shown.contains(">group</text>"),
        "legend title missing when shown"
    );
    assert!(
        !hidden.contains(">group</text>"),
        "legend title still present when hidden:\n{hidden}"
    );

    Ok(())
}
