use crate::core::legend::{LegendSpec, LegendPosition};
use crate::theme::Theme;
use crate::scale::ScaleDomain;

/// Internal helper structure representing the whitespace or reserved area 
/// on each side of the plot panel caused by legend placement.
///
/// Encapsulates the pixel dimensions required to accommodate the legend.
/// 
/// This is used during the 'Resolution' phase to subtract space from the 
/// total canvas area, resulting in the final 'Plot Panel' Rect.
#[derive(Default, Debug, Clone, Copy)]
pub struct LegendLayoutConstraints {
    /// Space to be reserved at the top (used by LegendPosition::Top)
    pub top: f64,
    /// Space to be reserved at the bottom (used by LegendPosition::Bottom)
    pub bottom: f64,
    /// Space to be reserved on the left (used by LegendPosition::Left)
    pub left: f64,
    /// Space to be reserved on the right (used by LegendPosition::Right)
    pub right: f64,
}

/// The `LayoutEngine` is responsible for the geometric partitioning of the chart canvas.
/// 
/// In the Grammar of Graphics, the layout phase must occur after data synchronization 
/// but before rendering. This engine estimates the bounding boxes of non-data 
/// elements (like legends) to calculate the final 'Panel'—the area where 
/// data marks will be drawn.
pub struct LayoutEngine;

impl LayoutEngine {
    /// Dynamically calculates the space required for legends based on their 
    /// intended position and the specific data series being guided.
    /// 
    /// This method performs a "pre-flight" measurement of the legend's dimensions 
    /// to prevent overlap with the plot area.
    pub fn calculate_legend_constraints(
        specs: &[LegendSpec],
        position: LegendPosition,
        margin: f64,
        theme: &Theme,
    ) -> LegendLayoutConstraints {
        let mut constraints = LegendLayoutConstraints::default();

        // If legends are hidden or no data mappings require a legend, return empty constraints.
        if position == LegendPosition::None || specs.is_empty() {
            return constraints;
        }

        // Determine typography metrics from the theme for accurate width estimation.
        let font_size = theme.legend_font_size.unwrap_or(theme.tick_label_font_size);
        let font_family = theme.legend_font_family.as_ref().unwrap_or(&theme.tick_label_font_family);
        
        // Monospaced fonts require a larger width factor than proportional fonts.
        let width_factor = if font_family.contains("Mono") { 0.65 } else { 0.55 };

        match position {
            // Vertical layouts: Reserved space is added to the horizontal margins.
            LegendPosition::Right | LegendPosition::Left => {
                let mut max_w = 0.0;
                for spec in specs {
                    let spec_w = Self::estimate_spec_width(spec, font_size, width_factor);
                    max_w = f64::max(max_w, spec_w);
                }
                
                // Final constraint = Estimated text width + user-defined margin + canvas safety buffer.
                let total_needed = max_w + margin + 10.0; 
                
                if position == LegendPosition::Right {
                    constraints.right = total_needed;
                } else {
                    constraints.left = total_needed;
                }
            }
            
            // Horizontal layouts: Reserved space is added to the vertical margins.
            LegendPosition::Top | LegendPosition::Bottom => {
                // Calculation assumes a single-row flow layout.
                // Height includes: Title space + Marker/Label space + User Margin.
                let title_height = font_size * 1.2;
                let item_height = font_size + 20.0;
                let total_needed = title_height + item_height + margin;
                
                if position == LegendPosition::Top {
                    constraints.top = total_needed;
                } else {
                    constraints.bottom = total_needed;
                }
            }
            LegendPosition::None => {}
        }
        constraints
    }

    /// Estimates the physical width (in pixels) of a single legend block.
    /// 
    /// It compares the width of the Legend Title against the width of the 
    /// longest data label plus the geometric symbol marker.
    fn estimate_spec_width(spec: &LegendSpec, font_size: f64, width_factor: f64) -> f64 {
        let max_label_len = match &spec.domain {
            ScaleDomain::Categorical(labels) => {
                labels.iter().map(|l| l.len()).max().unwrap_or(0)
            }
            ScaleDomain::Continuous(min, max) => {
                // For continuous scales, we estimate based on formatted numeric strings.
                format!("{:.2}", min).len().max(format!("{:.2}", max).len())
            },
            _ => 10, // Default fallback for unknown domains
        };

        // Standardized marker area: 20px symbol + 10px spacing to the text.
        let symbol_area_width = 30.0;
        
        // Calculate text widths based on the resolved font factor.
        let label_text_width = max_label_len as f64 * (font_size * width_factor);
        let title_text_width = spec.title.len() as f64 * (font_size * width_factor * 1.1);

        // The block width is the wider of the title or the symbol+label combination.
        f64::max(title_text_width, symbol_area_width + label_text_width)
    }
}

/// Calculate the approximate width of a text string in SVG
///
/// This function estimates text width by categorizing characters into different width groups:
/// - Narrow characters: '.', ',', ':', ';', '!', 'i', 'j', 'l', 'I', 'J', 'L', '-', ''', '|', '1', 't', 'f', 'r'
/// - Uppercase letters: 'A'-'Z' (except those already in narrow_chars)
/// - All other characters (including lowercase letters): wide_chars
///
/// Width multipliers:
/// - Narrow characters: 0.3 * font_size
/// - Uppercase letters: 0.65 * font_size (wider than lowercase)
/// - Other characters: 0.55 * font_size
///
/// # Parameters
/// * `text` - The text string to measure
/// * `font_size` - The font size in pixels
///
/// # Returns
/// Estimated width of the text in pixels
pub(crate) fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    let mut narrow_chars = 0;
    let mut uppercase_chars = 0;
    let mut other_chars = 0;

    for c in text.chars() {
        if matches!(
            c,
            '.' | ',' | ':' | ';' | '!' | 'i' | 'j' | 'l' | '-' | '|' | '1' | 't' | 'f' | 'r'
        ) {
            narrow_chars += 1;
        } else if c.is_ascii_uppercase() {
            uppercase_chars += 1;
        } else {
            other_chars += 1;
        }
    }

    (narrow_chars as f64 * 0.3 + uppercase_chars as f64 * 0.65 + other_chars as f64 * 0.55)
        * font_size
}