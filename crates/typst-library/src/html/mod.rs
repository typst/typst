//! HTML output.

mod dom;

pub use self::dom::*;

use ecow::EcoString;

use crate::foundations::{category, elem, Category, Content, Module, Scope};

/// HTML output.
#[category]
pub static HTML: Category;

/// Create a module with all HTML definitions.
pub fn module() -> Module {
    let mut html = Scope::deduplicating();
    html.start_category(HTML);
    html.define_elem::<HtmlElem>();
    html.define_elem::<FrameElem>();
    Module::new("html", html)
}

/// A HTML element that can contain Typst content.
#[elem(name = "elem")]
pub struct HtmlElem {
    /// The element's tag.
    #[required]
    pub tag: HtmlTag,

    /// The element's attributes.
    #[borrowed]
    pub attrs: HtmlAttrs,

    /// The contents of the HTML element.
    #[positional]
    #[borrowed]
    pub body: Option<Content>,
}

impl HtmlElem {
    /// Add an atribute to the element.
    pub fn with_attr(mut self, attr: HtmlAttr, value: impl Into<EcoString>) -> Self {
        self.attrs.get_or_insert_with(Default::default).push(attr, value);
        self
    }
}

/// An element that forces its contents to be laid out.
///
/// Integrates content that requires layout (e.g. a plot) into HTML output
/// by turning it into an inline SVG.
#[elem]
pub struct FrameElem {
    /// The contents that shall be laid out.
    #[positional]
    #[required]
    pub body: Content,
}
