//! Rendering helpers: charton chart → SVG / PNG, and PNG → inline terminal art.
//!
//! Three inline tiers are supported, highest fidelity first:
//!
//! 1. **Kitty graphics protocol** — Kitty, Ghostty, foot, recent WezTerm.
//! 2. **iTerm2 inline images** (`OSC 1337`) — iTerm2, WezTerm, many others.
//! 3. **Truecolor half-blocks** (`▀`) — universal fallback, works everywhere.
//!
//! The style is auto-detected from the environment but can be overridden.

use base64::Engine as _;
use charton::prelude::LayeredChart;
use icy_sixel::{BackgroundMode, PixelAspectRatio, SixelImage};
use std::io::Cursor;

/// Assumed terminal cell size in device pixels, used to pick a raster
/// resolution that matches what the terminal can actually show. Modern
/// monospace fonts (~11-12pt at 96 DPI) land around 9x20; tune it with the
/// `cell_width`/`cell_height` config keys. Rendering larger than the cell is
/// safer than smaller, because downscaling looks better than upscaling.
pub const DEFAULT_CELL_W: usize = 9;
pub const DEFAULT_CELL_H: usize = 20;

/// Cell size assumed for the half-block fallback. One character always covers a
/// 1x2 block of pixels, so this ratio is fixed at 2:1 regardless of the font.
const HALFBLOCK_CELL_W: usize = 8;
const HALFBLOCK_CELL_H: usize = 16;

/// Pick a raster scale factor so the chart's PNG is about as large as the
/// terminal's display area (`max_cols * cell_w` by `max_rows * cell_h`).
///
/// Rendering at the display resolution avoids the heavy, non-integer downscale
/// that terminals otherwise apply to a fixed-size image, which is what blurs
/// text and makes horizontal and vertical strokes appear to differ in width.
/// The chart's logical layout is unchanged; only the pixel density is fitted.
pub fn inline_scale(
    logical_w: u32,
    logical_h: u32,
    max_cols: usize,
    max_rows: usize,
    cell_w: usize,
    cell_h: usize,
) -> f32 {
    let logical_w = logical_w.max(1) as f32;
    let logical_h = logical_h.max(1) as f32;
    let target_w = (max_cols.max(1) * cell_w.max(1)) as f32;
    let target_h = (max_rows.max(1) * cell_h.max(1)) as f32;
    (target_w / logical_w)
        .min(target_h / logical_h)
        .clamp(0.25, 8.0)
}

/// Which inline image protocol to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineStyle {
    /// Detect from the environment.
    Auto,
    /// Universal ANSI truecolor half-blocks.
    HalfBlock,
    /// iTerm2 `OSC 1337;File=...` inline image protocol.
    Iterm2,
    /// Kitty graphics protocol.
    Kitty,
    /// DEC Sixel graphics.
    Sixel,
}

impl InlineStyle {
    /// Parse the `--inline-style` flag value.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().replace(['-', '_'], "").as_str() {
            "auto" => Ok(Self::Auto),
            "halfblock" | "half" | "blocks" => Ok(Self::HalfBlock),
            "iterm2" | "iterm" => Ok(Self::Iterm2),
            "kitty" => Ok(Self::Kitty),
            "sixel" => Ok(Self::Sixel),
            other => Err(format!(
                "unknown inline style '{other}'; expected auto, halfblock, iterm2, kitty, or sixel"
            )),
        }
    }

    /// Detect the best available protocol from environment variables.
    pub fn detect() -> Self {
        Self::detect_from(|key| std::env::var(key).ok())
    }

    /// Testable core of [`InlineStyle::detect`].
    fn detect_from(getenv: impl Fn(&str) -> Option<String>) -> Self {
        let var = |k: &str| getenv(k);

        // An explicit override wins, so users on a remote host can force a
        // protocol without touching their Nushell config.
        if let Some(v) = var("CHARTON_INLINE_STYLE")
            && let Ok(style) = Self::parse(&v)
            && style != Self::Auto
        {
            return style;
        }

        // Program identity. These are set by the terminal emulator itself, so
        // on a remote host they only exist when the SSH client forwards them.
        if let Some(tp) = var("TERM_PROGRAM") {
            let tp = tp.to_lowercase();
            if tp.contains("iterm") || tp.contains("wezterm") {
                return Self::Iterm2;
            }
            if tp.contains("ghostty") {
                return Self::Kitty;
            }
        }
        if var("KITTY_WINDOW_ID").is_some() || var("GHOSTTY_RESOURCES_DIR").is_some() {
            return Self::Kitty;
        }
        if var("WEZTERM_EXECUTABLE").is_some()
            || var("WEZTERM_PANE").is_some()
            || var("ITERM_SESSION_ID").is_some()
        {
            return Self::Iterm2;
        }

        // `TERM` *is* forwarded by SSH even when the program-specific variables
        // above are not, so it is the reliable hint on a remote server.
        if let Some(term) = var("TERM") {
            let term = term.to_lowercase();
            if term.contains("wezterm") || term.contains("iterm") {
                return Self::Iterm2;
            }
            if term.contains("kitty") || term.contains("ghostty") {
                return Self::Kitty;
            }
            if term.contains("sixel") || term.starts_with("foot") || term.starts_with("mlterm") {
                return Self::Sixel;
            }
        }

        Self::HalfBlock
    }
}

/// Render the chart to an SVG string.
pub fn to_svg(chart: &LayeredChart) -> Result<String, String> {
    chart.to_svg().map_err(|e| e.to_string())
}

/// Render the chart to PNG bytes.
pub fn to_png(chart: &LayeredChart) -> Result<Vec<u8>, String> {
    chart.to_png().map_err(|e| e.to_string())
}

/// Read the pixel dimensions from a PNG's IHDR chunk.
pub fn png_dimensions(png_bytes: &[u8]) -> Result<(usize, usize), String> {
    // 8-byte signature + 4-byte length + 4-byte "IHDR" + 4-byte width + 4-byte height
    if png_bytes.len() < 24 || &png_bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err("not a PNG".to_string());
    }
    let read_u32 = |o: usize| {
        u32::from_be_bytes([
            png_bytes[o],
            png_bytes[o + 1],
            png_bytes[o + 2],
            png_bytes[o + 3],
        ]) as usize
    };
    Ok((read_u32(16), read_u32(20)))
}

/// Fit an image into a terminal cell budget, preserving aspect ratio.
///
/// A terminal cell is roughly twice as tall as it is wide, so one cell covers
/// `1 x 2` square pixels.
pub fn fit_cells(img_w: usize, img_h: usize, max_cols: usize, max_rows: usize) -> (usize, usize) {
    fit_cells_with_cell(
        img_w,
        img_h,
        max_cols,
        max_rows,
        HALFBLOCK_CELL_W,
        HALFBLOCK_CELL_H,
    )
}

/// Like [`fit_cells`], but for an explicit terminal cell size. Using the real
/// cell aspect ratio keeps the display box the same shape as the image, so the
/// terminal does not stretch it (which makes horizontal and vertical strokes
/// appear to have different widths).
pub fn fit_cells_with_cell(
    img_w: usize,
    img_h: usize,
    max_cols: usize,
    max_rows: usize,
    cell_w: usize,
    cell_h: usize,
) -> (usize, usize) {
    let max_cols = max_cols.max(1);
    let max_rows = max_rows.max(1);
    if img_w == 0 || img_h == 0 {
        return (max_cols.min(1), max_rows.min(1));
    }
    // cols / rows such that (cols*cell_w) : (rows*cell_h) == img_w : img_h.
    let ratio = (cell_h.max(1) as f64 / cell_w.max(1) as f64) * (img_w as f64 / img_h as f64);
    let mut rows = max_rows;
    let mut cols = (rows as f64 * ratio).round() as usize;
    if cols > max_cols {
        cols = max_cols;
        rows = ((cols as f64 / ratio).round() as usize).max(1);
    }
    (cols.max(1), rows.max(1))
}

/// Encode the PNG as an iTerm2 inline image, sized to `cols` x `rows` cells.
pub fn iterm2_image(png_bytes: &[u8], cols: usize, rows: usize) -> String {
    let payload = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    format!(
        "\x1b]1337;File=inline=1;size={};width={};height={};preserveAspectRatio=1;type=image/png:{}\x07\n",
        png_bytes.len(),
        cols,
        rows,
        payload
    )
}

/// Encode the PNG as a Kitty graphics protocol image, sized to `cols` x `rows`.
pub fn kitty_image(png_bytes: &[u8], cols: usize, rows: usize) -> String {
    let payload = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    let bytes = payload.as_bytes();

    let mut out = String::with_capacity(payload.len() + 128);
    // Each chunk must be <= 4096 bytes and a multiple of 4 (except the last).
    const CHUNK: usize = 4096;
    let mut offset = 0;
    let mut first = true;
    while offset < bytes.len() {
        let end = (offset + CHUNK).min(bytes.len());
        let more = usize::from(end < bytes.len());
        let chunk = &payload[offset..end];
        if first {
            out.push_str(&format!(
                "\x1b_Ga=T,f=100,q=2,m={more},c={cols},r={rows};{chunk}\x1b\\"
            ));
            first = false;
        } else {
            out.push_str(&format!("\x1b_Gm={more};{chunk}\x1b\\"));
        }
        offset = end;
    }
    // Reserve a line so the shell prompt does not overlap the image.
    out.push('\n');
    out
}

/// Encode the PNG as a DEC Sixel image fitted to the terminal cell grid.
///
/// Sixel is a pixel protocol, so we downscale the bitmap to the target area
/// first. Terminal cells are assumed to be a typical 8x16 device pixels; this
/// is approximate, which is why Sixel is normally an explicit choice.
pub fn sixel_image(
    png_bytes: &[u8],
    max_cols: usize,
    max_rows: usize,
    cell_w: usize,
    cell_h: usize,
) -> Result<String, String> {
    let (w, h, rgba) = decode_png_rgba(png_bytes)?;
    if w == 0 || h == 0 {
        return Err("image has zero size".to_string());
    }
    let (dw, dh) = fit_pixels(
        w,
        h,
        max_cols.max(1) * cell_w.max(1),
        max_rows.max(1) * cell_h.max(1),
    );
    let resized = resize_nearest_rgba(&rgba, w, h, dw, dh);

    let image = SixelImage::try_from_rgba(resized, dw, dh)
        .map_err(|e| format!("sixel: {e}"))?
        .with_aspect_ratio(PixelAspectRatio::Square)
        .with_background_mode(BackgroundMode::Transparent);

    let mut out = image.encode().map_err(|e| format!("sixel: {e}"))?;
    out.push('\n');
    Ok(out)
}

/// Scale an image to fit within a pixel box, preserving aspect ratio.
fn fit_pixels(w: usize, h: usize, box_w: usize, box_h: usize) -> (usize, usize) {
    if w == 0 || h == 0 {
        return (box_w.max(1), box_h.max(1));
    }
    let scale = f64::min(box_w as f64 / w as f64, box_h as f64 / h as f64);
    (
        ((w as f64 * scale).round() as usize).max(1),
        ((h as f64 * scale).round() as usize).max(1),
    )
}

/// Nearest-neighbour resize of an RGBA buffer.
fn resize_nearest_rgba(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(dw * dh * 4);
    for dy in 0..dh {
        let sy = (dy * sh / dh).min(sh - 1);
        for dx in 0..dw {
            let sx = (dx * sw / dw).min(sw - 1);
            let i = (sy * sw + sx) * 4;
            out.extend_from_slice(&src[i..i + 4]);
        }
    }
    out
}

/// Convert PNG bytes into ANSI truecolor half-block art.
///
/// Each character cell carries two vertically stacked pixels via the upper
/// half block `▀`: the foreground color is the top pixel and the background
/// color is the bottom pixel.
pub fn png_to_halfblock(
    png_bytes: &[u8],
    max_cols: usize,
    max_rows: usize,
) -> Result<String, String> {
    let (w, h, rgba) = decode_png_rgba(png_bytes)?;
    if w == 0 || h == 0 {
        return Err("image has zero size".to_string());
    }

    let (cols, rows) = fit_cells(w, h, max_cols, max_rows);
    let dest_w = cols;
    let dest_h = rows * 2;

    let mut out = String::with_capacity(cols * rows * 32);
    for cy in 0..rows {
        for cx in 0..cols {
            let (tr, tg, tb) = downsample(&rgba, w, h, dest_w, dest_h, cx, cy * 2);
            let (br, bg, bb) = downsample(&rgba, w, h, dest_w, dest_h, cx, cy * 2 + 1);
            out.push_str("\x1b[38;2;");
            push_u8(&mut out, tr);
            out.push(';');
            push_u8(&mut out, tg);
            out.push(';');
            push_u8(&mut out, tb);
            out.push_str("m\x1b[48;2;");
            push_u8(&mut out, br);
            out.push(';');
            push_u8(&mut out, bg);
            out.push(';');
            push_u8(&mut out, bb);
            out.push_str("m\u{2580}");
        }
        out.push_str("\x1b[0m\n");
    }
    Ok(out)
}

/// Average the source rectangle that maps to output pixel `(dx, dy)`.
///
/// The plugin halves the raster into a `dest_w x dest_h` grid, which is a large
/// downscale. Nearest-neighbour sampling makes thin axis lines and text strokes
/// appear and disappear from cell to cell (some edges double, others vanish);
/// averaging instead keeps them visible as a lighter shade. Fully transparent
/// source pixels are ignored, so an alpha cut-out does not darken its cell.
fn downsample(
    rgba: &[u8],
    w: usize,
    h: usize,
    dest_w: usize,
    dest_h: usize,
    dx: usize,
    dy: usize,
) -> (u8, u8, u8) {
    // Half-open source span [start, end) for output index `i`; spans tile the
    // axis so no pixel is sampled twice or skipped. When upscaling (dest > src)
    // the span is empty, so fall back to the single nearest pixel.
    let span = |i: usize, dst: usize, src: usize| -> (usize, usize) {
        let start = i * src / dst;
        let end = (i + 1) * src / dst;
        if end > start {
            (start, end)
        } else {
            (start, (start + 1).min(src))
        }
    };
    let (x0, x1) = span(dx, dest_w, w);
    let (y0, y1) = span(dy, dest_h, h);
    let mut sum = [0u64; 3];
    let mut alpha = 0u64;
    for sy in y0..y1 {
        let row = sy * w;
        for sx in x0..x1 {
            let idx = (row + sx) * 4;
            let a = rgba[idx + 3] as u64;
            sum[0] += rgba[idx] as u64 * a;
            sum[1] += rgba[idx + 1] as u64 * a;
            sum[2] += rgba[idx + 2] as u64 * a;
            alpha += a;
        }
    }
    if let (Some(r), Some(g), Some(b)) = (
        sum[0].checked_div(alpha),
        sum[1].checked_div(alpha),
        sum[2].checked_div(alpha),
    ) {
        (r as u8, g as u8, b as u8)
    } else {
        (0, 0, 0)
    }
}

/// Decode a PNG into an RGBA8 buffer: `(width, height, rgba)`.
fn decode_png_rgba(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    // Expand palette / grayscale and drop 16-bit down to 8-bit so we always
    // deal with byte-per-channel data.
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info().clone();
    let width = info.width as usize;
    let height = info.height as usize;

    let mut buf = vec![
        0u8;
        reader
            .output_buffer_size()
            .ok_or_else(|| "decoded PNG is too large".to_string())?
    ];
    let frame = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let data = &buf[..frame.buffer_size()];

    let rgba = match frame.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity(width * height * 4);
            for px in data.as_chunks::<3>().0 {
                out.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
            out
        }
        png::ColorType::Grayscale => {
            let mut out = Vec::with_capacity(width * height * 4);
            for &g in data {
                out.extend_from_slice(&[g, g, g, 255]);
            }
            out
        }
        png::ColorType::GrayscaleAlpha => {
            let mut out = Vec::with_capacity(width * height * 4);
            for px in data.as_chunks::<2>().0 {
                out.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
            out
        }
        png::ColorType::Indexed => {
            let palette = info
                .palette
                .as_ref()
                .ok_or_else(|| "indexed PNG without palette".to_string())?;
            let trns = info.trns.as_ref();
            let mut out = Vec::with_capacity(width * height * 4);
            for &i in data {
                let i = i as usize;
                let r = palette.get(i * 3).copied().unwrap_or(0);
                let g = palette.get(i * 3 + 1).copied().unwrap_or(0);
                let b = palette.get(i * 3 + 2).copied().unwrap_or(0);
                let a = trns.and_then(|t| t.get(i).copied()).unwrap_or(255);
                out.extend_from_slice(&[r, g, b, a]);
            }
            out
        }
    };

    Ok((width, height, rgba))
}

fn push_u8(s: &mut String, v: u8) {
    use std::fmt::Write as _;
    let _ = write!(s, "{v}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_png() -> Vec<u8> {
        // 3x2 RGBA image encoded as PNG.
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, 3, 2);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            let data: Vec<u8> = (0..3 * 2 * 4).map(|i| i as u8).collect();
            writer.write_image_data(&data).unwrap();
        }
        out
    }

    #[test]
    fn dimensions_are_read() {
        assert_eq!(png_dimensions(&tiny_png()).unwrap(), (3, 2));
    }

    #[test]
    fn halfblock_has_expected_shape() {
        let art = png_to_halfblock(&tiny_png(), 80, 40).unwrap();
        assert!(art.contains('\u{2580}'));
        assert!(art.contains("\x1b[38;2;"));
    }

    #[test]
    fn box_downsampling_keeps_thin_lines() {
        // 20x20 white with a 1px black vertical line at x=11.
        let (w, h) = (20usize, 20usize);
        let mut rgba = vec![255u8; w * h * 4];
        for y in 0..h {
            let idx = (y * w + 11) * 4;
            rgba[idx] = 0;
            rgba[idx + 1] = 0;
            rgba[idx + 2] = 0;
        }
        // Downscale to 4 columns. The line falls inside column 2 ([10, 15));
        // nearest-neighbour at x = 2*20/4 = 10 would miss it entirely.
        let line = downsample(&rgba, w, h, 4, 4, 2, 0);
        assert!(line.0 < 255, "thin line must not vanish: {line:?}");
        // A column that does not cover the line stays white.
        assert_eq!(downsample(&rgba, w, h, 4, 4, 0, 0), (255, 255, 255));
    }

    #[test]
    fn box_downsampling_ignores_transparent_pixels() {
        // One transparent pixel and one opaque red pixel in the same cell.
        let (w, h) = (2usize, 1usize);
        let mut rgba = vec![0u8; w * h * 4];
        rgba[4] = 255; // x=1 red
        rgba[7] = 255; // x=1 alpha
        assert_eq!(
            downsample(&rgba, w, h, 1, 1, 0, 0),
            (255, 0, 0),
            "transparent pixels must not darken the cell"
        );
    }

    #[test]
    fn iterm2_frame_is_well_formed() {
        let s = iterm2_image(b"abcdef", 10, 5);
        assert!(s.starts_with("\x1b]1337;File=inline=1;size=6;width=10;height=5;"));
        assert!(s.contains("preserveAspectRatio=1"));
        assert!(s.ends_with('\n'));
    }

    #[test]
    fn kitty_chunks_long_payloads() {
        // Force a payload longer than one chunk (4096 base64 chars).
        let big = vec![0u8; 8000];
        let s = kitty_image(&big, 20, 10);
        assert!(s.starts_with("\x1b_Ga=T,f=100,q=2,m=1,c=20,r=10;"));
        // First chunk is `more`, the final chunk must be `m=0`.
        assert!(s.contains("\x1b_Gm=1;"));
        assert!(s.contains("\x1b_Gm=0;"));
        assert!(s.ends_with('\n'));
    }

    #[test]
    fn sixel_frame_is_well_formed() {
        let s = sixel_image(&tiny_png(), 40, 20, 8, 16).unwrap();
        // DEC sixel DCS introducer and string terminator.
        assert!(s.starts_with("\u{1b}P"), "got {:?}", &s[..s.len().min(8)]);
        assert!(s.trim_end().ends_with("\u{1b}\\"));
    }

    #[test]
    fn style_parsing() {
        assert_eq!(
            InlineStyle::parse("half-block").unwrap(),
            InlineStyle::HalfBlock
        );
        assert_eq!(InlineStyle::parse("iTerm2").unwrap(), InlineStyle::Iterm2);
        assert_eq!(InlineStyle::parse("KITTY").unwrap(), InlineStyle::Kitty);
        assert_eq!(InlineStyle::parse("sixel").unwrap(), InlineStyle::Sixel);
        assert!(InlineStyle::parse("nope").is_err());
    }

    #[test]
    fn inline_scale_matches_terminal_area() {
        // 119x28 cells at 8x16 px is 952x448 px. An 800x600 chart is
        // height-bound, so the scale is 448/600.
        let s = inline_scale(800, 600, 119, 28, 8, 16);
        assert!((s - 448.0 / 600.0).abs() < 1e-6, "got {s}");
        // A wide terminal lets the chart reach its full logical width.
        let s = inline_scale(800, 600, 400, 100, 8, 16);
        assert!(s > 2.0, "got {s}");
        // Degenerate inputs never panic or return zero.
        assert!(inline_scale(0, 0, 0, 0, 0, 0) > 0.0);
    }

    #[test]
    fn fit_preserves_aspect() {
        // 1000x500 (2:1) into 80x40 cells -> width-bound, 20 cells tall.
        assert_eq!(fit_cells(1000, 500, 80, 40), (80, 20));
        // Tall image (1:10) into 80x40 cells -> height-bound, 8 cells wide.
        assert_eq!(fit_cells(100, 1000, 80, 40), (8, 40));
        // A square image with non-2:1 cells still yields a matching box.
        let (c, r) = fit_cells_with_cell(600, 600, 80, 40, 8, 18);
        let box_aspect = (c * 8) as f64 / (r * 18) as f64;
        assert!((box_aspect - 1.0).abs() < 0.06, "{c}x{r} -> {box_aspect}");
    }

    /// Build a `getenv` closure from a fixed list of `(key, value)` pairs.
    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn detection_recognises_wezterm_over_ssh_via_term() {
        // Over SSH `TERM_PROGRAM`/`WEZTERM_*` are usually absent, but `TERM`
        // is forwarded. This is the Ubuntu-server-over-SSH case.
        let style = InlineStyle::detect_from(env(&[("TERM", "wezterm")]));
        assert_eq!(style, InlineStyle::Iterm2);
    }

    #[test]
    fn detection_recognises_kitty_and_foot_terms() {
        assert_eq!(
            InlineStyle::detect_from(env(&[("TERM", "xterm-kitty")])),
            InlineStyle::Kitty
        );
        assert_eq!(
            InlineStyle::detect_from(env(&[("TERM", "foot")])),
            InlineStyle::Sixel
        );
    }

    #[test]
    fn detection_prefers_term_program_hint() {
        let style = InlineStyle::detect_from(env(&[
            ("TERM_PROGRAM", "WezTerm"),
            ("TERM", "xterm-256color"),
        ]));
        assert_eq!(style, InlineStyle::Iterm2);
    }

    #[test]
    fn detection_env_override_wins() {
        let style = InlineStyle::detect_from(env(&[
            ("CHARTON_INLINE_STYLE", "halfblock"),
            ("TERM", "wezterm"),
        ]));
        assert_eq!(style, InlineStyle::HalfBlock);
    }

    #[test]
    fn detection_falls_back_to_halfblock() {
        assert_eq!(InlineStyle::detect_from(env(&[])), InlineStyle::HalfBlock);
        // A dumb terminal that matches nothing still falls back safely.
        assert_eq!(
            InlineStyle::detect_from(env(&[("TERM", "dumb")])),
            InlineStyle::HalfBlock
        );
    }
}
