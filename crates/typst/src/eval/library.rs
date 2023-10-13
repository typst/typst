use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;

use comemo::Tracked;
use ecow::EcoString;
use std::sync::OnceLock;

use super::Module;
use crate::diag::SourceResult;
use crate::doc::Document;
use crate::geom::{Abs, Dir};
use crate::model::{Content, Element, Introspector, Label, StyleChain, Styles, Vt};
use crate::util::hash128;

/// Definition of Typst's standard library.
#[derive(Debug, Clone, Hash)]
pub struct Library {
    /// The scope containing definitions that are available everywhere.
    pub global: Module,
    /// The scope containing definitions available in math mode.
    pub math: Module,
    /// The default properties for page size, font selection and so on.
    pub styles: Styles,
    /// Defines which standard library items fulfill which syntactical roles.
    pub items: LangItems,
}

/// Definition of library items the language is aware of.
#[derive(Clone)]
pub struct LangItems {
    /// The root layout function.
    pub layout:
        fn(vt: &mut Vt, content: &Content, styles: StyleChain) -> SourceResult<Document>,
    /// Access the em size.
    pub em: fn(StyleChain) -> Abs,
    /// Access the text direction.
    pub dir: fn(StyleChain) -> Dir,
    /// Whitespace.
    pub space: fn() -> Content,
    /// A forced line break: `\`.
    pub linebreak: fn() -> Content,
    /// Plain text without markup.
    pub text: fn(text: EcoString) -> Content,
    /// The text element.
    pub text_elem: Element,
    /// Get the string if this is a text element.
    pub text_str: fn(&Content) -> Option<EcoString>,
    /// A smart quote: `'` or `"`.
    pub smart_quote: fn(double: bool) -> Content,
    /// A paragraph break.
    pub parbreak: fn() -> Content,
    /// Strong content: `*Strong*`.
    pub strong: fn(body: Content) -> Content,
    /// Emphasized content: `_Emphasized_`.
    pub emph: fn(body: Content) -> Content,
    /// Raw text with optional syntax highlighting: `` `...` ``.
    pub raw: fn(text: EcoString, tag: Option<EcoString>, block: bool) -> Content,
    /// The language names and tags supported by raw text.
    pub raw_languages: fn() -> Vec<(&'static str, Vec<&'static str>)>,
    /// A hyperlink: `https://typst.org`.
    pub link: fn(url: EcoString) -> Content,
    /// A reference: `@target`, `@target[..]`.
    pub reference: fn(target: Label, supplement: Option<Content>) -> Content,
    /// The keys contained in the bibliography and short descriptions of them.
    #[allow(clippy::type_complexity)]
    pub bibliography_keys:
        fn(introspector: Tracked<Introspector>) -> Vec<(EcoString, Option<EcoString>)>,
    /// A section heading: `= Introduction`.
    pub heading: fn(level: NonZeroUsize, body: Content) -> Content,
    /// The heading element.
    pub heading_elem: Element,
    /// An item in a bullet list: `- ...`.
    pub list_item: fn(body: Content) -> Content,
    /// An item in an enumeration (numbered list): `+ ...` or `1. ...`.
    pub enum_item: fn(number: Option<usize>, body: Content) -> Content,
    /// An item in a term list: `/ Term: Details`.
    pub term_item: fn(term: Content, description: Content) -> Content,
    /// A mathematical equation: `$x$`, `$ x^2 $`.
    pub equation: fn(body: Content, block: bool) -> Content,
    /// An alignment point in math: `&`.
    pub math_align_point: fn() -> Content,
    /// Matched delimiters in math: `[x + y]`.
    pub math_delimited: fn(open: Content, body: Content, close: Content) -> Content,
    /// A base with optional attachments in math: `a_1^2`.
    #[allow(clippy::type_complexity)]
    pub math_attach: fn(
        base: Content,
        // Positioned smartly.
        t: Option<Content>,
        b: Option<Content>,
        // Fixed positions.
        tl: Option<Content>,
        bl: Option<Content>,
        tr: Option<Content>,
        br: Option<Content>,
    ) -> Content,
    /// Grouped primes: `a'''`.
    pub math_primes: fn(count: usize) -> Content,
    /// A base with an accent: `arrow(x)`.
    pub math_accent: fn(base: Content, accent: char) -> Content,
    /// A fraction in math: `x/2`.
    pub math_frac: fn(num: Content, denom: Content) -> Content,
    /// A root in math: `√x`, `∛x` or `∜x`.
    pub math_root: fn(index: Option<Content>, radicand: Content) -> Content,
}

impl Debug for LangItems {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("LangItems { .. }")
    }
}

impl Hash for LangItems {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.layout as usize).hash(state);
        (self.em as usize).hash(state);
        (self.dir as usize).hash(state);
        self.space.hash(state);
        self.linebreak.hash(state);
        self.text.hash(state);
        self.text_elem.hash(state);
        (self.text_str as usize).hash(state);
        self.smart_quote.hash(state);
        self.parbreak.hash(state);
        self.strong.hash(state);
        self.emph.hash(state);
        self.raw.hash(state);
        self.raw_languages.hash(state);
        self.link.hash(state);
        self.reference.hash(state);
        (self.bibliography_keys as usize).hash(state);
        self.heading.hash(state);
        self.heading_elem.hash(state);
        self.list_item.hash(state);
        self.enum_item.hash(state);
        self.term_item.hash(state);
        self.equation.hash(state);
        self.math_align_point.hash(state);
        self.math_delimited.hash(state);
        self.math_attach.hash(state);
        self.math_accent.hash(state);
        self.math_frac.hash(state);
        self.math_root.hash(state);
    }
}

/// Global storage for lang items.
#[doc(hidden)]
pub static LANG_ITEMS: OnceLock<LangItems> = OnceLock::new();

/// Set the lang items.
///
/// This is a hack :(
///
/// Passing the lang items everywhere they are needed (especially text related
/// things) is very painful. By storing them globally, in theory, we break
/// incremental, but only when different sets of lang items are used in the same
/// program. For this reason, if this function is called multiple times, the
/// items must be the same (and this is enforced).
pub fn set_lang_items(items: LangItems) {
    if let Err(items) = LANG_ITEMS.set(items) {
        let first = hash128(LANG_ITEMS.get().unwrap());
        let second = hash128(&items);
        assert_eq!(first, second, "set differing lang items");
    }
}

/// Access a lang item.
macro_rules! item {
    ($name:ident) => {
        $crate::eval::LANG_ITEMS.get().unwrap().$name
    };
}
