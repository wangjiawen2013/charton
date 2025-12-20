/// Normalize values linearly to a specified range
///
/// This function maps input values from their original range to a target range
/// using linear interpolation, similar to Altair's scaling approach.
///
/// # Parameters
/// * `values` - Slice of input values to normalize
/// * `range_min` - Target range minimum value
/// * `range_max` - Target range maximum value
///
/// # Returns
/// Vector of normalized values mapped to the target range
pub(crate) fn normalize_linear(values: &[f64], range_min: f64, range_max: f64) -> Vec<f64> {
    let min_val = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_val = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    // If all values are the same, return the midpoint of the range
    if (max_val - min_val).abs() < 1e-10 {
        return vec![range_min + (range_max - range_min) / 2.0; values.len()];
    }

    values
        .iter()
        .map(|&val| range_min + (val - min_val) / (max_val - min_val) * (range_max - range_min))
        .collect()
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
