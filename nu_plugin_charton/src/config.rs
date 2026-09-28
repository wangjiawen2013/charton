//! Plugin configuration, read from `$env.config.plugins.charton`.
//!
//! Precedence is: command-line flag > plugin config > built-in default.
//!
//! ```nu
//! $env.config.plugins.charton = {
//!     width: 1000
//!     height: 700
//!     scale: 2.0
//!     inline_style: kitty
//!     grid: true
//!     palette: tab10            # or a list like ["#333" "#6fc481" "red"]
//!     legend: bottom            # left | right | top | bottom | none
//!     x_angle: -45
//!     background: "#ffffff"
//! }
//! ```

use charton::prelude::{ColorPalette, LegendPosition};
use nu_plugin::EngineInterface;
use nu_protocol::{Record, Value};

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
    pub inline_style: Option<String>,
    pub palette: Option<PaletteSetting>,
    pub grid: Option<bool>,
    pub legend: Option<LegendSetting>,
    pub x_angle: Option<f64>,
    pub background: Option<String>,
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

        Ok(Self {
            width: int_field(val, "width").map(|v| v.max(16) as u32),
            height: int_field(val, "height").map(|v| v.max(16) as u32),
            scale: float_field(val, "scale").map(|v| v as f32),
            inline_style: str_field(val, "inline_style").or_else(|| str_field(val, "inline-style")),
            palette,
            grid: bool_field(val, "grid"),
            legend,
            x_angle: float_field(val, "x_angle").or_else(|| float_field(val, "x-angle")),
            background: str_field(val, "background"),
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
