//! Typst's HTML exporter.

mod attr;
mod charsets;
mod convert;
mod css;
mod document;
mod dom;
mod encode;
mod fragment;
mod link;
mod mathml;
mod rules;
mod tag;
mod typed;

pub use self::document::html_document;
pub use self::dom::*;
pub use self::encode::html;
pub use self::rules::{html_span_filled, register};

use ecow::EcoString;
use typst_library::Category;
use typst_library::foundations::{Content, Module, Scope};
use typst_library::introspection::Location;
use typst_macros::elem;

/// Creates the module with all HTML definitions.
pub fn module() -> Module {
    let mut html = Scope::deduplicating();
    html.start_category(Category::Html);
    html.define_elem::<HtmlElem>();
    html.define_elem::<FrameElem>();
    crate::typed::define(&mut html);
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
    pub attrs: HtmlAttrs,

    /// The contents of the HTML element.
    ///
    /// The body can be arbitrary Typst content.
    #[positional]
    pub body: Option<Content>,

    /// The element's logical parent, if any.
    #[internal]
    #[synthesized]
    pub parent: Location,

    /// A role that should be applied to the top-level styled HTML element, but
    /// not its descendants. If we ever get set rules that apply to a specific
    /// element instead of a subtree, they could supplant this. If we need the
    /// same mechanism for things like `class`, this could potentially also be
    /// extended to arbitrary attributes. It's minimal for now.
    ///
    /// This is ignored for `<p>` elements as it otherwise tends to
    /// unintentionally attach to paragraphs resulting from grouping of a single
    /// element instead of attaching to that element. This is a bit of a hack,
    /// but good enough for now as the `role` property is purely internal and
    /// we control what it is used for.
    #[internal]
    #[ghost]
    pub role: Option<EcoString>,
}

impl HtmlElem {
    /// Add an attribute to the element.
    pub fn with_attr(mut self, attr: HtmlAttr, value: impl Into<EcoString>) -> Self {
        self.attrs
            .as_option_mut()
            .get_or_insert_with(Default::default)
            .push(attr, value);
        self
    }

    /// Adds the attribute to the element if value is not `None`.
    pub fn with_optional_attr(
        self,
        attr: HtmlAttr,
        value: Option<impl Into<EcoString>>,
    ) -> Self {
        if let Some(value) = value { self.with_attr(attr, value) } else { self }
    }

    /// Add an attribute to the element.
    pub fn push_attr(&mut self, attr: HtmlAttr, value: impl Into<EcoString>) {
        self.attrs
            .as_option_mut()
            .get_or_insert_with(Default::default)
            .push(attr, value);
    }

    /// Adds the attribute to the element if value is not `None`.
    pub fn push_optional_attr(
        &mut self,
        attr: HtmlAttr,
        value: Option<impl Into<EcoString>>,
    ) {
        if let Some(value) = value {
            self.push_attr(attr, value)
        }
    }

    /// Adds CSS styles to an element.
    fn with_styles(self, properties: css::Properties) -> Self {
        if let Some(value) = properties.into_inline_styles() {
            self.with_attr(attr::style, value)
        } else {
            self
        }
    }

    /// Checks whether the given element is an inline-level HTML element.
    fn is_inline(elem: &Content) -> bool {
        elem.to_packed::<HtmlElem>()
            .is_some_and(|elem| tag::is_inline_by_default(elem.tag))
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
