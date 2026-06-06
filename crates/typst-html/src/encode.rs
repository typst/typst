use std::fmt::Write;

use comemo::{Track, Tracked};
use ecow::{EcoString, EcoVec, eco_format};
use typst_assets::html as html_data;
use typst_library::diag::{At, SourceResult, StrResult, bail};
use typst_library::foundations::Repr;
use typst_library::model::LateLinkResolver;
use typst_syntax::Span;

use crate::{
    HtmlDocument, HtmlElement, HtmlFrame, HtmlNode, HtmlTag, attr, charsets, property,
    tag,
};

/// Settings for HTML export.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct HtmlOptions {
    /// Whether to format the HTML in a human-readable way.
    pub pretty: bool,
    /// Whether to emit an XML declaration before the doctype.
    xml_declaration: bool,
    /// Whether text must satisfy XML character restrictions.
    xml_characters: bool,
    /// Whether empty elements are serialized with XML syntax.
    xml_empty_elements: bool,
    /// Whether attribute values must satisfy XML restrictions.
    xml_attribute_values: bool,
    /// Whether known HTML names are lowercased for XML compatibility.
    normalize_html_names: bool,
    /// Whether the root `<html>` element must carry the XHTML namespace.
    xhtml_namespace: bool,
    /// Whether `lang` and `xml:lang` should be mirrored.
    mirror_language_attributes: bool,
    /// Whether empty attributes can be minimized.
    minimize_empty_attributes: bool,
    /// Whether empty presence attributes are repeated as their value.
    repeat_empty_presence_attributes: bool,
    /// Whether raw text elements are serialized as CDATA.
    cdata_raw_text: bool,
    /// Whether to apply the HTML parser's leading newline rule.
    html_parser_newline_rule: bool,
    /// Whether bare table rows should be wrapped in explicit table bodies.
    explicit_table_bodies: bool,
    /// Whether `<noscript>` is rejected.
    reject_noscript: bool,
}

impl HtmlOptions {
    /// Enable pretty-printing.
    pub fn pretty(mut self) -> Self {
        self.pretty = true;
        self
    }

    /// Enable XHTML serialization.
    pub fn xhtml(mut self) -> Self {
        self.xml_declaration = true;
        self.xml_characters = true;
        self.xml_empty_elements = true;
        self.xml_attribute_values = true;
        self.normalize_html_names = true;
        self.xhtml_namespace = true;
        self.mirror_language_attributes = true;
        self.minimize_empty_attributes = false;
        self.repeat_empty_presence_attributes = true;
        self.cdata_raw_text = true;
        self.html_parser_newline_rule = false;
        self.explicit_table_bodies = true;
        self.reject_noscript = true;
        self
    }
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self {
            pretty: false,
            xml_declaration: false,
            xml_characters: false,
            xml_empty_elements: false,
            xml_attribute_values: false,
            normalize_html_names: false,
            xhtml_namespace: false,
            mirror_language_attributes: false,
            minimize_empty_attributes: true,
            repeat_empty_presence_attributes: false,
            cdata_raw_text: false,
            html_parser_newline_rule: true,
            explicit_table_bodies: false,
            reject_noscript: false,
        }
    }
}

/// Encodes an HTML document into a string.
pub fn html(document: &HtmlDocument, options: &HtmlOptions) -> SourceResult<String> {
    let link_resolver = LateLinkResolver::new(None, document.introspector().as_ref());
    let w = Writer::new(link_resolver.track(), options);
    html_impl(w, document.root())
}

/// Encodes an HTML root element into a string as part of a bundle.
///
/// See `export_html` in `typst-bundle` for more details on why this takes the
/// root element instead of the document.
pub fn html_in_bundle(
    root: &HtmlElement,
    options: &HtmlOptions,
    link_resolver: Tracked<LateLinkResolver>,
) -> SourceResult<String> {
    let w = Writer::new(link_resolver, options);
    html_impl(w, root)
}

/// The shared implementation of [`html`] and [`html_in_bundle`].
fn html_impl(mut w: Writer, root: &HtmlElement) -> SourceResult<String> {
    if w.options.xml_declaration {
        w.buf.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>");
    }
    w.buf.push_str("<!DOCTYPE html>");
    write_indent(&mut w);
    write_element(&mut w, root)?;
    if w.pretty {
        w.buf.push('\n');
    }
    Ok(w.buf)
}

/// Encodes HTML.
struct Writer<'a, 'o> {
    /// The output buffer.
    buf: String,
    /// The current indentation level
    level: usize,
    /// Used to resolve links between the document and contained frames as well
    /// as cross-document links in bundle export.
    link_resolver: Tracked<'a, LateLinkResolver<'a>>,
    /// Whether pretty printing is enabled.
    pretty: bool,
    /// The HTML export settings.
    options: &'o HtmlOptions,
}

impl<'a, 'o> Writer<'a, 'o> {
    /// Creates a new writer.
    fn new(
        link_resolver: Tracked<'a, LateLinkResolver<'a>>,
        options: &'o HtmlOptions,
    ) -> Self {
        Self {
            buf: String::new(),
            level: 0,
            link_resolver,
            pretty: options.pretty,
            options,
        }
    }
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
fn write_node(w: &mut Writer, node: &HtmlNode, escape_text: bool) -> SourceResult<()> {
    match node {
        HtmlNode::Tag(_) => {}
        HtmlNode::Text(text, span) => write_text(w, text, *span, escape_text)?,
        HtmlNode::Element(element) => write_element(w, element)?,
        HtmlNode::Frame(frame) => write_frame(w, frame),
    }
    Ok(())
}

/// Encodes plain text into the writer.
fn write_text(w: &mut Writer, text: &str, span: Span, escape: bool) -> SourceResult<()> {
    for c in text.chars() {
        if escape
            || !charsets::is_valid_in_normal_element_text(c)
            || w.options.xml_characters && !is_valid_xml_char(c)
        {
            write_escape(w, c).at(span)?;
        } else {
            w.buf.push(c);
        }
    }
    Ok(())
}

/// Encodes one element into the writer.
fn write_element(w: &mut Writer, element: &HtmlElement) -> SourceResult<()> {
    let tag = normalize_tag(element.tag, w.options);
    let tag_name = tag.resolve();
    let tag_name = tag_name.as_str();

    if w.options.reject_noscript && tag == tag::noscript {
        bail!(element.span, "`<noscript>` is not permitted in XHTML output");
    }

    w.buf.push('<');
    w.buf.push_str(tag_name);

    let mut wrote_xhtml_namespace = false;
    let mut wrote_lang = false;
    let mut wrote_xml_lang = false;
    let mut lang = None;
    let mut xml_lang = None;
    for (attr, value) in &element.attrs.0 {
        let resolved = attr.resolve();
        let lower = normalize_attr_name(resolved.as_str(), w.options);
        let name = lower.as_deref().unwrap_or(resolved.as_str());

        if w.options.xhtml_namespace && tag == tag::html && name == "xmlns" {
            wrote_xhtml_namespace = true;
        }

        if w.options.mirror_language_attributes && name == "lang" {
            wrote_lang = true;
            lang = Some(value.as_str());
        }

        if w.options.mirror_language_attributes && name == "xml:lang" {
            wrote_xml_lang = true;
            xml_lang = Some(value.as_str());
        }

        write_attr(w, name, value, element.span)?;
    }

    if w.options.xhtml_namespace && tag == tag::html && !wrote_xhtml_namespace {
        write_attr(w, "xmlns", "http://www.w3.org/1999/xhtml", element.span)?;
    }

    if w.options.mirror_language_attributes {
        if !wrote_xml_lang && let Some(lang) = lang {
            write_attr(w, "xml:lang", lang, element.span)?;
        }

        if !wrote_lang && let Some(xml_lang) = xml_lang {
            write_attr(w, "lang", xml_lang, element.span)?;
        }
    }

    let foreign_self_closing = tag::is_foreign_self_closing(tag);
    if foreign_self_closing && !w.options.xml_empty_elements {
        w.buf.push('/');
    }

    if tag::is_void(tag) || foreign_self_closing {
        if !element.children.is_empty() {
            bail!(element.span, "HTML void elements must not have children");
        }
        if w.options.xml_empty_elements {
            w.buf.push_str(" />");
        } else {
            w.buf.push('>');
        }
        return Ok(());
    }

    w.buf.push('>');

    // See HTML spec § 13.1.2.5.
    if w.options.html_parser_newline_rule
        && matches!(tag, tag::pre | tag::textarea)
        && starts_with_newline(element)
    {
        w.buf.push('\n');
    }

    if tag::is_raw(tag) {
        write_raw(w, tag, element)?;
    } else if tag::is_escapable_raw(tag) {
        write_escapable_raw(w, element)?;
    } else if w.options.explicit_table_bodies
        && tag == tag::table
        && let Some(table) = table_with_bodies(element, w.options)
    {
        write_children(w, &table)?;
    } else if !element.children.is_empty() {
        write_children(w, element)?;
    }

    w.buf.push_str("</");
    w.buf.push_str(tag_name);
    w.buf.push('>');

    Ok(())
}

/// Encodes the children of an element.
fn write_children(w: &mut Writer, element: &HtmlElement) -> SourceResult<()> {
    let pretty = w.pretty;
    let pretty_inside = allows_pretty_inside(element.tag)
        && element.children.iter().any(|node| match node {
            HtmlNode::Element(child) => wants_pretty_around(child),
            HtmlNode::Frame(_) => true,
            _ => false,
        });

    w.pretty &= pretty_inside;
    let mut indent = w.pretty;

    w.level += 1;
    for c in &element.children {
        let pretty_around = match c {
            HtmlNode::Tag(_) => continue,
            HtmlNode::Element(child) => w.pretty && wants_pretty_around(child),
            HtmlNode::Text(..) | HtmlNode::Frame(_) => false,
        };

        if core::mem::take(&mut indent) || pretty_around {
            write_indent(w);
        }
        write_node(w, c, element.pre_span)?;
        indent = pretty_around;
    }
    w.level -= 1;

    write_indent(w);
    w.pretty = pretty;

    Ok(())
}

/// Returns a table whose bare `<tr>` children are wrapped in `<tbody>`.
fn table_with_bodies(
    element: &HtmlElement,
    options: &HtmlOptions,
) -> Option<HtmlElement> {
    let mut wrapped = EcoVec::with_capacity(element.children.len() + 1);
    let mut rows = EcoVec::new();
    let mut had_row = false;

    for child in &element.children {
        if matches!(child, HtmlNode::Element(el) if normalize_tag(el.tag, options) == tag::tr)
        {
            rows.push(child.clone());
            had_row = true;
        } else {
            push_tbody(&mut wrapped, &mut rows);
            wrapped.push(child.clone());
        }
    }

    push_tbody(&mut wrapped, &mut rows);

    if had_row {
        let mut table = element.clone();
        table.children = wrapped;
        Some(table)
    } else {
        None
    }
}

/// Pushes a pending `<tbody>` if there are collected rows.
fn push_tbody(children: &mut EcoVec<HtmlNode>, rows: &mut EcoVec<HtmlNode>) {
    if !rows.is_empty() {
        children.push(
            HtmlElement::new(tag::tbody)
                .with_children(core::mem::take(rows))
                .into(),
        );
    }
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
fn write_raw(w: &mut Writer, tag: HtmlTag, element: &HtmlElement) -> SourceResult<()> {
    let text = collect_raw_text(element, w.options)?;

    if w.options.cdata_raw_text {
        write_cdata(w, tag, &text);
        return Ok(());
    }

    if let Some(closing) = find_closing_tag(&text, tag) {
        bail!(
            element.span,
            "HTML raw text element cannot contain its own closing tag";
            hint: "the sequence `{closing}` appears in the raw text";
        )
    }

    let mode = if w.pretty { RawMode::of(tag, element, &text) } else { RawMode::Keep };
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

/// Encodes the contents of an escapable raw text element.
fn write_escapable_raw(w: &mut Writer, element: &HtmlElement) -> SourceResult<()> {
    walk_raw_text(element, |piece, span| {
        write_text(w, piece, span, w.options.xml_characters)
    })
}

/// Collects the textual contents of a raw text element.
fn collect_raw_text(
    element: &HtmlElement,
    options: &HtmlOptions,
) -> SourceResult<String> {
    let mut text = String::new();
    let valid_char: fn(char) -> bool = if options.xml_characters {
        is_valid_xml_char
    } else {
        charsets::is_w3c_text_char
    };

    walk_raw_text(element, |piece, span| {
        if let Some(c) = piece.chars().find(|&c| !valid_char(c)) {
            return Err(unencodable(c, options)).at(span);
        }
        text.push_str(piece);
        Ok(())
    })?;
    Ok(text)
}

/// Iterates over the textual contents of a raw text element.
fn walk_raw_text(
    element: &HtmlElement,
    mut f: impl FnMut(&str, Span) -> SourceResult<()>,
) -> SourceResult<()> {
    for c in &element.children {
        match c {
            HtmlNode::Tag(_) => continue,
            HtmlNode::Text(text, span) => f(text, *span)?,
            HtmlNode::Element(HtmlElement { span, .. })
            | HtmlNode::Frame(HtmlFrame { span, .. }) => {
                bail!(*span, "HTML raw text element cannot have non-text children")
            }
        }
    }
    Ok(())
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
    fn of(tag: HtmlTag, element: &HtmlElement, text: &str) -> Self {
        match tag {
            tag::script
                if !element.attrs.0.iter().any(|(attr, value)| {
                    *attr == attr::r#type && value != "text/javascript"
                }) =>
            {
                // Template literals can be multi-line, so indent may change
                // the semantics of the JavaScript.
                if text.contains('`') { Self::Wrap } else { Self::Indent }
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
    if tag::mathml::is_mathml(tag) && !tag::mathml::is_token(tag) {
        return true;
    }
    let Some(display) = property::Display::default_for(tag) else { return false };
    (display == property::Display::Block && tag != tag::pre)
        || display.is_tabular()
        || display == property::Display::ListItem
        || tag == tag::head
}

/// Whether newlines should be added before and after the element if the parent
/// allows it.
///
/// In contrast to `allows_pretty_inside`, which is purely spec-driven, this is
/// more subjective and depends on preference.
fn wants_pretty_around(element: &HtmlElement) -> bool {
    match element.tag {
        tag::mathml::math => {
            element.attrs.get(attr::mathml::display).is_some_and(|v| v == "block")
        }
        t if tag::mathml::is_mathml(t) => true,
        tag::pre => true,
        t if tag::is_metadata_content(t) => true,
        t => allows_pretty_inside(t),
    }
}

/// Escape a character.
fn write_escape(w: &mut Writer, c: char) -> StrResult<()> {
    match c {
        '&' => w.buf.push_str("&amp;"),
        '<' => w.buf.push_str("&lt;"),
        '>' => w.buf.push_str("&gt;"),
        '"' => w.buf.push_str("&quot;"),
        '\'' => w.buf.push_str("&apos;"),
        c if (charsets::is_w3c_text_char(c)
            || w.options.xml_characters && is_discouraged_xml_char(c))
            && (!w.options.xml_characters || is_valid_xml_char(c))
            && c != '\r' =>
        {
            write!(w.buf, "&#x{:x};", c as u32).unwrap()
        }
        _ => return Err(unencodable(c, w.options)),
    }
    Ok(())
}

/// The error message for a character that cannot be encoded.
#[cold]
fn unencodable(c: char, options: &HtmlOptions) -> EcoString {
    let format = if options.xml_characters { "XHTML" } else { "HTML" };
    eco_format!("the character `{}` cannot be encoded in {format}", c.repr())
}

/// Writes an attribute and escapes its value as needed.
fn write_attr(w: &mut Writer, name: &str, value: &str, span: Span) -> SourceResult<()> {
    w.buf.push(' ');
    w.buf.push_str(name);

    if value.is_empty() && w.options.minimize_empty_attributes {
        return Ok(());
    }

    w.buf.push('=');
    w.buf.push('"');
    let value = if w.options.repeat_empty_presence_attributes
        && value.is_empty()
        && html_data::ATTRS.iter().any(|attr| {
            attr.name.eq_ignore_ascii_case(name)
                && matches!(attr.ty, html_data::Type::Presence)
        }) {
        name
    } else {
        value
    };
    for c in value.chars() {
        if charsets::is_valid_in_attribute_value(c)
            && (!w.options.xml_attribute_values || c != '<' && is_valid_xml_char(c))
        {
            w.buf.push(c);
        } else {
            write_escape(w, c).at(span)?;
        }
    }
    w.buf.push('"');
    Ok(())
}

/// Whether the character is permitted by XML 1.0.
///
/// See <https://www.w3.org/TR/xml/#NT-Char>.
fn is_valid_xml_char(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{A}' | '\u{D}'
            | '\u{20}'..='\u{D7FF}'
            | '\u{E000}'..='\u{FFFD}'
            | '\u{10000}'..='\u{10FFFF}'
    )
}

/// Whether the character is legal in XML 1.0 but discouraged by the spec.
fn is_discouraged_xml_char(c: char) -> bool {
    matches!(c as u32, 0x7F..=0x84 | 0x86..=0x9F | 0xFDD0..=0xFDEF)
        || matches!(c as u32, code if code > 0xFFFF && code & 0xFFFE == 0xFFFE)
}

/// Lowercase names in XHTML output so HTML vocabulary stays XML-compatible.
fn normalize_tag(tag: HtmlTag, options: &HtmlOptions) -> HtmlTag {
    if !options.normalize_html_names {
        return tag;
    }

    let resolved = tag.resolve();
    if !resolved.as_str().bytes().any(|byte| byte.is_ascii_uppercase())
        || !html_data::ELEMS
            .iter()
            .any(|info| info.name.eq_ignore_ascii_case(resolved.as_str()))
    {
        return tag;
    }

    HtmlTag::intern(&resolved.as_str().to_ascii_lowercase())
        .expect("lowercasing preserves valid HTML tag names")
}

/// Lowercase relevant names in XHTML output so HTML vocabulary stays
/// XML-compatible.
fn normalize_attr_name(name: &str, options: &HtmlOptions) -> Option<String> {
    (options.normalize_html_names
        && name.bytes().any(|byte| byte.is_ascii_uppercase())
        && (html_data::ATTRS
            .iter()
            .any(|info| info.name.eq_ignore_ascii_case(name))
            || name.eq_ignore_ascii_case("xmlns")
            || name.eq_ignore_ascii_case("xml:lang")
            || name
                .get(..5)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data-"))))
    .then(|| name.to_ascii_lowercase())
}

/// Writes a CDATA-wrapped script or style body.
fn write_cdata(w: &mut Writer, tag: HtmlTag, text: &str) {
    assert!(matches!(tag, tag::script | tag::style));

    w.buf.push_str("<![CDATA[");
    for (i, piece) in text.split("]]>").enumerate() {
        if i > 0 {
            w.buf.push_str("]]]]><![CDATA[>");
        }
        w.buf.push_str(piece);
    }
    w.buf.push_str("]]>");
}

/// Encode a laid out frame into the writer.
fn write_frame(w: &mut Writer, frame: &HtmlFrame) {
    let svg = typst_svg::svg_in_html(
        &frame.inner,
        frame.text_size,
        w.pretty,
        frame.id.as_deref(),
        &eco_format!("{}", frame.css.to_inline()),
        &frame.anchors,
        w.link_resolver,
    );
    w.buf.push_str(&svg);
}
