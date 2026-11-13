//! Rendering helpers: charton chart → SVG / PNG, and PNG → inline terminal art.
//!
//! Four inline transports are supported:
//!
//! 1. **Kitty graphics protocol** (`ESC_G…`) — full bitmap.
//! 2. **iTerm2 inline images** (`OSC 1337`) — full bitmap.
//! 3. **Sixel** (`ESC P…q`) — full bitmap.
//! 4. **Truecolor half-blocks** (`▀`) — universal fallback, works everywhere.
//!
//! Each one produces the bytes for its own protocol; [`Frame`] adds the cursor
//! handling that makes all four leave the screen in the same state.
//!
//! The renderer is chosen explicitly (`--inline-style`, `CHARTON_INLINE_STYLE`,
//! or the `inline_style` config key). The default is the universal half-block
//! renderer; the plugin never guesses from terminal-brand environment variables.

use base64::Engine as _;
use charton::prelude::LayeredChart;
use icy_sixel::{BackgroundMode, PixelAspectRatio, SixelImage};
use std::io::{Cursor, Write};

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
            "halfblock" | "half" | "blocks" => Ok(Self::HalfBlock),
            "iterm2" | "iterm" => Ok(Self::Iterm2),
            "kitty" => Ok(Self::Kitty),
            "sixel" => Ok(Self::Sixel),
            other => Err(format!(
                "unknown inline style '{other}'; expected halfblock, iterm2, kitty, or sixel"
            )),
        }
    }

    /// The spelling `--inline-style` accepts, which is also the name used when
    /// a renderer is reported in a message.
    pub fn name(self) -> &'static str {
        match self {
            Self::HalfBlock => "halfblock",
            Self::Iterm2 => "iterm2",
            Self::Kitty => "kitty",
            Self::Sixel => "sixel",
        }
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

/// An inline chart frame: a protocol payload plus the cursor framing that makes
/// every renderer behave the same way on screen.
///
/// The frame starts at the left margin and ends at column 0 of the line below
/// the chart, so the shell prompt — or the next command's output — always lands
/// in the same place, whichever protocol drew the chart.
pub struct Frame {
    bytes: Vec<u8>,
}

impl Frame {
    /// Wrap a payload that leaves the cursor below the chart when it ends:
    /// iTerm2 and Sixel move it there as part of displaying the image, and
    /// half-block art ends its last row with a newline. All that is left to do
    /// is reset the column.
    pub fn new(payload: String) -> Self {
        Self::framed(payload, 0)
    }

    /// Wrap a payload that asks the terminal to leave the cursor where it is.
    ///
    /// The Kitty graphics protocol promises only that the cursor ends up
    /// *after* the image, which is ambiguous for an image narrower than the
    /// terminal. So the image is sent with `C=1` — do not move the cursor — and
    /// the frame reserves the `rows` the image covers itself, putting the next
    /// output exactly below the chart on every conforming terminal.
    pub fn advancing(payload: String, rows: usize) -> Self {
        Self::framed(payload, rows)
    }

    /// Lay the frame out as `CR payload (CR LF × rows) CR`.
    ///
    /// The leading CR moves to the left margin, which is the column the cell
    /// budget that sized the image assumes, and keeps a long OSC/DCS string
    /// from starting next to the right edge, where a terminal may wrap it in
    /// the middle of the sequence. The closing CR fixes the column, so a chart
    /// takes up exactly the rows it draws and never a blank line more.
    fn framed(payload: String, rows: usize) -> Self {
        let mut bytes = Vec::with_capacity(payload.len() + 2 + rows * 2);
        bytes.push(b'\r');
        bytes.extend_from_slice(payload.as_bytes());
        for _ in 0..rows {
            bytes.extend_from_slice(b"\r\n");
        }
        bytes.push(b'\r');
        Self { bytes }
    }

    /// Write the frame as one buffer.
    ///
    /// The result is returned rather than discarded: a chart that never reached
    /// the terminal — a closed pipe, a rejected escape sequence — is an I/O
    /// failure for the caller to report, not an empty result.
    pub fn write_to<W: Write>(&self, out: &mut W) -> std::io::Result<()> {
        out.write_all(&self.bytes)
    }
}

/// Encode the PNG as an iTerm2 inline image payload, sized to `cols` x `rows`
/// cells.
///
/// The payload carries only the parameters that decide how the image is shown:
/// `inline` (display it in place rather than offering to save it), `width` and
/// `height` in cells, and `preserveAspectRatio`. The type is left out because
/// the protocol infers it from the payload's magic number, which a PNG always
/// has, and the byte count is left out because it is only a progress hint.
/// Every optional parameter a payload omits is one less thing a terminal's
/// parser can get wrong.
pub fn iterm2_image(png_bytes: &[u8], cols: usize, rows: usize) -> String {
    let payload = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    format!(
        "\x1b]1337;File=inline=1;width={};height={};preserveAspectRatio=1:{}\x07",
        cols, rows, payload
    )
}

/// Encode the PNG as a Kitty graphics protocol payload, sized to `cols` x
/// `rows` cells.
///
/// The image is transmitted with `C=1`, so the terminal leaves the cursor where
/// it is and the caller reserves the image's rows instead (see
/// [`Frame::advancing`]). `q=2` keeps the terminal from answering on the shell's
/// input stream.
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
                "\x1b_Ga=T,f=100,q=2,C=1,m={more},c={cols},r={rows};{chunk}\x1b\\"
            ));
            first = false;
        } else {
            out.push_str(&format!("\x1b_Gm={more};{chunk}\x1b\\"));
        }
        offset = end;
    }
    out
}

/// Encode the PNG as a DEC Sixel image fitted to the terminal cell grid.
///
/// Sixel is a pixel protocol, so the bitmap is downscaled to the pixel area the
/// configured cell size gives the chart, and the background stays transparent so
/// the terminal's own background shows through.
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

    let out = image.encode().map_err(|e| format!("sixel: {e}"))?;
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

/// Encode the PNG as ANSI truecolor half-block art, one character per 1x2
/// block of pixels.
///
/// Each character cell carries two vertically stacked pixels via the upper
/// half block `▀`: the foreground color is the top pixel and the background
/// color is the bottom pixel. Every row is reset and terminated with CRLF, so
/// the art is a stack of independent lines that start and finish at column 0.
pub fn halfblock_image(
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
        out.push_str("\x1b[0m\r\n");
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
    fn halfblock_payload_has_expected_shape() {
        let art = halfblock_image(&tiny_png(), 80, 40).unwrap();
        assert!(art.contains('\u{2580}'));
        assert!(art.contains("\x1b[38;2;"));
        // Each row ends with CRLF so the next row (and prompt) starts at column 0.
        assert!(art.contains("\x1b[0m\r\n"));
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
    fn iterm2_payload_is_well_formed() {
        let s = iterm2_image(b"abcdef", 10, 5);
        assert!(
            s.starts_with("\x1b]1337;File=inline=1;width=10;height=5;preserveAspectRatio=1:"),
            "got {:?}",
            &s[..s.find(':').unwrap_or(s.len())]
        );
        // Size and type are optional in the protocol, so they are not sent:
        // the terminal reads the type from the payload and always knows the
        // byte count from the payload it decodes.
        assert!(!s.contains("size="), "byte count is a progress hint only");
        assert!(
            !s.contains("type="),
            "type is inferred from the magic number"
        );
        assert!(s.ends_with('\x07'), "the OSC string ends with BEL");
    }

    #[test]
    fn kitty_chunks_long_payloads() {
        // Force a payload longer than one chunk (4096 base64 chars).
        let big = vec![0u8; 8000];
        let s = kitty_image(&big, 20, 10);
        assert!(s.starts_with("\x1b_Ga=T,f=100,q=2,C=1,m=1,c=20,r=10;"));
        // First chunk is `more`, the final chunk must be `m=0`.
        assert!(s.contains("\x1b_Gm=1;"));
        assert!(s.contains("\x1b_Gm=0;"));
        assert!(s.ends_with("\x1b\\"), "the last chunk ends with ST");
    }

    #[test]
    fn sixel_payload_is_well_formed() {
        let s = sixel_image(&tiny_png(), 40, 20, 8, 16).unwrap();
        // DEC sixel DCS introducer and string terminator.
        assert!(s.starts_with("\u{1b}P"), "got {:?}", &s[..s.len().min(8)]);
        assert!(s.ends_with("\u{1b}\\"));
    }

    /// One frame per renderer, wrapped the way the command wraps them, so the
    /// cursor contract is checked for every protocol rather than one by one.
    fn all_frames() -> Vec<(&'static str, Frame)> {
        vec![
            ("iterm2", Frame::new(iterm2_image(b"abcdef", 10, 5))),
            ("kitty", Frame::advancing(kitty_image(b"abcdef", 10, 5), 5)),
            (
                "sixel",
                Frame::new(sixel_image(&tiny_png(), 40, 20, 8, 16).unwrap()),
            ),
            (
                "halfblock",
                Frame::new(halfblock_image(&tiny_png(), 40, 20).unwrap()),
            ),
        ]
    }

    #[test]
    fn frames_start_and_end_at_the_left_margin() {
        for (name, frame) in all_frames() {
            let mut out = Vec::new();
            frame.write_to(&mut out).unwrap();
            assert_eq!(out.first(), Some(&b'\r'), "{name} starts at column 0");
            assert_eq!(out.last(), Some(&b'\r'), "{name} ends at column 0");
        }
    }

    #[test]
    fn frames_carry_the_payload_unchanged() {
        let payload = iterm2_image(b"abcdef", 10, 5);
        let mut out = Vec::new();
        Frame::new(payload.clone()).write_to(&mut out).unwrap();
        assert_eq!(out, [&b"\r"[..], payload.as_bytes(), b"\r"].concat());
    }

    #[test]
    fn a_frame_can_reserve_the_rows_it_draws() {
        // Kitty is asked not to move the cursor, so the frame advances it by
        // the image's rows and the next line starts below the chart.
        let mut out = Vec::new();
        Frame::advancing("payload".to_string(), 3)
            .write_to(&mut out)
            .unwrap();
        assert_eq!(out, b"\rpayload\r\n\r\n\r\n\r");
        // Zero rows adds nothing beyond the column resync.
        let mut out = Vec::new();
        Frame::advancing("payload".to_string(), 0)
            .write_to(&mut out)
            .unwrap();
        assert_eq!(out, b"\rpayload\r");
    }

    #[test]
    fn frames_report_write_failures() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "terminal went away",
                ))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let err = Frame::new("payload".to_string())
            .write_to(&mut Closed)
            .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
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
}
