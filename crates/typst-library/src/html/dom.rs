use std::fmt::{self, Debug, Display, Formatter};

use ecow::{EcoString, EcoVec};
use typst_syntax::Span;
use typst_utils::{PicoStr, ResolvedPicoStr};

use crate::diag::{bail, HintedStrResult, StrResult};
use crate::foundations::{cast, Dict, Repr, Str};
use crate::introspection::{Introspector, Tag};
use crate::layout::{Abs, Frame};
use crate::model::DocumentInfo;

/// An HTML document.
#[derive(Debug, Clone)]
pub struct HtmlDocument {
    /// The document's root HTML element.
    pub root: HtmlElement,
    /// Details about the document.
    pub info: DocumentInfo,
    /// Provides the ability to execute queries on the document.
    pub introspector: Introspector,
}

/// A child of an HTML element.
#[derive(Debug, Clone, Hash)]
pub enum HtmlNode {
    /// An introspectable element that produced something within this node.
    Tag(Tag),
    /// Plain text.
    Text(EcoString, Span),
    /// Another element.
    Element(HtmlElement),
    /// Layouted content that will be embedded into HTML as an SVG.
    Frame(HtmlFrame),
}

impl HtmlNode {
    /// Create a plain text node.
    pub fn text(text: impl Into<EcoString>, span: Span) -> Self {
        Self::Text(text.into(), span)
    }
}

impl From<HtmlElement> for HtmlNode {
    fn from(element: HtmlElement) -> Self {
        Self::Element(element)
    }
}

/// An HTML element.
#[derive(Debug, Clone, Hash)]
pub struct HtmlElement {
    /// The HTML tag.
    pub tag: HtmlTag,
    /// The element's attributes.
    pub attrs: HtmlAttrs,
    /// The element's children.
    pub children: Vec<HtmlNode>,
    /// The span from which the element originated, if any.
    pub span: Span,
}

impl HtmlElement {
    /// Create a new, blank element without attributes or children.
    pub fn new(tag: HtmlTag) -> Self {
        Self {
            tag,
            attrs: HtmlAttrs::default(),
            children: vec![],
            span: Span::detached(),
        }
    }

    /// Attach children to the element.
    ///
    /// Note: This overwrites potential previous children.
    pub fn with_children(mut self, children: Vec<HtmlNode>) -> Self {
        self.children = children;
        self
    }

    /// Add an atribute to the element.
    pub fn with_attr(mut self, key: HtmlAttr, value: impl Into<EcoString>) -> Self {
        self.attrs.push(key, value);
        self
    }

    /// Attach a span to the element.
    pub fn spanned(mut self, span: Span) -> Self {
        self.span = span;
        self
    }
}

/// The tag of an HTML element.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct HtmlTag(PicoStr);

impl HtmlTag {
    /// Intern an HTML tag string at runtime.
    pub fn intern(string: &str) -> StrResult<Self> {
        if string.is_empty() {
            bail!("tag name must not be empty");
        }

        if let Some(c) = string.chars().find(|&c| !charsets::is_valid_in_tag_name(c)) {
            bail!("the character {} is not valid in a tag name", c.repr());
        }

        Ok(Self(PicoStr::intern(string)))
    }

    /// Creates a compile-time constant `HtmlTag`.
    ///
    /// Should only be used in const contexts because it can panic.
    #[track_caller]
    pub const fn constant(string: &'static str) -> Self {
        if string.is_empty() {
            panic!("tag name must not be empty");
        }

        let bytes = string.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !bytes[i].is_ascii() || !charsets::is_valid_in_tag_name(bytes[i] as char) {
                panic!("not all characters are valid in a tag name");
            }
            i += 1;
        }

        Self(PicoStr::constant(string))
    }

    /// Resolves the tag to a string.
    pub fn resolve(self) -> ResolvedPicoStr {
        self.0.resolve()
    }

    /// Turns the tag into its inner interned string.
    pub const fn into_inner(self) -> PicoStr {
        self.0
    }
}

impl Debug for HtmlTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for HtmlTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.resolve())
    }
}

cast! {
    HtmlTag,
    self => self.0.resolve().as_str().into_value(),
    v: Str => Self::intern(&v)?,
}

/// Attributes of an HTML element.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct HtmlAttrs(pub EcoVec<(HtmlAttr, EcoString)>);

impl HtmlAttrs {
    /// Add an attribute.
    pub fn push(&mut self, attr: HtmlAttr, value: impl Into<EcoString>) {
        self.0.push((attr, value.into()));
    }
}

cast! {
    HtmlAttrs,
    self => self.0
        .into_iter()
        .map(|(key, value)| (key.resolve().as_str().into(), value.into_value()))
        .collect::<Dict>()
        .into_value(),
    values: Dict => Self(values
        .into_iter()
        .map(|(k, v)| {
            let attr = HtmlAttr::intern(&k)?;
            let value = v.cast::<EcoString>()?;
            Ok((attr, value))
        })
        .collect::<HintedStrResult<_>>()?),
}

/// An attribute of an HTML element.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct HtmlAttr(PicoStr);

impl HtmlAttr {
    /// Intern an HTML attribute string at runtime.
    pub fn intern(string: &str) -> StrResult<Self> {
        if string.is_empty() {
            bail!("attribute name must not be empty");
        }

        if let Some(c) =
            string.chars().find(|&c| !charsets::is_valid_in_attribute_name(c))
        {
            bail!("the character {} is not valid in an attribute name", c.repr());
        }

        Ok(Self(PicoStr::intern(string)))
    }

    /// Creates a compile-time constant `HtmlAttr`.
    ///
    /// Must only be used in const contexts (in a constant definition or
    /// explicit `const { .. }` block) because otherwise a panic for a malformed
    /// attribute or not auto-internible constant will only be caught at
    /// runtime.
    #[track_caller]
    pub const fn constant(string: &'static str) -> Self {
        if string.is_empty() {
            panic!("attribute name must not be empty");
        }

        let bytes = string.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !bytes[i].is_ascii()
                || !charsets::is_valid_in_attribute_name(bytes[i] as char)
            {
                panic!("not all characters are valid in an attribute name");
            }
            i += 1;
        }

        Self(PicoStr::constant(string))
    }

    /// Resolves the attribute to a string.
    pub fn resolve(self) -> ResolvedPicoStr {
        self.0.resolve()
    }

    /// Turns the attribute into its inner interned string.
    pub const fn into_inner(self) -> PicoStr {
        self.0
    }
}

impl Debug for HtmlAttr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for HtmlAttr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.resolve())
    }
}

cast! {
    HtmlAttr,
    self => self.0.resolve().as_str().into_value(),
    v: Str => Self::intern(&v)?,
}

/// Layouted content that will be embedded into HTML as an SVG.
#[derive(Debug, Clone, Hash)]
pub struct HtmlFrame {
    /// The frame that will be displayed as an SVG.
    pub inner: Frame,
    /// The text size where the frame was defined. This is used to size the
    /// frame with em units to make text in and outside of the frame sized
    /// consistently.
    pub text_size: Abs,
}

/// Defines syntactical properties of HTML tags, attributes, and text.
pub mod charsets {
    /// Check whether a character is in a tag name.
    pub const fn is_valid_in_tag_name(c: char) -> bool {
        c.is_ascii_alphanumeric()
    }

    /// Check whether a character is valid in an attribute name.
    pub const fn is_valid_in_attribute_name(c: char) -> bool {
        match c {
            // These are forbidden.
            '\0' | ' ' | '"' | '\'' | '>' | '/' | '=' => false,
            c if is_whatwg_control_char(c) => false,
            c if is_whatwg_non_char(c) => false,
            // _Everything_ else is allowed, including U+2029 paragraph
            // separator. Go wild.
            _ => true,
        }
    }

    /// Check whether a character can be an used in an attribute value without
    /// escaping.
    ///
    /// See <https://html.spec.whatwg.org/multipage/syntax.html#attributes-2>
    pub const fn is_valid_in_attribute_value(c: char) -> bool {
        match c {
            // Ampersands are sometimes legal (i.e. when they are not _ambiguous
            // ampersands_) but it is not worth the trouble to check for that.
            '&' => false,
            // Quotation marks are not allowed in double-quote-delimited attribute
            // values.
            '"' => false,
            // All other text characters are allowed.
            c => is_w3c_text_char(c),
        }
    }

    /// Check whether a character can be an used in normal text without
    /// escaping.
    pub const fn is_valid_in_normal_element_text(c: char) -> bool {
        match c {
            // Ampersands are sometimes legal (i.e. when they are not _ambiguous
            // ampersands_) but it is not worth the trouble to check for that.
            '&' => false,
            // Less-than signs are not allowed in text.
            '<' => false,
            // All other text characters are allowed.
            c => is_w3c_text_char(c),
        }
    }

    /// Check if something is valid text in HTML.
    pub const fn is_w3c_text_char(c: char) -> bool {
        match c {
            // Non-characters are obviously not text characters.
            c if is_whatwg_non_char(c) => false,
            // Control characters are disallowed, except for whitespace.
            c if is_whatwg_control_char(c) => c.is_ascii_whitespace(),
            // Everything else is allowed.
            _ => true,
        }
    }

    const fn is_whatwg_non_char(c: char) -> bool {
        match c {
            '\u{fdd0}'..='\u{fdef}' => true,
            // Non-characters matching xxFFFE or xxFFFF up to x10FFFF (inclusive).
            c if c as u32 & 0xfffe == 0xfffe && c as u32 <= 0x10ffff => true,
            _ => false,
        }
    }

    const fn is_whatwg_control_char(c: char) -> bool {
        match c {
            // C0 control characters.
            '\u{00}'..='\u{1f}' => true,
            // Other control characters.
            '\u{7f}'..='\u{9f}' => true,
            _ => false,
        }
    }
}

/// Predefined constants for HTML tags.
#[allow(non_upper_case_globals)]
pub mod tag {
    use super::HtmlTag;

    pub const a: HtmlTag = HtmlTag::constant("a");
    pub const abbr: HtmlTag = HtmlTag::constant("abbr");
    pub const address: HtmlTag = HtmlTag::constant("address");
    pub const area: HtmlTag = HtmlTag::constant("area");
    pub const article: HtmlTag = HtmlTag::constant("article");
    pub const aside: HtmlTag = HtmlTag::constant("aside");
    pub const audio: HtmlTag = HtmlTag::constant("audio");
    pub const b: HtmlTag = HtmlTag::constant("b");
    pub const base: HtmlTag = HtmlTag::constant("base");
    pub const bdi: HtmlTag = HtmlTag::constant("bdi");
    pub const bdo: HtmlTag = HtmlTag::constant("bdo");
    pub const blockquote: HtmlTag = HtmlTag::constant("blockquote");
    pub const body: HtmlTag = HtmlTag::constant("body");
    pub const br: HtmlTag = HtmlTag::constant("br");
    pub const button: HtmlTag = HtmlTag::constant("button");
    pub const canvas: HtmlTag = HtmlTag::constant("canvas");
    pub const caption: HtmlTag = HtmlTag::constant("caption");
    pub const cite: HtmlTag = HtmlTag::constant("cite");
    pub const code: HtmlTag = HtmlTag::constant("code");
    pub const col: HtmlTag = HtmlTag::constant("col");
    pub const colgroup: HtmlTag = HtmlTag::constant("colgroup");
    pub const data: HtmlTag = HtmlTag::constant("data");
    pub const datalist: HtmlTag = HtmlTag::constant("datalist");
    pub const dd: HtmlTag = HtmlTag::constant("dd");
    pub const del: HtmlTag = HtmlTag::constant("del");
    pub const details: HtmlTag = HtmlTag::constant("details");
    pub const dfn: HtmlTag = HtmlTag::constant("dfn");
    pub const dialog: HtmlTag = HtmlTag::constant("dialog");
    pub const div: HtmlTag = HtmlTag::constant("div");
    pub const dl: HtmlTag = HtmlTag::constant("dl");
    pub const dt: HtmlTag = HtmlTag::constant("dt");
    pub const em: HtmlTag = HtmlTag::constant("em");
    pub const embed: HtmlTag = HtmlTag::constant("embed");
    pub const fieldset: HtmlTag = HtmlTag::constant("fieldset");
    pub const figcaption: HtmlTag = HtmlTag::constant("figcaption");
    pub const figure: HtmlTag = HtmlTag::constant("figure");
    pub const footer: HtmlTag = HtmlTag::constant("footer");
    pub const form: HtmlTag = HtmlTag::constant("form");
    pub const h1: HtmlTag = HtmlTag::constant("h1");
    pub const h2: HtmlTag = HtmlTag::constant("h2");
    pub const h3: HtmlTag = HtmlTag::constant("h3");
    pub const h4: HtmlTag = HtmlTag::constant("h4");
    pub const h5: HtmlTag = HtmlTag::constant("h5");
    pub const h6: HtmlTag = HtmlTag::constant("h6");
    pub const head: HtmlTag = HtmlTag::constant("head");
    pub const header: HtmlTag = HtmlTag::constant("header");
    pub const hgroup: HtmlTag = HtmlTag::constant("hgroup");
    pub const hr: HtmlTag = HtmlTag::constant("hr");
    pub const html: HtmlTag = HtmlTag::constant("html");
    pub const i: HtmlTag = HtmlTag::constant("i");
    pub const iframe: HtmlTag = HtmlTag::constant("iframe");
    pub const img: HtmlTag = HtmlTag::constant("img");
    pub const input: HtmlTag = HtmlTag::constant("input");
    pub const ins: HtmlTag = HtmlTag::constant("ins");
    pub const kbd: HtmlTag = HtmlTag::constant("kbd");
    pub const label: HtmlTag = HtmlTag::constant("label");
    pub const legend: HtmlTag = HtmlTag::constant("legend");
    pub const li: HtmlTag = HtmlTag::constant("li");
    pub const link: HtmlTag = HtmlTag::constant("link");
    pub const main: HtmlTag = HtmlTag::constant("main");
    pub const map: HtmlTag = HtmlTag::constant("map");
    pub const mark: HtmlTag = HtmlTag::constant("mark");
    pub const menu: HtmlTag = HtmlTag::constant("menu");
    pub const meta: HtmlTag = HtmlTag::constant("meta");
    pub const meter: HtmlTag = HtmlTag::constant("meter");
    pub const nav: HtmlTag = HtmlTag::constant("nav");
    pub const noscript: HtmlTag = HtmlTag::constant("noscript");
    pub const object: HtmlTag = HtmlTag::constant("object");
    pub const ol: HtmlTag = HtmlTag::constant("ol");
    pub const optgroup: HtmlTag = HtmlTag::constant("optgroup");
    pub const option: HtmlTag = HtmlTag::constant("option");
    pub const output: HtmlTag = HtmlTag::constant("output");
    pub const p: HtmlTag = HtmlTag::constant("p");
    pub const picture: HtmlTag = HtmlTag::constant("picture");
    pub const pre: HtmlTag = HtmlTag::constant("pre");
    pub const progress: HtmlTag = HtmlTag::constant("progress");
    pub const q: HtmlTag = HtmlTag::constant("q");
    pub const rp: HtmlTag = HtmlTag::constant("rp");
    pub const rt: HtmlTag = HtmlTag::constant("rt");
    pub const ruby: HtmlTag = HtmlTag::constant("ruby");
    pub const s: HtmlTag = HtmlTag::constant("s");
    pub const samp: HtmlTag = HtmlTag::constant("samp");
    pub const script: HtmlTag = HtmlTag::constant("script");
    pub const search: HtmlTag = HtmlTag::constant("search");
    pub const section: HtmlTag = HtmlTag::constant("section");
    pub const select: HtmlTag = HtmlTag::constant("select");
    pub const slot: HtmlTag = HtmlTag::constant("slot");
    pub const small: HtmlTag = HtmlTag::constant("small");
    pub const source: HtmlTag = HtmlTag::constant("source");
    pub const span: HtmlTag = HtmlTag::constant("span");
    pub const strong: HtmlTag = HtmlTag::constant("strong");
    pub const style: HtmlTag = HtmlTag::constant("style");
    pub const sub: HtmlTag = HtmlTag::constant("sub");
    pub const summary: HtmlTag = HtmlTag::constant("summary");
    pub const sup: HtmlTag = HtmlTag::constant("sup");
    pub const table: HtmlTag = HtmlTag::constant("table");
    pub const tbody: HtmlTag = HtmlTag::constant("tbody");
    pub const td: HtmlTag = HtmlTag::constant("td");
    pub const template: HtmlTag = HtmlTag::constant("template");
    pub const textarea: HtmlTag = HtmlTag::constant("textarea");
    pub const tfoot: HtmlTag = HtmlTag::constant("tfoot");
    pub const th: HtmlTag = HtmlTag::constant("th");
    pub const thead: HtmlTag = HtmlTag::constant("thead");
    pub const time: HtmlTag = HtmlTag::constant("time");
    pub const title: HtmlTag = HtmlTag::constant("title");
    pub const tr: HtmlTag = HtmlTag::constant("tr");
    pub const track: HtmlTag = HtmlTag::constant("track");
    pub const u: HtmlTag = HtmlTag::constant("u");
    pub const ul: HtmlTag = HtmlTag::constant("ul");
    pub const var: HtmlTag = HtmlTag::constant("var");
    pub const video: HtmlTag = HtmlTag::constant("video");
    pub const wbr: HtmlTag = HtmlTag::constant("wbr");

    /// Whether this is a void tag whose associated element may not have
    /// children.
    pub fn is_void(tag: HtmlTag) -> bool {
        matches!(
            tag,
            self::area
                | self::base
                | self::br
                | self::col
                | self::embed
                | self::hr
                | self::img
                | self::input
                | self::link
                | self::meta
                | self::source
                | self::track
                | self::wbr
        )
    }

    /// Whether this is a tag containing raw text.
    pub fn is_raw(tag: HtmlTag) -> bool {
        matches!(tag, self::script | self::style)
    }

    /// Whether this is a tag containing escapable raw text.
    pub fn is_escapable_raw(tag: HtmlTag) -> bool {
        matches!(tag, self::textarea | self::title)
    }

    /// Whether an element is considered metadata.
    pub fn is_metadata(tag: HtmlTag) -> bool {
        matches!(
            tag,
            self::base
                | self::link
                | self::meta
                | self::noscript
                | self::script
                | self::style
                | self::template
                | self::title
        )
    }

    /// Whether nodes with the tag have the CSS property `display: block` by
    /// default.
    pub fn is_block_by_default(tag: HtmlTag) -> bool {
        matches!(
            tag,
            self::html
                | self::head
                | self::body
                | self::article
                | self::aside
                | self::h1
                | self::h2
                | self::h3
                | self::h4
                | self::h5
                | self::h6
                | self::hgroup
                | self::nav
                | self::section
                | self::dd
                | self::dl
                | self::dt
                | self::menu
                | self::ol
                | self::ul
                | self::address
                | self::blockquote
                | self::dialog
                | self::div
                | self::fieldset
                | self::figure
                | self::figcaption
                | self::footer
                | self::form
                | self::header
                | self::hr
                | self::legend
                | self::main
                | self::p
                | self::pre
                | self::search
        )
    }

    /// Whether the element is inline-level as opposed to being block-level.
    ///
    /// Not sure whether this distinction really makes sense. But we somehow
    /// need to decide what to put into automatic paragraphs. A `<strong>`
    /// should merged into a paragraph created by realization, but a `<div>`
    /// shouldn't.
    ///
    /// <https://www.w3.org/TR/html401/struct/global.html#block-inline>
    /// <https://developer.mozilla.org/en-US/docs/Glossary/Inline-level_content>
    /// <https://github.com/orgs/mdn/discussions/353>
    pub fn is_inline_by_default(tag: HtmlTag) -> bool {
        matches!(
            tag,
            self::abbr
                | self::a
                | self::bdi
                | self::b
                | self::br
                | self::bdo
                | self::code
                | self::cite
                | self::dfn
                | self::data
                | self::i
                | self::em
                | self::mark
                | self::kbd
                | self::rp
                | self::q
                | self::ruby
                | self::rt
                | self::samp
                | self::s
                | self::span
                | self::small
                | self::sub
                | self::strong
                | self::time
                | self::sup
                | self::var
                | self::u
        )
    }

    /// Whether nodes with the tag have the CSS property `display: table(-.*)?`
    /// by default.
    pub fn is_tabular_by_default(tag: HtmlTag) -> bool {
        matches!(
            tag,
            self::table
                | self::thead
                | self::tbody
                | self::tfoot
                | self::tr
                | self::th
                | self::td
                | self::caption
                | self::col
                | self::colgroup
        )
    }

    /// Wether an element is considered flow content.
    pub fn is_flow_content(tag: HtmlTag) -> bool {
        matches!(
            tag,
            self::a
                | self::abbr
                | self::address
                | self::area
                | self::article
                | self::aside
                | self::audio
                | self::b
                | self::bdi
                | self::bdo
                | self::blockquote
                | self::br
                | self::button
                | self::canvas
                | self::cite
                | self::code
                | self::data
                | self::datalist
                | self::del
                | self::details
                | self::dfn
                | self::dialog
                | self::div
                | self::dl
                | self::em
                | self::embed
                | self::fieldset
                | self::figure
                | self::footer
                | self::form
                | self::h1
                | self::h2
                | self::h3
                | self::h4
                | self::h5
                | self::h6
                | self::header
                | self::hgroup
                | self::hr
                | self::i
                | self::iframe
                | self::img
                | self::input
                | self::ins
                | self::kbd
                | self::label
                | self::link
                | self::main
                | self::map
                | self::mark
                | self::menu
                | self::meta
                | self::meter
                | self::nav
                | self::noscript
                | self::object
                | self::ol
                | self::output
                | self::p
                | self::picture
                | self::pre
                | self::progress
                | self::q
                | self::ruby
                | self::s
                | self::samp
                | self::script
                | self::search
                | self::section
                | self::select
                | self::slot
                | self::small
                | self::span
                | self::strong
                | self::sub
                | self::sup
                | self::table
                | self::template
                | self::textarea
                | self::time
                | self::u
                | self::ul
                | self::var
                | self::video
                | self::wbr
        )
    }

    /// Whether an element is considered sectioning content.
    pub fn is_sectioning_content(tag: HtmlTag) -> bool {
        matches!(tag, self::article | self::aside | self::nav | self::section)
    }

    /// Whether an element is considered heading content.
    pub fn is_heading_content(tag: HtmlTag) -> bool {
        matches!(
            tag,
            self::h1
                | self::h2
                | self::h3
                | self::h4
                | self::h5
                | self::h6
                | self::hgroup
        )
    }

    /// Whether an element is considered phrasing content.
    pub fn is_phrasing_content(tag: HtmlTag) -> bool {
        matches!(
            tag,
            self::a
                | self::abbr
                | self::area
                | self::audio
                | self::b
                | self::bdi
                | self::bdo
                | self::br
                | self::button
                | self::canvas
                | self::cite
                | self::code
                | self::data
                | self::datalist
                | self::del
                | self::dfn
                | self::em
                | self::embed
                | self::i
                | self::iframe
                | self::img
                | self::input
                | self::ins
                | self::kbd
                | self::label
                | self::link
                | self::map
                | self::mark
                | self::meta
                | self::meter
                | self::noscript
                | self::object
                | self::output
                | self::picture
                | self::progress
                | self::q
                | self::ruby
                | self::s
                | self::samp
                | self::script
                | self::select
                | self::slot
                | self::small
                | self::span
                | self::strong
                | self::sub
                | self::sup
                | self::template
                | self::textarea
                | self::time
                | self::u
                | self::var
                | self::video
                | self::wbr
        )
    }

    /// Whether an element is considered embedded content.
    pub fn is_embedded_content(tag: HtmlTag) -> bool {
        matches!(
            tag,
            self::audio
                | self::canvas
                | self::embed
                | self::iframe
                | self::img
                | self::object
                | self::picture
                | self::video
        )
    }

    /// Whether an element is considered interactive content.
    pub fn is_interactive_content(tag: HtmlTag) -> bool {
        matches!(
            tag,
            self::a
                | self::audio
                | self::button
                | self::details
                | self::embed
                | self::iframe
                | self::img
                | self::input
                | self::label
                | self::select
                | self::textarea
                | self::video
        )
    }

    /// Whether an element is considered palpable content.
    pub fn is_palpable_content(tag: HtmlTag) -> bool {
        matches!(
            tag,
            self::a
                | self::abbr
                | self::address
                | self::article
                | self::aside
                | self::audio
                | self::b
                | self::bdi
                | self::bdo
                | self::blockquote
                | self::button
                | self::canvas
                | self::cite
                | self::code
                | self::data
                | self::del
                | self::details
                | self::dfn
                | self::div
                | self::dl
                | self::em
                | self::embed
                | self::fieldset
                | self::figure
                | self::footer
                | self::form
                | self::h1
                | self::h2
                | self::h3
                | self::h4
                | self::h5
                | self::h6
                | self::header
                | self::hgroup
                | self::i
                | self::iframe
                | self::img
                | self::input
                | self::ins
                | self::kbd
                | self::label
                | self::main
                | self::map
                | self::mark
                | self::menu
                | self::meter
                | self::nav
                | self::object
                | self::ol
                | self::output
                | self::p
                | self::picture
                | self::pre
                | self::progress
                | self::q
                | self::ruby
                | self::s
                | self::samp
                | self::search
                | self::section
                | self::select
                | self::small
                | self::span
                | self::strong
                | self::sub
                | self::sup
                | self::table
                | self::textarea
                | self::time
                | self::u
                | self::ul
                | self::var
                | self::video
        )
    }

    /// Whether an element is considered a script-supporting element.
    pub fn is_script_supporting_content(tag: HtmlTag) -> bool {
        matches!(tag, self::script | self::template)
    }
}

#[allow(non_upper_case_globals)]
#[rustfmt::skip]
pub mod attr {
    use crate::html::HtmlAttr;
    pub const abbr: HtmlAttr = HtmlAttr::constant("abbr");
    pub const accept: HtmlAttr = HtmlAttr::constant("accept");
    pub const accept_charset: HtmlAttr = HtmlAttr::constant("accept-charset");
    pub const accesskey: HtmlAttr = HtmlAttr::constant("accesskey");
    pub const action: HtmlAttr = HtmlAttr::constant("action");
    pub const allow: HtmlAttr = HtmlAttr::constant("allow");
    pub const allowfullscreen: HtmlAttr = HtmlAttr::constant("allowfullscreen");
    pub const alpha: HtmlAttr = HtmlAttr::constant("alpha");
    pub const alt: HtmlAttr = HtmlAttr::constant("alt");
    pub const aria_activedescendant: HtmlAttr = HtmlAttr::constant("aria-activedescendant");
    pub const aria_atomic: HtmlAttr = HtmlAttr::constant("aria-atomic");
    pub const aria_autocomplete: HtmlAttr = HtmlAttr::constant("aria-autocomplete");
    pub const aria_busy: HtmlAttr = HtmlAttr::constant("aria-busy");
    pub const aria_checked: HtmlAttr = HtmlAttr::constant("aria-checked");
    pub const aria_colcount: HtmlAttr = HtmlAttr::constant("aria-colcount");
    pub const aria_colindex: HtmlAttr = HtmlAttr::constant("aria-colindex");
    pub const aria_colspan: HtmlAttr = HtmlAttr::constant("aria-colspan");
    pub const aria_controls: HtmlAttr = HtmlAttr::constant("aria-controls");
    pub const aria_current: HtmlAttr = HtmlAttr::constant("aria-current");
    pub const aria_describedby: HtmlAttr = HtmlAttr::constant("aria-describedby");
    pub const aria_details: HtmlAttr = HtmlAttr::constant("aria-details");
    pub const aria_disabled: HtmlAttr = HtmlAttr::constant("aria-disabled");
    pub const aria_errormessage: HtmlAttr = HtmlAttr::constant("aria-errormessage");
    pub const aria_expanded: HtmlAttr = HtmlAttr::constant("aria-expanded");
    pub const aria_flowto: HtmlAttr = HtmlAttr::constant("aria-flowto");
    pub const aria_haspopup: HtmlAttr = HtmlAttr::constant("aria-haspopup");
    pub const aria_hidden: HtmlAttr = HtmlAttr::constant("aria-hidden");
    pub const aria_invalid: HtmlAttr = HtmlAttr::constant("aria-invalid");
    pub const aria_keyshortcuts: HtmlAttr = HtmlAttr::constant("aria-keyshortcuts");
    pub const aria_label: HtmlAttr = HtmlAttr::constant("aria-label");
    pub const aria_labelledby: HtmlAttr = HtmlAttr::constant("aria-labelledby");
    pub const aria_level: HtmlAttr = HtmlAttr::constant("aria-level");
    pub const aria_live: HtmlAttr = HtmlAttr::constant("aria-live");
    pub const aria_modal: HtmlAttr = HtmlAttr::constant("aria-modal");
    pub const aria_multiline: HtmlAttr = HtmlAttr::constant("aria-multiline");
    pub const aria_multiselectable: HtmlAttr = HtmlAttr::constant("aria-multiselectable");
    pub const aria_orientation: HtmlAttr = HtmlAttr::constant("aria-orientation");
    pub const aria_owns: HtmlAttr = HtmlAttr::constant("aria-owns");
    pub const aria_placeholder: HtmlAttr = HtmlAttr::constant("aria-placeholder");
    pub const aria_posinset: HtmlAttr = HtmlAttr::constant("aria-posinset");
    pub const aria_pressed: HtmlAttr = HtmlAttr::constant("aria-pressed");
    pub const aria_readonly: HtmlAttr = HtmlAttr::constant("aria-readonly");
    pub const aria_relevant: HtmlAttr = HtmlAttr::constant("aria-relevant");
    pub const aria_required: HtmlAttr = HtmlAttr::constant("aria-required");
    pub const aria_roledescription: HtmlAttr = HtmlAttr::constant("aria-roledescription");
    pub const aria_rowcount: HtmlAttr = HtmlAttr::constant("aria-rowcount");
    pub const aria_rowindex: HtmlAttr = HtmlAttr::constant("aria-rowindex");
    pub const aria_rowspan: HtmlAttr = HtmlAttr::constant("aria-rowspan");
    pub const aria_selected: HtmlAttr = HtmlAttr::constant("aria-selected");
    pub const aria_setsize: HtmlAttr = HtmlAttr::constant("aria-setsize");
    pub const aria_sort: HtmlAttr = HtmlAttr::constant("aria-sort");
    pub const aria_valuemax: HtmlAttr = HtmlAttr::constant("aria-valuemax");
    pub const aria_valuemin: HtmlAttr = HtmlAttr::constant("aria-valuemin");
    pub const aria_valuenow: HtmlAttr = HtmlAttr::constant("aria-valuenow");
    pub const aria_valuetext: HtmlAttr = HtmlAttr::constant("aria-valuetext");
    pub const r#as: HtmlAttr = HtmlAttr::constant("as");
    pub const r#async: HtmlAttr = HtmlAttr::constant("async");
    pub const autocapitalize: HtmlAttr = HtmlAttr::constant("autocapitalize");
    pub const autocomplete: HtmlAttr = HtmlAttr::constant("autocomplete");
    pub const autocorrect: HtmlAttr = HtmlAttr::constant("autocorrect");
    pub const autofocus: HtmlAttr = HtmlAttr::constant("autofocus");
    pub const autoplay: HtmlAttr = HtmlAttr::constant("autoplay");
    pub const blocking: HtmlAttr = HtmlAttr::constant("blocking");
    pub const charset: HtmlAttr = HtmlAttr::constant("charset");
    pub const checked: HtmlAttr = HtmlAttr::constant("checked");
    pub const cite: HtmlAttr = HtmlAttr::constant("cite");
    pub const class: HtmlAttr = HtmlAttr::constant("class");
    pub const closedby: HtmlAttr = HtmlAttr::constant("closedby");
    pub const color: HtmlAttr = HtmlAttr::constant("color");
    pub const colorspace: HtmlAttr = HtmlAttr::constant("colorspace");
    pub const cols: HtmlAttr = HtmlAttr::constant("cols");
    pub const colspan: HtmlAttr = HtmlAttr::constant("colspan");
    pub const command: HtmlAttr = HtmlAttr::constant("command");
    pub const commandfor: HtmlAttr = HtmlAttr::constant("commandfor");
    pub const content: HtmlAttr = HtmlAttr::constant("content");
    pub const contenteditable: HtmlAttr = HtmlAttr::constant("contenteditable");
    pub const controls: HtmlAttr = HtmlAttr::constant("controls");
    pub const coords: HtmlAttr = HtmlAttr::constant("coords");
    pub const crossorigin: HtmlAttr = HtmlAttr::constant("crossorigin");
    pub const data: HtmlAttr = HtmlAttr::constant("data");
    pub const datetime: HtmlAttr = HtmlAttr::constant("datetime");
    pub const decoding: HtmlAttr = HtmlAttr::constant("decoding");
    pub const default: HtmlAttr = HtmlAttr::constant("default");
    pub const defer: HtmlAttr = HtmlAttr::constant("defer");
    pub const dir: HtmlAttr = HtmlAttr::constant("dir");
    pub const dirname: HtmlAttr = HtmlAttr::constant("dirname");
    pub const disabled: HtmlAttr = HtmlAttr::constant("disabled");
    pub const download: HtmlAttr = HtmlAttr::constant("download");
    pub const draggable: HtmlAttr = HtmlAttr::constant("draggable");
    pub const enctype: HtmlAttr = HtmlAttr::constant("enctype");
    pub const enterkeyhint: HtmlAttr = HtmlAttr::constant("enterkeyhint");
    pub const fetchpriority: HtmlAttr = HtmlAttr::constant("fetchpriority");
    pub const r#for: HtmlAttr = HtmlAttr::constant("for");
    pub const form: HtmlAttr = HtmlAttr::constant("form");
    pub const formaction: HtmlAttr = HtmlAttr::constant("formaction");
    pub const formenctype: HtmlAttr = HtmlAttr::constant("formenctype");
    pub const formmethod: HtmlAttr = HtmlAttr::constant("formmethod");
    pub const formnovalidate: HtmlAttr = HtmlAttr::constant("formnovalidate");
    pub const formtarget: HtmlAttr = HtmlAttr::constant("formtarget");
    pub const headers: HtmlAttr = HtmlAttr::constant("headers");
    pub const height: HtmlAttr = HtmlAttr::constant("height");
    pub const hidden: HtmlAttr = HtmlAttr::constant("hidden");
    pub const high: HtmlAttr = HtmlAttr::constant("high");
    pub const href: HtmlAttr = HtmlAttr::constant("href");
    pub const hreflang: HtmlAttr = HtmlAttr::constant("hreflang");
    pub const http_equiv: HtmlAttr = HtmlAttr::constant("http-equiv");
    pub const id: HtmlAttr = HtmlAttr::constant("id");
    pub const imagesizes: HtmlAttr = HtmlAttr::constant("imagesizes");
    pub const imagesrcset: HtmlAttr = HtmlAttr::constant("imagesrcset");
    pub const inert: HtmlAttr = HtmlAttr::constant("inert");
    pub const inputmode: HtmlAttr = HtmlAttr::constant("inputmode");
    pub const integrity: HtmlAttr = HtmlAttr::constant("integrity");
    pub const is: HtmlAttr = HtmlAttr::constant("is");
    pub const ismap: HtmlAttr = HtmlAttr::constant("ismap");
    pub const itemid: HtmlAttr = HtmlAttr::constant("itemid");
    pub const itemprop: HtmlAttr = HtmlAttr::constant("itemprop");
    pub const itemref: HtmlAttr = HtmlAttr::constant("itemref");
    pub const itemscope: HtmlAttr = HtmlAttr::constant("itemscope");
    pub const itemtype: HtmlAttr = HtmlAttr::constant("itemtype");
    pub const kind: HtmlAttr = HtmlAttr::constant("kind");
    pub const label: HtmlAttr = HtmlAttr::constant("label");
    pub const lang: HtmlAttr = HtmlAttr::constant("lang");
    pub const list: HtmlAttr = HtmlAttr::constant("list");
    pub const loading: HtmlAttr = HtmlAttr::constant("loading");
    pub const r#loop: HtmlAttr = HtmlAttr::constant("loop");
    pub const low: HtmlAttr = HtmlAttr::constant("low");
    pub const max: HtmlAttr = HtmlAttr::constant("max");
    pub const maxlength: HtmlAttr = HtmlAttr::constant("maxlength");
    pub const media: HtmlAttr = HtmlAttr::constant("media");
    pub const method: HtmlAttr = HtmlAttr::constant("method");
    pub const min: HtmlAttr = HtmlAttr::constant("min");
    pub const minlength: HtmlAttr = HtmlAttr::constant("minlength");
    pub const multiple: HtmlAttr = HtmlAttr::constant("multiple");
    pub const muted: HtmlAttr = HtmlAttr::constant("muted");
    pub const name: HtmlAttr = HtmlAttr::constant("name");
    pub const nomodule: HtmlAttr = HtmlAttr::constant("nomodule");
    pub const nonce: HtmlAttr = HtmlAttr::constant("nonce");
    pub const novalidate: HtmlAttr = HtmlAttr::constant("novalidate");
    pub const open: HtmlAttr = HtmlAttr::constant("open");
    pub const optimum: HtmlAttr = HtmlAttr::constant("optimum");
    pub const pattern: HtmlAttr = HtmlAttr::constant("pattern");
    pub const ping: HtmlAttr = HtmlAttr::constant("ping");
    pub const placeholder: HtmlAttr = HtmlAttr::constant("placeholder");
    pub const playsinline: HtmlAttr = HtmlAttr::constant("playsinline");
    pub const popover: HtmlAttr = HtmlAttr::constant("popover");
    pub const popovertarget: HtmlAttr = HtmlAttr::constant("popovertarget");
    pub const popovertargetaction: HtmlAttr = HtmlAttr::constant("popovertargetaction");
    pub const poster: HtmlAttr = HtmlAttr::constant("poster");
    pub const preload: HtmlAttr = HtmlAttr::constant("preload");
    pub const readonly: HtmlAttr = HtmlAttr::constant("readonly");
    pub const referrerpolicy: HtmlAttr = HtmlAttr::constant("referrerpolicy");
    pub const rel: HtmlAttr = HtmlAttr::constant("rel");
    pub const required: HtmlAttr = HtmlAttr::constant("required");
    pub const reversed: HtmlAttr = HtmlAttr::constant("reversed");
    pub const role: HtmlAttr = HtmlAttr::constant("role");
    pub const rows: HtmlAttr = HtmlAttr::constant("rows");
    pub const rowspan: HtmlAttr = HtmlAttr::constant("rowspan");
    pub const sandbox: HtmlAttr = HtmlAttr::constant("sandbox");
    pub const scope: HtmlAttr = HtmlAttr::constant("scope");
    pub const selected: HtmlAttr = HtmlAttr::constant("selected");
    pub const shadowrootclonable: HtmlAttr = HtmlAttr::constant("shadowrootclonable");
    pub const shadowrootcustomelementregistry: HtmlAttr = HtmlAttr::constant("shadowrootcustomelementregistry");
    pub const shadowrootdelegatesfocus: HtmlAttr = HtmlAttr::constant("shadowrootdelegatesfocus");
    pub const shadowrootmode: HtmlAttr = HtmlAttr::constant("shadowrootmode");
    pub const shadowrootserializable: HtmlAttr = HtmlAttr::constant("shadowrootserializable");
    pub const shape: HtmlAttr = HtmlAttr::constant("shape");
    pub const size: HtmlAttr = HtmlAttr::constant("size");
    pub const sizes: HtmlAttr = HtmlAttr::constant("sizes");
    pub const slot: HtmlAttr = HtmlAttr::constant("slot");
    pub const span: HtmlAttr = HtmlAttr::constant("span");
    pub const spellcheck: HtmlAttr = HtmlAttr::constant("spellcheck");
    pub const src: HtmlAttr = HtmlAttr::constant("src");
    pub const srcdoc: HtmlAttr = HtmlAttr::constant("srcdoc");
    pub const srclang: HtmlAttr = HtmlAttr::constant("srclang");
    pub const srcset: HtmlAttr = HtmlAttr::constant("srcset");
    pub const start: HtmlAttr = HtmlAttr::constant("start");
    pub const step: HtmlAttr = HtmlAttr::constant("step");
    pub const style: HtmlAttr = HtmlAttr::constant("style");
    pub const tabindex: HtmlAttr = HtmlAttr::constant("tabindex");
    pub const target: HtmlAttr = HtmlAttr::constant("target");
    pub const title: HtmlAttr = HtmlAttr::constant("title");
    pub const translate: HtmlAttr = HtmlAttr::constant("translate");
    pub const r#type: HtmlAttr = HtmlAttr::constant("type");
    pub const usemap: HtmlAttr = HtmlAttr::constant("usemap");
    pub const value: HtmlAttr = HtmlAttr::constant("value");
    pub const width: HtmlAttr = HtmlAttr::constant("width");
    pub const wrap: HtmlAttr = HtmlAttr::constant("wrap");
    pub const writingsuggestions: HtmlAttr = HtmlAttr::constant("writingsuggestions");
}
