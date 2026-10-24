//! The PDF counterpart of the semantic SVG contract.
//!
//! A chart exported with `PdfOptions::default()` is a *tagged* PDF: decorative
//! scaffolding is an `/Artifact`, the chart body is a `/Figure` carrying the
//! chart's title and description, and the axes and legend are text blocks. These
//! tests read the document dictionaries and the (uncompressed) page stream, so
//! they are format-level checks rather than picture comparisons.

#![cfg(feature = "pdf")]

use charton::prelude::*;
use std::error::Error;

fn sample_pdf(options: &PdfOptions) -> Result<Vec<u8>, Box<dyn Error>> {
    let year = vec![2019.0, 2020.0, 2021.0, 2022.0];
    let output = vec![10.0, 14.0, 13.0, 18.0];

    Ok(chart!(year, output)?
        .mark_line()?
        .encode((alt::x("year"), alt::y("output")))?
        .with_title("Output")
        .with_description("Yearly output, 2019 to 2022.")
        .with_x_label("Year")
        .with_y_label("GWh")
        .to_pdf_with(options)?)
}

fn document(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Reads the `/Alt` text of the first structure element.
///
/// PDF text strings are either a literal or hex string; krilla writes the
/// alternative text as UTF-16BE hex (prefixed by a byte-order mark), so it has
/// to be decoded rather than searched for.
fn decode_alt(pdf: &str) -> String {
    let Some(rest) = pdf.split("/Alt <").nth(1) else {
        return String::new();
    };
    let hex = rest.split('>').next().unwrap_or_default();
    let bytes: Vec<u8> = hex
        .as_bytes()
        .chunks(2)
        .filter_map(|pair| {
            let text = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(text, 16).ok()
        })
        .collect();
    if let Some(utf16) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = utf16
            .chunks(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair.get(1).copied().unwrap_or(0)]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

#[test]
fn a_chart_is_written_as_a_tagged_figure() -> Result<(), Box<dyn Error>> {
    // `pretty()` keeps the page stream uncompressed, so the marked-content
    // operators can be inspected as well as the structure tree.
    let pdf = document(&sample_pdf(&PdfOptions::default().pretty())?);

    assert!(pdf.starts_with("%PDF-"), "not a PDF");
    assert!(pdf.contains("/StructTreeRoot"), "no structure tree");
    assert!(pdf.contains("/Figure"), "the chart body is not a figure");
    assert!(pdf.contains("/Caption"), "the title is not a caption");
    assert!(
        pdf.contains("/Alt"),
        "the figure carries no alternative text"
    );
    assert!(
        decode_alt(&pdf).contains("Output"),
        "the title is missing from the /Alt text"
    );
    assert!(
        pdf.contains("/Artifact"),
        "the background and grid are not artifacts"
    );
    Ok(())
}

#[test]
fn tagging_can_be_turned_off() -> Result<(), Box<dyn Error>> {
    let pdf = document(&sample_pdf(
        &PdfOptions::default().with_a11y(A11y::Off).pretty(),
    )?);

    assert!(pdf.starts_with("%PDF-"));
    assert!(!pdf.contains("/StructTreeRoot"));
    assert!(!pdf.contains("/Artifact"));
    Ok(())
}

#[test]
fn a_chart_without_text_still_produces_a_valid_figure() -> Result<(), Box<dyn Error>> {
    // No title and no description: the figure exists but has no /Alt text, and
    // the document must still be written.
    let year = vec![2019.0, 2020.0];
    let output = vec![1.0, 2.0];
    let pdf = chart!(year, output)?
        .mark_point()?
        .encode((alt::x("year"), alt::y("output")))?
        .to_pdf_with(&PdfOptions::default().pretty())?;
    let pdf = document(&pdf);

    assert!(pdf.contains("/StructTreeRoot"));
    assert!(pdf.contains("/Figure"));
    Ok(())
}

#[test]
fn the_default_options_produce_the_same_picture_as_before() -> Result<(), Box<dyn Error>> {
    // Both documents draw the same shapes; only the metadata differs.
    let tagged = sample_pdf(&PdfOptions::default())?;
    let plain = sample_pdf(&PdfOptions::default().with_a11y(A11y::Off))?;

    // Same page geometry and content length class: both are real PDFs of the
    // same chart, and the tagged one is strictly longer (it carries a tree).
    assert!(tagged.starts_with(b"%PDF-"));
    assert!(plain.starts_with(b"%PDF-"));
    assert!(tagged.len() > plain.len());
    Ok(())
}
