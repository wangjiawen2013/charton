//! Helpers shared by the SVG output tests.
//!
//! Each integration test crate compiles its own copy, so a helper a given file
//! does not use would otherwise warn.
#![allow(dead_code)]
//!
//! The tests assert on the *document*, so they need to find a semantic scope
//! without depending on the exact serialisation of unrelated elements. Locating
//! a group by its class and then walking its children is exactly what a page
//! stylesheet does, which makes the tests a second consumer of the semantic
//! contract: if the classes disappear, these tests fail.

/// The value of `name="…"` in one line of SVG, parsed as a number.
pub fn attr(line: &str, name: &str) -> f64 {
    attr_str(line, name)
        .parse()
        .unwrap_or_else(|_| panic!("attribute {name} should be numeric in: {line}"))
}

/// The value of `name="…"` in one line of SVG.
pub fn attr_str<'a>(line: &'a str, name: &str) -> &'a str {
    line.split(&format!("{name}=\""))
        .nth(1)
        .and_then(|value| value.split('"').next())
        .unwrap_or_else(|| panic!("expected attribute {name} in: {line}"))
}

/// Whether `class` is one of the tokens in any `class="…"` attribute.
pub fn has_class(svg: &str, class: &str) -> bool {
    let mut rest = svg;
    while let Some(found) = rest.find(" class=\"") {
        let after = found + " class=\"".len();
        rest = &rest[after..];
        let list = &rest[..rest.find('"').expect("unterminated class attribute")];
        if list.split(' ').any(|token| token == class) {
            return true;
        }
    }
    false
}

/// Every drawn primitive, in document order.
///
/// This is what "the same picture" means concretely: metadata may change, the
/// shapes and their coordinates may not.
pub fn shapes(svg: &str) -> Vec<&str> {
    svg.lines()
        .filter(|line| {
            ["<rect", "<line", "<path", "<circle", "<polygon", "<text"]
                .iter()
                .any(|start| line.starts_with(start))
        })
        .collect()
}

/// The body of the `<g>` element whose opening tag contains `marker`.
///
/// Nesting is tracked, so the returned slice is the whole subtree of that group
/// rather than its first child.
pub fn group_body<'a>(svg: &'a str, marker: &str) -> &'a str {
    let found = svg
        .find(marker)
        .unwrap_or_else(|| panic!("no element matching {marker}"));
    let open = svg[..found]
        .rfind('<')
        .unwrap_or_else(|| panic!("{marker} is not inside an element"));
    assert!(
        svg[open..].starts_with("<g"),
        "{marker} is not a group: {}",
        &svg[open..(open + 40).min(svg.len())]
    );

    let body_start = open + svg[open..].find('>').expect("unterminated open tag") + 1;
    let mut depth = 1usize;
    let mut index = body_start;
    while index < svg.len() {
        let rest = &svg[index..];
        if rest.starts_with("</g>") {
            depth -= 1;
            if depth == 0 {
                return &svg[body_start..index];
            }
            index += 4;
        } else if rest.starts_with("<g") {
            depth += 1;
            index += 2;
        } else {
            index += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    panic!("unbalanced <g> around {marker}");
}

/// The plot panel rectangle, taken from the clip path the plot group uses.
pub fn plot_rect(svg: &str) -> (f64, f64, f64, f64) {
    let line = svg
        .lines()
        .find(|line| line.contains("<clipPath id="))
        .expect("expected the plot panel clip rect");
    (
        attr(line, "x"),
        attr(line, "y"),
        attr(line, "width"),
        attr(line, "height"),
    )
}

/// The body of the plot scope, i.e. everything the clip protects.
pub fn plot_body(svg: &str) -> &str {
    group_body(svg, r#"class="charton-plot""#)
}

/// Reduces an SVG document to the part that is *drawn*.
///
/// Two documents that render the same picture may still differ in metadata: the
/// id namespace is derived from the chart structure (so two charts on one page
/// cannot share `url(#…)` targets), and the mark identity names the layer's mark
/// type (`contour` against `path`). Neither changes a single coordinate, so both
/// are removed before comparing.
pub fn drawing_only(svg: &str) -> String {
    // 1. Drop identity tokens from class lists: `charton-mark-<type>` stays
    //    only when the suffix is the layer index.
    let mut pass = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(found) = rest.find(" class=\"") {
        let after = found + " class=\"".len();
        pass.push_str(&rest[..after]);
        rest = &rest[after..];
        let end = rest.find('"').expect("unterminated class attribute");
        let (list, tail) = rest.split_at(end);
        let kept: Vec<&str> = list
            .split(' ')
            .filter(|token| match token.strip_prefix("charton-mark-") {
                Some(identity) => identity.is_empty() || identity.parse::<usize>().is_ok(),
                None => true,
            })
            .collect();
        pass.push_str(&kept.join(" "));
        rest = tail;
    }
    pass.push_str(rest);

    // 2. Drop the `data-mark` attribute.
    let mut without_mark_attr = String::with_capacity(pass.len());
    let mut rest = pass.as_str();
    while let Some(found) = rest.find(" data-mark=\"") {
        without_mark_attr.push_str(&rest[..found]);
        let after = found + " data-mark=\"".len();
        let tail = &rest[after..];
        rest = &tail[tail.find('"').expect("unterminated data-mark") + 1..];
    }
    without_mark_attr.push_str(rest);

    // 3. Drop the generated id namespace (`charton-<8 hex>-…`).
    let mut out = String::with_capacity(without_mark_attr.len());
    let mut rest = without_mark_attr.as_str();
    while let Some(found) = rest.find("charton-") {
        let after = found + "charton-".len();
        out.push_str(&rest[..after]);
        rest = &rest[after..];
        let hex = rest
            .bytes()
            .take(8)
            .take_while(u8::is_ascii_hexdigit)
            .count();
        if hex == 8 && rest.as_bytes().get(8) == Some(&b'-') {
            rest = &rest[9..];
        }
    }
    out.push_str(rest);
    out
}
