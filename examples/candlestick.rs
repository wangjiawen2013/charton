//! Candlestick — an OHLC bar per period.
//!
//! A recipe: `mark_rule` draws the high–low wick (`y` to `y2`) and `mark_bar`
//! with a `y2` bound draws the open–close body as a *floating* bar. Colour tells
//! an up day from a down day. The two layers share the category axis, so the
//! bodies sit exactly on their wicks.
//!
//! The prices are a seeded random walk, so the figure looks like a real price
//! series while still rendering identically every run.

use charton::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // A tiny seeded LCG: deterministic, but with the run of a random walk.
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut noise = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    };

    let days = 22;
    let mut open = Vec::with_capacity(days);
    let mut high = Vec::with_capacity(days);
    let mut low = Vec::with_capacity(days);
    let mut close = Vec::with_capacity(days);
    let round = |v: f64| (v * 100.0).round() / 100.0;
    let mut price = 100.0;

    for _ in 0..days {
        let o = price + noise() * 0.8; // gap from the previous close
        let c = o + noise() * 2.0 + (100.0 - o) * 0.08; // move, pulled toward 100
        let h = o.max(c) + noise().abs() * 1.5 + 0.2;
        let l = o.min(c) - noise().abs() * 1.5 - 0.2;
        open.push(round(o));
        close.push(round(c));
        high.push(round(h));
        low.push(round(l));
        price = c;
    }

    let day: Vec<String> = (1..=days).map(|d| format!("07-{d:02}")).collect();
    let direction: Vec<&str> = open
        .iter()
        .zip(close.iter())
        .map(|(o, c)| if c >= o { "Up" } else { "Down" })
        .collect();

    // A floating bar (a `y2` bound) does not force a zero baseline, so the price
    // axis stays on the traded range and the candles fill the panel.
    let wick = chart!(day, low, high, direction)?
        .mark_rule()?
        .configure_rule(|rule| rule.with_stroke_width(1.0))
        .encode((
            alt::x("day"),
            alt::y("low"),
            alt::y2("high"),
            alt::color("direction"),
        ))?;

    let body = chart!(day, open, close, direction)?
        .mark_bar()?
        .configure_bar(|bar| bar.with_width(0.6))
        .encode((
            alt::x("day"),
            alt::y("open"),
            alt::y2("close"),
            alt::color("direction"),
        ))?;

    wick.and(body)
        .configure_theme(|theme| {
            // Chinese-market convention: red for an up day, green for a down day.
            theme
                .with_palette(ColorPalette::from(vec!["#e53935", "#43a047"]))
                .with_x_tick_label_angle(-45.0)
        })
        .with_title("Daily price")
        .with_x_label("")
        .with_y_label("Price")
        .with_color_label("")
        .save("docs/src/images/candlestick.svg")?;

    println!("Saved docs/src/images/candlestick.svg");
    Ok(())
}
