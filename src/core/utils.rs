/// Estimates text width using character categorization.
pub(crate) fn estimate_text_width(text: &str, font_size: f32) -> f32 {
    let mut narrow_chars = 0;
    let mut uppercase_chars = 0;
    let mut other_chars = 0;

    for c in text.chars() {
        if matches!(c, '.'|','|':'|';'|'!'|'i'|'j'|'l'|'-'|'|'|'1'|'t'|'f'|'r') {
            narrow_chars += 1;
        } else if c.is_ascii_uppercase() {
            uppercase_chars += 1;
        } else {
            other_chars += 1;
        }
    }

    (narrow_chars as f32 * 0.3 + uppercase_chars as f32 * 0.65 + other_chars as f32 * 0.55) * font_size
}