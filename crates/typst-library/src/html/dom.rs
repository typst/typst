use std::fmt::{self, Debug, Display, Formatter};

use ecow::{EcoString, EcoVec};
use typst_syntax::Span;
use typst_utils::{PicoStr, ResolvedPicoStr};

use crate::diag::{bail, HintedStrResult, StrResult};
use crate::foundations::{cast, Dict, Repr, Str};
use crate::introspection::{Introspector, Tag};
use crate::layout::Frame;
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
    /// A frame that will be displayed as an embedded SVG.
    Frame(Frame),
}

impl HtmlNode {
    /// Create a plain text node.
    pub fn text(text: impl Into<EcoString>, span: Span) -> Self {
        Self::Text(text.into(), span)
    }

    /// Whether the node should be pretty-printed.
    pub fn is_pretty(&self) -> bool {
        match self {
            Self::Element(element) => element.is_pretty(),
            Self::Tag(_) | Self::Text(..) | Self::Frame(_) => false,
        }
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

    /// Whether the element should be pretty-printed.
    pub fn is_pretty(&self) -> bool {
        tag::is_block_by_default(self.tag)
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
            if !bytes[i].is_ascii_alphanumeric() {
                panic!("constant tag name must be ASCII alphanumeric");
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

/// An attribute of an HTML.
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
    /// Should only be used in const contexts because it can panic.
    #[track_caller]
    pub const fn constant(string: &'static str) -> Self {
        if string.is_empty() {
            panic!("attribute name must not be empty");
        }

        let bytes = string.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !bytes[i].is_ascii_alphanumeric() {
                panic!("constant attribute name must be ASCII alphanumeric");
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
pub mod tag {
    use super::HtmlTag;

    macro_rules! tags {
        ($($tag:ident)*) => {
            $(#[allow(non_upper_case_globals)]
            pub const $tag: HtmlTag = HtmlTag::constant(
                stringify!($tag)
            );)*
        }
    }

    tags! {
        a
        abbr
        address
        area
        article
        aside
        audio
        b
        base
        bdi
        bdo
        blockquote
        body
        br
        button
        canvas
        caption
        cite
        code
        col
        colgroup
        data
        datalist
        dd
        del
        details
        dfn
        dialog
        div
        dl
        dt
        em
        embed
        fieldset
        figcaption
        figure
        footer
        form
        h1
        h2
        h3
        h4
        h5
        h6
        head
        header
        hgroup
        hr
        html
        i
        iframe
        img
        input
        ins
        kbd
        label
        legend
        li
        link
        main
        map
        mark
        menu
        meta
        meter
        nav
        noscript
        object
        ol
        optgroup
        option
        output
        p
        param
        picture
        pre
        progress
        q
        rp
        rt
        ruby
        s
        samp
        script
        search
        section
        select
        slot
        small
        source
        span
        strong
        style
        sub
        summary
        sup
        table
        tbody
        td
        template
        textarea
        tfoot
        th
        thead
        time
        title
        tr
        track
        u
        ul
        var
        video
        wbr
    }

    /// Can the content of a node with the tag be surrounded by whitespace without impacting the document?
    ///
    /// This is an underapproximation.
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

    /// Whether this is a void tag whose associated element may not have a
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
                | self::param
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
}

/// Predefined constants for HTML attributes.
///
/// Note: These are very incomplete.
pub mod attr {
    use super::HtmlAttr;

    macro_rules! attrs {
        ($($attr:ident)*) => {
            $(#[allow(non_upper_case_globals)]
            pub const $attr: HtmlAttr = HtmlAttr::constant(
                stringify!($attr)
            );)*
        }
    }

    attrs! {
        charset
        content
        href
        name
        value
    }
}
