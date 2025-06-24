//! HTML output.

mod dom;
mod typed;

pub use self::dom::*;

use ecow::EcoString;

use crate::foundations::{elem, Content, Module, Scope};

/// Create a module with all HTML definitions.
pub fn module() -> Module {
    let mut html = Scope::deduplicating();
    html.start_category(crate::Category::Html);
    html.define_elem::<HtmlElem>();
    html.define_elem::<FrameElem>();
    self::typed::define(&mut html);
    Module::new("html", html)
}

/// An HTML element that can contain Typst content.
///
/// Typst's HTML export automatically generates the appropriate tags for most
/// elements. However, sometimes, it is desirable to retain more control. For
/// example, when using Typst to generate your blog, you could use this function
/// to wrap each article in an `<article>` tag.
///
/// Typst is aware of what is valid HTML. A tag and its attributes must form
/// syntactically valid HTML. Some tags, like `meta` do not accept content.
/// Hence, you must not provide a body for them. We may add more checks in the
/// future, so be sure that you are generating valid HTML when using this
/// function.
///
/// Normally, Typst will generate `html`, `head`, and `body` tags for you. If
/// you instead create them with this function, Typst will omit its own tags.
///
/// ```typ
/// #html.elem("div", attrs: (style: "background: aqua"))[
///   A div with _Typst content_ inside!
/// ]
/// ```
#[elem(name = "elem")]
pub struct HtmlElem {
    /// The element's tag.
    #[required]
    pub tag: HtmlTag,

    /// The element's HTML attributes.
    #[borrowed]
    pub attrs: HtmlAttrs,

    /// The contents of the HTML element.
    ///
    /// The body can be arbitrary Typst content.
    #[positional]
    #[borrowed]
    pub body: Option<Content>,
}

impl HtmlElem {
    /// Add an attribute to the element.
    pub fn with_attr(mut self, attr: HtmlAttr, value: impl Into<EcoString>) -> Self {
        self.attrs.get_or_insert_with(Default::default).push(attr, value);
        self
    }
}

/// An element that lays out its content as an inline SVG.
///
/// Sometimes, converting Typst content to HTML is not desirable. This can be
/// the case for plots and other content that relies on positioning and styling
/// to convey its message.
///
/// This function allows you to use the Typst layout engine that would also be
/// used for PDF, SVG, and PNG export to render a part of your document exactly
/// how it would appear when exported in one of these formats. It embeds the
/// content as an inline SVG.
#[elem]
pub struct FrameElem {
    /// The content that shall be laid out.
    #[positional]
    #[required]
    pub body: Content,
}
