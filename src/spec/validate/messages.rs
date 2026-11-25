//! serde's first failure, turned into a located diagnostic.
//!
//! serde says what went wrong and `serde_path_to_error` says where; this module
//! joins the two and adds the hint, the candidates and the JSON Pointer.
use crate::spec::error::{SpecError, codes};
use crate::spec::types::{AggregateSpec, ExtensionMark, MarkType, ScaleTypeSpec};

/// Convert the first structural failure into a located diagnostic.
pub(super) fn structural(err: &serde_path_to_error::Error<serde_json::Error>) -> SpecError {
    use serde_json::error::Category;

    let path = err.path().to_string();
    let inner = err.inner();
    let raw = inner.to_string();

    // A hand-written deserialiser can carry the path of the field it failed on
    // ("at aggregate[0].op: ..."), because the outer path stops at the entry.
    let (located, raw) = hoist_inner_path(&path, &raw);
    let at = pointer(&located);

    if matches!(inner.classify(), Category::Syntax | Category::Eof) {
        return SpecError::new(codes::JSON_SYNTAX, "", strip_location(&raw)).with_hint(
            "The document is not valid JSON. Look for a trailing comma, an unquoted key, \
             a comment, or a truncated file.",
        );
    }

    let message = strip_location(&raw);
    let expected = expected_values(&message);
    // Everything below decides from the *resolved* location, which may be
    // deeper than where the outer deserialiser stopped.
    let suffix = last_segment(&located).to_owned();
    let path = located;

    // Transform entries are hand-deserialised, so these messages are ours and
    // the prefixes are a stable internal contract (asserted by the corpus).
    if message.starts_with("expression ") {
        return SpecError::new(codes::UNSUPPORTED_EXPRESSION, &at, message).with_hint(
            "A spec is data, not a program. Write the predicate as data:              {\"filter\": {\"field\": \"x\", \"equal\": 3}}, and compute derived              columns upstream.",
        );
    }
    if message.starts_with("unsupported ") {
        return SpecError::new(codes::UNSUPPORTED_FIELD, &at, message)
            .with_hint("Supported transforms: `filter`, `fold`, `aggregate`, `window`.");
    }
    if message.starts_with("misplaced ") {
        return SpecError::new(codes::UNKNOWN_FIELD, &at, message);
    }
    if message.starts_with("unknown transform") {
        return SpecError::new(codes::UNKNOWN_FIELD, &at, message).with_expected(["filter"]);
    }

    // `unknown field` — the word is wrong, and serde already knows the
    // candidates. The only real work is deciding which family it belongs to.
    if let Some(name) = quoted_after(&message, "unknown field ") {
        let is_channel = last_segment(&parent_path(&path)) == "encoding";
        let code = if is_channel {
            codes::UNKNOWN_CHANNEL
        } else {
            codes::UNKNOWN_FIELD
        };
        let what = if is_channel {
            "encoding channel"
        } else {
            "field"
        };
        let error =
            SpecError::new(code, &at, format!("Unknown {what} `{name}`")).with_expected(expected);
        return with_suggestion(error, &name);
    }

    // Marks are hand-deserialised, so this message is ours and can be trusted.
    if let Some(name) = quoted_after(&message, "unknown mark ") {
        let error = SpecError::new(codes::UNKNOWN_MARK, &at, format!("Unknown mark `{name}`"))
            .with_expected(MarkType::ALL.iter().map(|mark| mark.as_str()));
        return with_suggestion(error, &name);
    }

    if let Some(name) = quoted_after(&message, "unknown variant ") {
        let (code, expected, hint) = variant_family(&path, expected);
        let error = SpecError::new(code, &at, format!("Unknown value `{name}` here"))
            .with_expected(expected);
        let error = with_suggestion(error, &name);
        return match hint {
            Some(hint) => error.with_hint(hint),
            None => error,
        };
    }

    if let Some(name) = quoted_after(&message, "missing field ") {
        return SpecError::new(
            codes::MISSING_FIELD,
            &at,
            format!("Missing required field `{name}`"),
        )
        .with_hint(missing_field_hint(&suffix, &name));
    }

    if message.starts_with("invalid type") || message.starts_with("invalid value") {
        let mut error = SpecError::new(codes::TYPE_MISMATCH, &at, message).with_expected(expected);
        if suffix == "type" {
            error = error.with_hint(
                "`type` is optional. Omit it and Charton infers the type from the data.",
            );
        }
        return error;
    }

    // `data did not match any variant of untagged enum ...` — surfaces from the
    // few enums that still derive `untagged` (bin, stack).
    SpecError::new(codes::TYPE_MISMATCH, &at, message)
        .with_hint("Check the shape of this value; both spellings of it were rejected.")
}

/// Which family an unknown enum variant belongs to, from the field it sits in.
pub(super) fn variant_family(
    path: &str,
    expected: Vec<String>,
) -> (&'static str, Vec<String>, Option<&'static str>) {
    let suffix = last_segment(path);
    let parent = parent_path(path);
    match (last_segment(&parent), suffix) {
        (_, "aggregate") => (
            codes::UNKNOWN_AGGREGATE,
            AggregateSpec::ALL
                .iter()
                .map(|op| op.as_str().to_owned())
                .collect(),
            Some(
                "Vega-Lite has operators Charton does not implement; they are rejected, not approximated.",
            ),
        ),
        (_, "mark") => (
            codes::UNKNOWN_MARK,
            ExtensionMark::ALL
                .iter()
                .map(|mark| mark.as_str().to_owned())
                .collect(),
            Some("Charton-only marks live under `charton.mark`, not `mark`."),
        ),
        // `scale.type` is a scale family; `encoding.x.type` is a data type.
        ("scale", "type") => (
            codes::UNKNOWN_SCALE,
            ScaleTypeSpec::ALL
                .iter()
                .map(|kind| kind.as_str().to_owned())
                .collect(),
            Some("Only the listed scale types exist in this subset."),
        ),
        // An operator name inside a transform entry.
        (_, "op") => (
            codes::UNKNOWN_OPERATOR,
            expected,
            Some("The message above lists the operators this build knows."),
        ),
        (_, "stack") => (
            codes::TYPE_MISMATCH,
            expected,
            Some("Use \"zero\", \"normalize\", \"center\", \"none\", true or false."),
        ),
        _ => (codes::TYPE_MISMATCH, expected, None),
    }
}

/// Add a "did you mean" hint when one candidate is a near miss.
pub(super) fn with_suggestion(mut error: SpecError, name: &str) -> SpecError {
    if let Some(best) = error
        .expected
        .iter()
        .min_by_key(|candidate| edit_distance(name, candidate))
        .filter(|candidate| {
            // Only suggest when it is genuinely close: one edit for short
            // names, two for longer ones. Otherwise the hint is noise.
            let distance = edit_distance(name, candidate);
            distance <= if name.len() > 6 { 2 } else { 1 }
        })
    {
        error.hint = Some(format!("Did you mean `{best}`?"));
    }
    error
}

pub(super) fn missing_field_hint(context: &str, name: &str) -> String {
    match (context, name) {
        ("", "mark") => {
            "Add `mark` (a Vega-Lite mark name) or `charton.mark` (a Charton mark).".to_owned()
        }
        ("mark", "type") => "The object form needs a type: {\"type\": \"line\"}.".to_owned(),
        ("data", _) => "`data` needs either `url` or `values`.".to_owned(),
        _ => format!("`{name}` is required here."),
    }
}

// ---------------------------------------------------------------------------
// Stage 2: semantics that need only the document
// ---------------------------------------------------------------------------

/// serde_path_to_error's path becomes a JSON Pointer.
///
/// Paths look like `encoding.x.field` for struct fields and `transform[0]` for
/// sequence elements, so both separators have to be understood. Segments are
/// escaped per RFC 6901, which is what makes the pointer safe to hand to an
/// editor or an agent.
pub(super) fn pointer(path: &str) -> String {
    if path.is_empty() || path == "." {
        return String::new();
    }

    let mut out = String::with_capacity(path.len() + 1);
    let mut segment = String::new();
    let push = |out: &mut String, segment: &str| {
        out.push('/');
        for ch in segment.chars() {
            match ch {
                '~' => out.push_str("~0"),
                '/' => out.push_str("~1"),
                _ => out.push(ch),
            }
        }
    };

    for ch in path.chars() {
        match ch {
            '.' => {
                if !segment.is_empty() {
                    push(&mut out, &segment);
                    segment.clear();
                }
            }
            '[' | ']' => {
                if !segment.is_empty() {
                    push(&mut out, &segment);
                    segment.clear();
                }
            }
            _ => segment.push(ch),
        }
    }
    if !segment.is_empty() {
        push(&mut out, &segment);
    }
    out
}

/// Splits `at <inner path>: <message>` and joins the inner path onto the outer
/// one, so a diagnostic points at the field rather than at the whole entry.
/// Returns the resolved dotted path and the message without its prefix.
pub(super) fn hoist_inner_path(outer: &str, message: &str) -> (String, String) {
    let Some(rest) = message.strip_prefix("at ") else {
        return (outer.to_owned(), message.to_owned());
    };
    let Some((inner, text)) = rest.split_once(": ") else {
        return (outer.to_owned(), message.to_owned());
    };
    if outer == "." || outer.is_empty() {
        return (inner.to_owned(), text.to_owned());
    }
    (format!("{outer}.{inner}"), text.to_owned())
}

pub(super) fn parent_path(path: &str) -> String {
    match path.rfind('.') {
        Some(index) => path[..index].to_owned(),
        None => String::new(),
    }
}

pub(super) fn last_segment(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or("")
}

/// serde_json appends ` at line L column C`, which the pointer makes redundant.
pub(super) fn strip_location(message: &str) -> String {
    match message.rfind(" at line ") {
        Some(index) => message[..index].to_owned(),
        None => message.to_owned(),
    }
}

/// Extract `` `name` `` after a known serde prefix.
pub(super) fn quoted_after(message: &str, prefix: &str) -> Option<String> {
    let rest = message.strip_prefix(prefix)?;
    let rest = rest.strip_prefix('`')?;
    let end = rest.find('`')?;
    Some(rest[..end].to_owned())
}

/// Extract the candidates serde listed after `expected one of`.
pub(super) fn expected_values(message: &str) -> Vec<String> {
    let Some(index) = message.find("expected one of ") else {
        return Vec::new();
    };
    message[index + "expected one of ".len()..]
        .split(',')
        .map(|part| part.trim().trim_matches('`').trim())
        .filter(|part| !part.is_empty() && !part.contains(' '))
        .map(ToOwned::to_owned)
        .collect()
}

/// Levenshtein distance, used only for "did you mean" hints.
pub(super) fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0; b.len() + 1];

    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != cb);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointers_escape_the_two_special_characters() {
        assert_eq!(pointer("."), "");
        assert_eq!(pointer("encoding.x.field"), "/encoding/x/field");
        assert_eq!(pointer("transform[0]"), "/transform/0");
        assert_eq!(pointer("data.values[2].amount"), "/data/values/2/amount");
        assert_eq!(pointer("values.0.a~b"), "/values/0/a~0b");
        assert_eq!(pointer("a/b"), "/a~1b");
    }

    #[test]
    fn edit_distance_is_sane() {
        assert_eq!(edit_distance("linee", "line"), 1);
        assert_eq!(edit_distance("line", "line"), 0);
        assert_eq!(edit_distance("", "abc"), 3);
    }

    #[test]
    fn expected_values_are_parsed() {
        let message = "unknown field `aa`, expected one of `x`, `y`, `y2`";
        assert_eq!(expected_values(message), ["x", "y", "y2"]);
    }
}
