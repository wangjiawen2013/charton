//! Plugin configuration, read from `$env.config.plugins.charton`.
//!
//! Precedence is: command-line flag > plugin config > built-in default.
//!
//! ```nu
//! $env.config.plugins.charton = {
//!     width: 1000
//!     height: 700
//!     scale: 2.0
//!     cell_width: 9       # terminal cell size in device px (inline fitting)
//!     cell_height: 20
//!     inline_style: kitty
//!     grid: true
//!     palette: tab10            # or a list like ["#333" "#6fc481" "red"]
//!     legend: bottom            # left | right | top | bottom | none
//!     x_angle: -45
//!     background: "#ffffff"
//!     theme: dark               # auto | light | dark
//!     color_map: viridis        # continuous color scheme (heatmaps, density)
//! }
//! ```
//!
//! Unknown keys are ignored with a warning on stderr, so typos surface instead
//! of silently doing nothing.

use charton::prelude::{ColorMap, ColorPalette, LegendPosition};
use nu_plugin::EngineInterface;
use nu_protocol::{Record, Value};

/// Every config key `Config::load` understands, including kebab-case aliases.
const KNOWN_KEYS: &[&str] = &[
    "width",
    "height",
    "scale",
    "cell_width",
    "cell-width",
    "cell_height",
    "cell-height",
    "inline_style",
    "inline-style",
    "palette",
    "color_map",
    "color-map",
    "grid",
    "legend",
    "x_angle",
    "x-angle",
    "background",
    "theme",
];

/// A configured palette: either a named built-in or an explicit color list.
#[derive(Clone, Debug)]
pub enum PaletteSetting {
    Named(String),
    Custom(Vec<String>),
}

/// Where (or whether) to show the legend.
#[derive(Clone, Copy, Debug)]
pub enum LegendSetting {
    Position(LegendPosition),
    Off,
}

impl LegendSetting {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "none" | "off" | "false" | "hidden" => Ok(Self::Off),
            "left" => Ok(Self::Position(LegendPosition::Left)),
            "right" => Ok(Self::Position(LegendPosition::Right)),
            "top" => Ok(Self::Position(LegendPosition::Top)),
            "bottom" => Ok(Self::Position(LegendPosition::Bottom)),
            other => Err(format!(
                "unknown legend position '{other}'; expected left, right, top, bottom, or none"
            )),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub scale: Option<f32>,
    /// Assumed terminal cell size in device pixels, used to fit the raster
    /// resolution of inline images. Tune if inline charts look soft or tiny.
    pub cell_width: Option<u32>,
    pub cell_height: Option<u32>,
    pub inline_style: Option<String>,
    pub palette: Option<PaletteSetting>,
    pub grid: Option<bool>,
    pub legend: Option<LegendSetting>,
    pub x_angle: Option<f64>,
    pub background: Option<String>,
    /// Preferred color theme: `auto`, `light` or `dark`.
    pub theme: Option<String>,
    /// Continuous color scheme for heatmaps / density plots.
    pub color_map: Option<ColorMap>,
}

impl Config {
    /// Load `$env.config.plugins.charton`, if present.
    pub fn load(engine: &EngineInterface) -> Result<Self, String> {
        let Some(value) = engine.get_plugin_config().map_err(|e| e.to_string())? else {
            return Ok(Self::default());
        };
        let Value::Record { val, .. } = &value else {
            return Err("$env.config.plugins.charton must be a record".to_string());
        };

        let legend = match val.get("legend") {
            Some(Value::Bool { val: false, .. }) => Some(LegendSetting::Off),
            Some(Value::String { val, .. }) => Some(LegendSetting::parse(val)?),
            _ => None,
        };

        let palette = match val.get("palette") {
            Some(Value::String { val, .. }) => Some(PaletteSetting::Named(val.clone())),
            Some(Value::List { vals, .. }) => {
                let colors: Vec<String> = vals
                    .iter()
                    .filter_map(|v| v.as_str().ok().map(str::to_string))
                    .collect();
                (!colors.is_empty()).then_some(PaletteSetting::Custom(colors))
            }
            _ => None,
        };

        let color_map = match str_field(val, "color_map").or_else(|| str_field(val, "color-map")) {
            Some(name) => Some(to_color_map(&name)?),
            None => None,
        };

        // Surface typos: unknown keys do nothing, which is otherwise invisible.
        for key in val.columns() {
            if !KNOWN_KEYS.contains(&key.as_str()) {
                eprintln!("[charton] warning: unknown config key `{key}` ignored");
            }
        }

        Ok(Self {
            width: int_field(val, "width").map(|v| v.max(16) as u32),
            height: int_field(val, "height").map(|v| v.max(16) as u32),
            scale: float_field(val, "scale").map(|v| v as f32),
            cell_width: int_field(val, "cell_width")
                .or_else(|| int_field(val, "cell-width"))
                .map(|v| v.max(1) as u32),
            cell_height: int_field(val, "cell_height")
                .or_else(|| int_field(val, "cell-height"))
                .map(|v| v.max(1) as u32),
            inline_style: str_field(val, "inline_style").or_else(|| str_field(val, "inline-style")),
            palette,
            grid: bool_field(val, "grid"),
            legend,
            x_angle: float_field(val, "x_angle").or_else(|| float_field(val, "x-angle")),
            background: str_field(val, "background"),
            theme: str_field(val, "theme"),
            color_map,
        })
    }
}

/// Convert a palette setting into a charton [`ColorPalette`].
pub fn to_color_palette(setting: &PaletteSetting) -> Result<ColorPalette, String> {
    match setting {
        PaletteSetting::Named(name) => match name.to_lowercase().replace(['-', '_'], "").as_str() {
            "tab10" => Ok(ColorPalette::Tab10),
            "tab20" => Ok(ColorPalette::Tab20),
            "set1" => Ok(ColorPalette::Set1),
            "set2" => Ok(ColorPalette::Set2),
            "set3" => Ok(ColorPalette::Set3),
            "pastel1" => Ok(ColorPalette::Pastel1),
            "pastel2" => Ok(ColorPalette::Pastel2),
            "dark2" => Ok(ColorPalette::Dark2),
            "accent" => Ok(ColorPalette::Accent),
            other => Err(format!(
                "unknown palette '{other}'; try tab10, tab20, set1, set2, set3, \
                 pastel1, pastel2, dark2, accent, or a list of colors"
            )),
        },
        PaletteSetting::Custom(colors) => {
            let refs: Vec<&str> = colors.iter().map(String::as_str).collect();
            Ok(ColorPalette::from(refs))
        }
    }
}

/// Convert a color-map name into a charton [`ColorMap`].
///
/// Names accept `-`, `_` and spaces as separators, so `yl-gn-bu`, `YlGnBu` and
/// `ylorgn` all resolve.
pub fn to_color_map(name: &str) -> Result<ColorMap, String> {
    let key = name.to_lowercase().replace(['-', '_', ' '], "");
    Ok(match key.as_str() {
        "viridis" => ColorMap::Viridis,
        "inferno" => ColorMap::Inferno,
        "magma" => ColorMap::Magma,
        "plasma" => ColorMap::Plasma,
        "cividis" => ColorMap::Cividis,
        "blues" => ColorMap::Blues,
        "greens" => ColorMap::Greens,
        "greys" | "grays" => ColorMap::Greys,
        "oranges" => ColorMap::Oranges,
        "purples" => ColorMap::Purples,
        "reds" => ColorMap::Reds,
        "bugn" => ColorMap::BuGn,
        "bupu" => ColorMap::BuPu,
        "gnbu" => ColorMap::GnBu,
        "orrd" => ColorMap::OrRd,
        "pubugn" => ColorMap::PuBuGn,
        "pubu" => ColorMap::PuBu,
        "purd" => ColorMap::PuRd,
        "rdpu" => ColorMap::RdPu,
        "ylgnbu" => ColorMap::YlGnBu,
        "ylgn" => ColorMap::YlGn,
        "ylorbr" => ColorMap::YlOrBr,
        "ylorrd" => ColorMap::YlOrRd,
        "rainbow" => ColorMap::Rainbow,
        "jet" => ColorMap::Jet,
        "hot" => ColorMap::Hot,
        "cool" => ColorMap::Cool,
        other => {
            return Err(format!(
                "unknown color map '{other}'; try viridis, inferno, magma, plasma, 
                 cividis, blues, greens, greys, oranges, purples, reds, or a 
                 multi-hue map like ylgnbu"
            ));
        }
    })
}

fn int_field(rec: &Record, key: &str) -> Option<i64> {
    rec.get(key).and_then(|v| v.as_int().ok())
}

fn float_field(rec: &Record, key: &str) -> Option<f64> {
    rec.get(key).and_then(|v| {
        v.as_float()
            .ok()
            .or_else(|| v.as_int().ok().map(|i| i as f64))
    })
}

fn bool_field(rec: &Record, key: &str) -> Option<bool> {
    rec.get(key).and_then(|v| v.as_bool().ok())
}

fn str_field(rec: &Record, key: &str) -> Option<String> {
    rec.get(key)
        .and_then(|v| v.as_str().ok().map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_palette_resolves() {
        assert!(to_color_palette(&PaletteSetting::Named("tab10".into())).is_ok());
        assert!(to_color_palette(&PaletteSetting::Named("dark-2".into())).is_ok());
        assert!(to_color_palette(&PaletteSetting::Named("nope".into())).is_err());
    }

    #[test]
    fn color_map_resolves_aliases() {
        assert!(matches!(to_color_map("viridis"), Ok(ColorMap::Viridis)));
        assert!(matches!(to_color_map("Magma"), Ok(ColorMap::Magma)));
        // Separators are ignored, so kebab/snake/camel case all work.
        assert!(matches!(to_color_map("yl-gn-bu"), Ok(ColorMap::YlGnBu)));
        assert!(matches!(to_color_map("YlGnBu"), Ok(ColorMap::YlGnBu)));
        assert!(matches!(to_color_map("grays"), Ok(ColorMap::Greys)));
        assert!(to_color_map("nope").is_err());
    }

    #[test]
    fn custom_palette_from_list() {
        let p = PaletteSetting::Custom(vec!["#333".into(), "red".into()]);
        assert!(to_color_palette(&p).is_ok());
    }

    #[test]
    fn legend_parsing() {
        assert!(matches!(
            LegendSetting::parse("off").unwrap(),
            LegendSetting::Off
        ));
        assert!(matches!(
            LegendSetting::parse("bottom").unwrap(),
            LegendSetting::Position(LegendPosition::Bottom)
        ));
        assert!(LegendSetting::parse("middle").is_err());
    }
}
