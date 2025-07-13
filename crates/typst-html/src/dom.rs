use std::fmt::{self, Debug, Display, Formatter};

use ecow::{EcoString, EcoVec};
use typst_library::diag::{bail, HintedStrResult, StrResult};
use typst_library::foundations::{cast, Dict, Repr, Str};
use typst_library::introspection::{Introspector, Tag};
use typst_library::layout::{Abs, Frame};
use typst_library::model::DocumentInfo;
use typst_syntax::Span;
use typst_utils::{PicoStr, ResolvedPicoStr};

use crate::charsets;

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
    /// Creates an empty attribute list.
    pub fn new() -> Self {
        Self::default()
    }

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
