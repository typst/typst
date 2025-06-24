use std::fmt::Write;

use typst_library::diag::{bail, At, SourceResult, StrResult};
use typst_library::foundations::Repr;
use typst_library::html::{
    attr, charsets, tag, HtmlDocument, HtmlElement, HtmlNode, HtmlTag,
};
use typst_library::layout::Frame;
use typst_syntax::Span;

/// Encodes an HTML document into a string.
pub fn html(document: &HtmlDocument) -> SourceResult<String> {
    let mut w = Writer { pretty: true, ..Writer::default() };
    w.buf.push_str("<!DOCTYPE html>");
    write_indent(&mut w);
    write_element(&mut w, &document.root)?;
    if w.pretty {
        w.buf.push('\n');
    }
    Ok(w.buf)
}

#[derive(Default)]
struct Writer {
    /// The output buffer.
    buf: String,
    /// The current indentation level
    level: usize,
    /// Whether pretty printing is enabled.
    pretty: bool,
}

/// Writes a newline and indent, if pretty printing is enabled.
fn write_indent(w: &mut Writer) {
    if w.pretty {
        w.buf.push('\n');
        for _ in 0..w.level {
            w.buf.push_str("  ");
        }
    }
}

/// Encodes an HTML node into the writer.
fn write_node(w: &mut Writer, node: &HtmlNode) -> SourceResult<()> {
    match node {
        HtmlNode::Tag(_) => {}
        HtmlNode::Text(text, span) => write_text(w, text, *span)?,
        HtmlNode::Element(element) => write_element(w, element)?,
        HtmlNode::Frame(frame) => write_frame(w, frame),
    }
    Ok(())
}

/// Encodes plain text into the writer.
fn write_text(w: &mut Writer, text: &str, span: Span) -> SourceResult<()> {
    for c in text.chars() {
        if charsets::is_valid_in_normal_element_text(c) {
            w.buf.push(c);
        } else {
            write_escape(w, c).at(span)?;
        }
    }
    Ok(())
}

/// Encodes one element into the writer.
fn write_element(w: &mut Writer, element: &HtmlElement) -> SourceResult<()> {
    w.buf.push('<');
    w.buf.push_str(&element.tag.resolve());

    for (attr, value) in &element.attrs.0 {
        w.buf.push(' ');
        w.buf.push_str(&attr.resolve());

        // If the string is empty, we can use shorthand syntax.
        // `<elem attr="">..</div` is equivalent to `<elem attr>..</div>`
        if !value.is_empty() {
            w.buf.push('=');
            w.buf.push('"');
            for c in value.chars() {
                if charsets::is_valid_in_attribute_value(c) {
                    w.buf.push(c);
                } else {
                    write_escape(w, c).at(element.span)?;
                }
            }
            w.buf.push('"');
        }
    }

    w.buf.push('>');

    if tag::is_void(element.tag) {
        if !element.children.is_empty() {
            bail!(element.span, "HTML void elements must not have children");
        }
        return Ok(());
    }

    if tag::is_raw(element.tag) {
        write_raw(w, element)?;
    } else if !element.children.is_empty() {
        write_children(w, element)?;
    }

    w.buf.push_str("</");
    w.buf.push_str(&element.tag.resolve());
    w.buf.push('>');

    Ok(())
}

/// Encodes the children of an element.
fn write_children(w: &mut Writer, element: &HtmlElement) -> SourceResult<()> {
    // See HTML spec § 13.1.2.5.
    if matches!(element.tag, tag::pre | tag::textarea) && starts_with_newline(element) {
        w.buf.push('\n');
    }

    let pretty = w.pretty;
    let pretty_inside = allows_pretty_inside(element.tag)
        && element.children.iter().any(|node| match node {
            HtmlNode::Element(child) => wants_pretty_around(child.tag),
            _ => false,
        });

    w.pretty &= pretty_inside;
    let mut indent = w.pretty;

    w.level += 1;
    for c in &element.children {
        let pretty_around = match c {
            HtmlNode::Tag(_) => continue,
            HtmlNode::Element(child) => w.pretty && wants_pretty_around(child.tag),
            HtmlNode::Text(..) | HtmlNode::Frame(_) => false,
        };

        if core::mem::take(&mut indent) || pretty_around {
            write_indent(w);
        }
        write_node(w, c)?;
        indent = pretty_around;
    }
    w.level -= 1;

    write_indent(w);
    w.pretty = pretty;

    Ok(())
}

/// Whether the first character in the element is a newline.
fn starts_with_newline(element: &HtmlElement) -> bool {
    for child in &element.children {
        match child {
            HtmlNode::Tag(_) => {}
            HtmlNode::Text(text, _) => return text.starts_with(['\n', '\r']),
            _ => return false,
        }
    }
    false
}

/// Encodes the contents of a raw text element.
fn write_raw(w: &mut Writer, element: &HtmlElement) -> SourceResult<()> {
    let text = collect_raw_text(element)?;

    if let Some(closing) = find_closing_tag(&text, element.tag) {
        bail!(
            element.span,
            "HTML raw text element cannot contain its own closing tag";
            hint: "the sequence `{closing}` appears in the raw text",
        )
    }

    let mode = if w.pretty { RawMode::of(element, &text) } else { RawMode::Keep };
    match mode {
        RawMode::Keep => {
            w.buf.push_str(&text);
        }
        RawMode::Wrap => {
            w.buf.push('\n');
            w.buf.push_str(&text);
            write_indent(w);
        }
        RawMode::Indent => {
            w.level += 1;
            for line in text.lines() {
                write_indent(w);
                w.buf.push_str(line);
            }
            w.level -= 1;
            write_indent(w);
        }
    }

    Ok(())
}

/// Collects the textual contents of a raw text element.
fn collect_raw_text(element: &HtmlElement) -> SourceResult<String> {
    let mut output = String::new();
    for c in &element.children {
        match c {
            HtmlNode::Tag(_) => continue,
            HtmlNode::Text(text, _) => output.push_str(text),
            HtmlNode::Element(_) | HtmlNode::Frame(_) => {
                let span = match c {
                    HtmlNode::Element(child) => child.span,
                    _ => element.span,
                };
                bail!(span, "HTML raw text element cannot have non-text children")
            }
        };
    }
    Ok(output)
}

/// Finds a closing sequence for the given tag in the text, if it exists.
///
/// See HTML spec § 13.1.2.6.
fn find_closing_tag(text: &str, tag: HtmlTag) -> Option<&str> {
    let s = tag.resolve();
    let len = s.len();
    text.match_indices("</").find_map(|(i, _)| {
        let rest = &text[i + 2..];
        let disallowed = rest.len() >= len
            && rest[..len].eq_ignore_ascii_case(&s)
            && rest[len..].starts_with(['\t', '\n', '\u{c}', '\r', ' ', '>', '/']);
        disallowed.then(|| &text[i..i + 2 + len])
    })
}

/// How to format the contents of a raw text element.
enum RawMode {
    /// Just don't touch it.
    Keep,
    /// Newline after the opening and newline + indent before the closing tag.
    Wrap,
    /// Newlines after opening and before closing tag and each line indented.
    Indent,
}

impl RawMode {
    fn of(element: &HtmlElement, text: &str) -> Self {
        match element.tag {
            tag::script
                if !element.attrs.0.iter().any(|(attr, value)| {
                    *attr == attr::r#type && value != "text/javascript"
                }) =>
            {
                // Template literals can be multi-line, so indent may change
                // the semantics of the JavaScript.
                if text.contains('`') {
                    Self::Wrap
                } else {
                    Self::Indent
                }
            }
            tag::style => Self::Indent,
            _ => Self::Keep,
        }
    }
}

/// Whether we are allowed to add an extra newline at the start and end of the
/// element's contents.
///
/// Technically, users can change CSS `display` properties such that the
/// insertion of whitespace may actually impact the visual output. For example,
/// <https://www.w3.org/TR/css-text-3/#example-af2745cd> shows how adding CSS
/// rules to `<p>` can make it sensitive to whitespace. For this reason, we
/// should also respect the `style` tag in the future.
fn allows_pretty_inside(tag: HtmlTag) -> bool {
    (tag::is_block_by_default(tag) && tag != tag::pre)
        || tag::is_tabular_by_default(tag)
        || tag == tag::li
}

/// Whether newlines should be added before and after the element if the parent
/// allows it.
///
/// In contrast to `allows_pretty_inside`, which is purely spec-driven, this is
/// more subjective and depends on preference.
fn wants_pretty_around(tag: HtmlTag) -> bool {
    allows_pretty_inside(tag) || tag::is_metadata(tag) || tag == tag::pre
}

/// Escape a character.
fn write_escape(w: &mut Writer, c: char) -> StrResult<()> {
    // See <https://html.spec.whatwg.org/multipage/syntax.html#syntax-charref>
    match c {
        '&' => w.buf.push_str("&amp;"),
        '<' => w.buf.push_str("&lt;"),
        '>' => w.buf.push_str("&gt;"),
        '"' => w.buf.push_str("&quot;"),
        '\'' => w.buf.push_str("&apos;"),
        c if charsets::is_w3c_text_char(c) && c != '\r' => {
            write!(w.buf, "&#x{:x};", c as u32).unwrap()
        }
        _ => bail!("the character `{}` cannot be encoded in HTML", c.repr()),
    }
    Ok(())
}

/// Encode a laid out frame into the writer.
fn write_frame(w: &mut Writer, frame: &Frame) {
    // FIXME: This string replacement is obviously a hack.
    let svg = typst_svg::svg_frame(frame)
        .replace("<svg class", "<svg style=\"overflow: visible;\" class");
    w.buf.push_str(&svg);
}
