//! Font-path reproduction for the CPU raster (PNG) backend.
//!
//! Renders the issue's minimal skeleton. With no arguments it renders the four
//! reported cases using font families that exist on the current OS, and writes
//! the PNGs under `target/example-output/font-issue/`:
//!
//! ```text
//! cargo run --example chinese --features png
//! ```
//!
//! Or render a single chart by passing a title and an optional family:
//!
//! ```text
//! cargo run --example chinese --features png -- "销售额（m²）" "Hiragino Sans GB"
//! ```
//!
//! On macOS, `"PingFang SC"` and `"Hiragino Sans GB"` reproduce reports 3 and 4
//! exactly; on Windows, `"Microsoft YaHei"` and `"SimHei"` play the same roles.
//! The automated version of these checks lives in
//! `src/render/backend/raster/font_tests.rs` (`cargo test --features png --lib`).

/// The same font roles the issue reports, using whatever the host OS ships.
#[cfg(feature = "png")]
mod demo {
    use std::path::{Path, PathBuf};

    use charton::prelude::*;

    fn render(title: &str, family: Option<&str>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let ds = Dataset::new()
            .with_column("x", vec![1.0, 2.0, 3.0])?
            .with_column("y", vec![2.0, 3.0, 5.0])?;
        let lc: LayeredChart = Chart::build(ds)?
            .mark_point()?
            .encode((alt::x("x"), alt::y("y")))?
            .with_title(title)
            .with_size(400, 200);
        let lc = match family {
            Some(f) => lc.configure_theme(|t| {
                t.with_title_family(f)
                    .with_label_family(f)
                    .with_tick_label_family(f)
                    .with_legend_label_family(f)
            }),
            None => lc,
        };
        Ok(lc.to_png()?)
    }

    fn latin_only() -> &'static str {
        if cfg!(target_os = "macos") {
            "Helvetica"
        } else if cfg!(target_os = "windows") {
            "Arial"
        } else {
            "DejaVu Sans"
        }
    }

    fn cjk() -> &'static str {
        if cfg!(target_os = "macos") {
            "PingFang SC"
        } else if cfg!(target_os = "windows") {
            "Microsoft YaHei"
        } else {
            "Noto Sans CJK SC"
        }
    }

    /// A CJK face that lacks `²`/`³`/`µ` (report 3b).
    fn cjk_without_superscripts() -> &'static str {
        if cfg!(target_os = "macos") {
            "Hiragino Sans GB"
        } else if cfg!(target_os = "windows") {
            "SimHei"
        } else {
            "WenQuanYi Zen Hei"
        }
    }

    fn out_dir() -> PathBuf {
        Path::new("target/example-output/font-issue").to_path_buf()
    }

    fn save(
        name: &str,
        title: &str,
        family: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = out_dir();
        std::fs::create_dir_all(&dir)?;
        let png = match render(title, family) {
            Ok(png) => png,
            // A family this OS does not ship is reported, not fatal: the same
            // example runs on every platform and in CI.
            Err(e) => {
                println!(
                    "  {:<24} {:<22} -> skipped: {e}",
                    title,
                    family.unwrap_or("-")
                );
                return Ok(());
            }
        };
        let path = dir.join(format!("{name}.png"));
        std::fs::write(&path, png)?;
        println!(
            "  {:<24} {:<22} -> {}",
            title,
            family.unwrap_or("<default stack>"),
            path.display()
        );
        Ok(())
    }

    pub fn main() -> Result<(), Box<dyn std::error::Error>> {
        let mut args = std::env::args().skip(1);
        let (Some(title), family) = (args.next(), args.next()) else {
            // Batch mode: the four reports of the issue.
            println!("Rendering the reported cases into {}:", out_dir().display());
            save("1-default-stack-cjk", "销售额趋势", None)?;
            save("2-latin-only-cjk", "销售额", Some(latin_only()))?;
            save(
                "3b-cjk-without-superscript",
                "销售额（m²）",
                Some(cjk_without_superscripts()),
            )?;
            save("3b-expected-superscript", "销售额（m²）", Some(cjk()))?;
            save("4-decimal", "2.5", Some(cjk()))?;

            // Report 2: an unknown family must be an error rather than a silent
            // fallback. Print it instead of failing the whole run.
            match render("Hello", Some("NoSuchFontXYZ123")) {
                Ok(_) => {
                    println!("  unknown family: unexpectedly rendered (report 2 is not fixed)")
                }
                Err(e) => println!("  unknown family: rejected as expected: {e}"),
            }
            return Ok(());
        };

        let family = family.as_deref();
        let name = format!(
            "custom-{}-{}",
            title
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect::<String>(),
            family.unwrap_or("default")
        );
        save(&name, &title, family)?;
        Ok(())
    }
}

#[cfg(feature = "png")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    demo::main()
}

#[cfg(not(feature = "png"))]
fn main() {
    eprintln!("This example needs the raster backend:");
    eprintln!("    cargo run --example chinese --features png");
    std::process::exit(1);
}
