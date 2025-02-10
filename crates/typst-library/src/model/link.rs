use std::ops::Deref;

use ecow::{eco_format, EcoString};

use crate::diag::{bail, warning, At, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Content, Label, NativeElement, Packed, Repr, Show, ShowSet, Smart,
    StyleChain, Styles, TargetElem,
};
use crate::html::{attr, tag, HtmlElem};
use crate::introspection::Location;
use crate::layout::Position;
use crate::text::TextElem;

/// Links to a URL or a location in the document.
///
/// By default, links do not look any different from normal text. However,
/// you can easily apply a style of your choice with a show rule.
///
/// # Example
/// ```example
/// #show link: underline
///
/// https://example.com \
///
/// #link("https://example.com") \
/// #link("https://example.com")[
///   See example.com
/// ]
/// ```
///
/// # Hyphenation
/// If you enable hyphenation or justification, by default, it will not apply to
/// links to prevent unwanted hyphenation in URLs. You can opt out of this
/// default via `{show link: set text(hyphenate: true)}`.
///
/// # Syntax
/// This function also has dedicated syntax: Text that starts with `http://` or
/// `https://` is automatically turned into a link.
#[elem(Show)]
pub struct LinkElem {
    /// The destination the link points to.
    ///
    /// - To link to web pages, `dest` should be a valid URL string. If the URL
    ///   is in the `mailto:` or `tel:` scheme and the `body` parameter is
    ///   omitted, the email address or phone number will be the link's body,
    ///   without the scheme.
    ///
    /// - To link to another part of the document, `dest` can take one of three
    ///   forms:
    ///   - A [label] attached to an element. If you also want automatic text
    ///     for the link based on the element, consider using a
    ///     [reference]($ref) instead.
    ///
    ///   - A [`location`] (typically retrieved from [`here`], [`locate`] or
    ///     [`query`]).
    ///
    ///   - A dictionary with a `page` key of type [integer]($int) and `x` and
    ///     `y` coordinates of type [length]. Pages are counted from one, and
    ///     the coordinates are relative to the page's top left corner.
    ///
    /// ```example
    /// = Introduction <intro>
    /// #link("mailto:hello@typst.app") \
    /// #link(<intro>)[Go to intro] \
    /// #link((page: 1, x: 0pt, y: 0pt))[
    ///   Go to top
    /// ]
    /// ```
    #[required]
    #[parse(
        let dest = args.expect::<LinkTarget>("destination")?;
        dest.clone()
    )]
    pub dest: LinkTarget,

    /// The content that should become a link.
    ///
    /// If `dest` is an URL string, the parameter can be omitted. In this case,
    /// the URL will be shown as the link.
    #[required]
    #[parse(match &dest {
        LinkTarget::Dest(Destination::Url(url)) => match args.eat()? {
            Some(body) => body,
            None => body_from_url(url),
        },
        _ => args.expect("body")?,
    })]
    pub body: Content,

    /// A destination style that should be applied to elements.
    #[internal]
    #[ghost]
    pub current: Option<Destination>,
}

impl LinkElem {
    /// Create a link element from a URL with its bare text.
    pub fn from_url(url: Url) -> Self {
        let body = body_from_url(&url);
        Self::new(LinkTarget::Dest(Destination::Url(url)), body)
    }
}

impl Show for Packed<LinkElem> {
    #[typst_macros::time(name = "link", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let body = self.body.clone();

        Ok(if TargetElem::target_in(styles).is_html() {
            if let LinkTarget::Dest(Destination::Url(url)) = &self.dest {
                HtmlElem::new(tag::a)
                    .with_attr(attr::href, url.clone().into_inner())
                    .with_body(Some(body))
                    .pack()
                    .spanned(self.span())
            } else {
                engine.sink.warn(warning!(
                    self.span(),
                    "non-URL links are not yet supported by HTML export"
                ));
                body
            }
        } else {
            match &self.dest {
                LinkTarget::Dest(dest) => body.linked(dest.clone()),
                LinkTarget::Label(label) => {
                    let elem = engine.introspector.query_label(*label).at(self.span())?;
                    let dest = Destination::Location(elem.location().unwrap());
                    body.clone().linked(dest)
                }
            }
        })
    }
}

impl ShowSet for Packed<LinkElem> {
    fn show_set(&self, _: StyleChain) -> Styles {
        let mut out = Styles::new();
        out.set(TextElem::set_hyphenate(Smart::Custom(false)));
        out
    }
}

fn body_from_url(url: &Url) -> Content {
    let text = ["mailto:", "tel:"]
        .into_iter()
        .find_map(|prefix| url.strip_prefix(prefix))
        .unwrap_or(url);
    let shorter = text.len() < url.len();
    TextElem::packed(if shorter { text.into() } else { (**url).clone() })
}

/// A target where a link can go.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum LinkTarget {
    Dest(Destination),
    Label(Label),
}

cast! {
    LinkTarget,
    self => match self {
        Self::Dest(v) => v.into_value(),
        Self::Label(v) => v.into_value(),
    },
    v: Destination => Self::Dest(v),
    v: Label => Self::Label(v),
}

impl From<Destination> for LinkTarget {
    fn from(dest: Destination) -> Self {
        Self::Dest(dest)
    }
}

/// A link destination.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Destination {
    /// A link to a URL.
    Url(Url),
    /// A link to a point on a page.
    Position(Position),
    /// An unresolved link to a location in the document.
    Location(Location),
}

impl Destination {}

impl Repr for Destination {
    fn repr(&self) -> EcoString {
        eco_format!("{self:?}")
    }
}

cast! {
    Destination,
    self => match self {
        Self::Url(v) => v.into_value(),
        Self::Position(v) => v.into_value(),
        Self::Location(v) => v.into_value(),
    },
    v: Url => Self::Url(v),
    v: Position => Self::Position(v),
    v: Location => Self::Location(v),
}

/// A uniform resource locator with a maximum length.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Url(EcoString);

impl Url {
    /// Create a URL from a string, checking the maximum length.
    pub fn new(url: impl Into<EcoString>) -> StrResult<Self> {
        let url = url.into();
        if url.len() > 8000 {
            bail!("URL is too long")
        }
        Ok(Self(url))
    }

    /// Extract the underlying [`EcoString`].
    pub fn into_inner(self) -> EcoString {
        self.0
    }
}

impl Deref for Url {
    type Target = EcoString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

cast! {
    Url,
    self => self.0.into_value(),
    v: EcoString => Self::new(v)?,
}
