//! Verbatim representation of content.

use ecow::EcoString;

use crate::engine::Engine;
use crate::foundations::{func, Content, Str};
use crate::World;

/// Returns the verbatim source code text that was used to create the content
/// passed to this function.
///
/// If the source text cannot be extracted (e.g., when the content doesn't have
/// valid span information or the source is not available), this function will
/// panic with an error message.
///
/// # Example
///
/// ```typ
/// #let markup = [Some _italic_ and *bold* text]
/// #verbatim(markup)
/// ```
#[func]
pub fn verbatim(
    engine: &mut Engine,
    /// The content whose verbatim representation to produce.
    content: Content,
) -> Str {
    if let Some(source_text) = extract_content_source_text(engine, &content) {
        source_text.into()
    } else {
        panic!("cannot extract verbatim source text for this content")
    }
}

/// Extract the original source text for a content value.
///
/// Returns `None` if the source text cannot be extracted (e.g., when the
/// content doesn't have valid span information or the source is not available).
fn extract_content_source_text(
    engine: &Engine,
    content: &crate::foundations::Content,
) -> Option<EcoString> {
    let span = content.span();
    let file_id = span.id()?;

    // Get the source file
    let source = engine.world.source(file_id).ok()?;

    // Get the byte range for this span
    let range = source.range(span)?;

    // Extract the text from the source
    let text = source.get(range.clone())?;

    // If the text doesn't start with '[', it likely means this content was
    // created from a ContentBlock but the span only covers the inner markup,
    // not the brackets. Try to find the surrounding ContentBlock by looking for
    // brackets.
    if !text.starts_with('[') {
        if let Some(expanded_text) = find_surrounding_content_block(&source, range) {
            return Some(expanded_text);
        }
    }

    Some(text.into())
}

/// Try to find the ContentBlock brackets surrounding the given range.
fn find_surrounding_content_block(
    source: &typst_syntax::Source,
    range: std::ops::Range<usize>,
) -> Option<EcoString> {
    let source_text = source.text();
    let bytes = source_text.as_bytes();

    // Search backward from the start of the range to find '['
    let mut start = range.start;
    let mut bracket_depth = 0;

    // First, scan backward to find the opening bracket
    while start > 0 {
        start -= 1;
        if start < bytes.len() {
            match bytes[start] {
                b'[' if bracket_depth == 0 => {
                    // Found the opening bracket, now find the matching closing bracket
                    let mut end = range.end;
                    bracket_depth = 1;
                    let mut pos = start + 1;

                    while pos < bytes.len() && bracket_depth > 0 {
                        match bytes[pos] {
                            b'[' => bracket_depth += 1,
                            b']' => bracket_depth -= 1,
                            _ => {}
                        }
                        pos += 1;
                        if bracket_depth == 0 {
                            end = pos;
                            break;
                        }
                    }

                    if bracket_depth == 0 {
                        // Found matching brackets, extract the text
                        if let Some(bracketed_text) = source_text.get(start..end) {
                            return Some(bracketed_text.into());
                        }
                    }
                    return None;
                }
                b']' => bracket_depth += 1,
                b'[' => bracket_depth -= 1,
                // Stop searching if we hit certain delimiters that suggest
                // we've gone too far
                b'\n' | b';' | b'{' | b'}' if bracket_depth == 0 => return None,
                _ => {}
            }
        }
    }

    None
}
