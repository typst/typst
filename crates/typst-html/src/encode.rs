use std::fmt::Write;

use typst_library::diag::{bail, At, SourceResult, StrResult};
use typst_library::foundations::Repr;
use typst_library::html::{charsets, tag, HtmlDocument, HtmlElement, HtmlNode};
use typst_library::layout::Frame;
use typst_syntax::Span;

/// Encodes an HTML document into a string.
pub fn html(document: &HtmlDocument) -> SourceResult<String> {
    let mut w = Writer { pretty: true, ..Writer::default() };
    w.buf.push_str("<!DOCTYPE html>");
    write_indent(&mut w);
    write_element(&mut w, &document.root)?;
    Ok(w.buf)
}

#[derive(Default)]
struct Writer {
    buf: String,
    /// current indentation level
    level: usize,
    /// pretty printing enabled?
    pretty: bool,
}

/// Write a newline and indent, if pretty printing is enabled.
fn write_indent(w: &mut Writer) {
    if w.pretty {
        w.buf.push('\n');
        for _ in 0..w.level {
            w.buf.push_str("  ");
        }
    }
}

/// Encode an HTML node into the writer.
fn write_node(w: &mut Writer, node: &HtmlNode) -> SourceResult<()> {
    match node {
        HtmlNode::Tag(_) => {}
        HtmlNode::Text(text, span) => write_text(w, text, *span)?,
        HtmlNode::Element(element) => write_element(w, element)?,
        HtmlNode::Frame(frame) => write_frame(w, frame),
    }
    Ok(())
}

/// Encode plain text into the writer.
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

/// Encode one element into the write.
fn write_element(w: &mut Writer, element: &HtmlElement) -> SourceResult<()> {
    w.buf.push('<');
    w.buf.push_str(&element.tag.resolve());

    for (attr, value) in &element.attrs.0 {
        w.buf.push(' ');
        w.buf.push_str(&attr.resolve());
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

    w.buf.push('>');

    if tag::is_void(element.tag) {
        return Ok(());
    }

    let pretty = w.pretty;
    if !element.children.is_empty() {
        w.pretty &= is_pretty(element);
        let mut indent = w.pretty;

        w.level += 1;
        for c in &element.children {
            let pretty_child = match c {
                HtmlNode::Tag(_) => continue,
                HtmlNode::Element(element) => is_pretty(element),
                HtmlNode::Text(..) | HtmlNode::Frame(_) => false,
            };

            if core::mem::take(&mut indent) || pretty_child {
                write_indent(w);
            }
            write_node(w, c)?;
            indent = pretty_child;
        }
        w.level -= 1;

        write_indent(w)
    }
    w.pretty = pretty;

    w.buf.push_str("</");
    w.buf.push_str(&element.tag.resolve());
    w.buf.push('>');

    Ok(())
}

/// Whether the element should be pretty-printed.
fn is_pretty(element: &HtmlElement) -> bool {
    tag::is_block_by_default(element.tag) || matches!(element.tag, tag::meta)
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
        _ => bail!("the character {} cannot be encoded in HTML", c.repr()),
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
